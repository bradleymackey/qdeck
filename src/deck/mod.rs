mod device;
mod input;
mod render;

pub use device::{blank_and_sleep_deck, cols_for_kind, connect_deck, file_mtime};
pub use render::{apply_mode_layout, render_key_to_deck};

use crate::config::{QDeckConfig, ResolvedTheme};
use crate::macos::{is_macos_screen_locked_or_asleep, is_shutdown_requested};
use crate::telemetry::{
    build_snapshot, current_ai_limits, is_dynamic_telemetry_tile, query_macos_gpu_stats,
};
use crate::widgets::TelemetrySnapshot;
use crate::xplane::{A320FcuState, XPlaneClient};
use anyhow::{Result, anyhow};
use input::{collect_expired_highlights, process_button_edges};
use std::path::Path;
use std::time::{Duration, Instant, SystemTime};
use streamdeck::StreamDeck;
use sysinfo::System;

const MAX_BLOCK_WAIT: Duration = Duration::from_millis(150);

struct ActiveLayout {
    total_modes: usize,
    active_mode: String,
    next_mode: String,
    total_panels: usize,
    active_panel: String,
    next_panel: String,
}

impl ActiveLayout {
    fn resolve(
        cfg: &QDeckConfig,
        modes: &[String],
        mode_idx: usize,
        panel_idx: &mut usize,
    ) -> Self {
        let total_modes = modes.len().max(1);
        let active_mode = modes[mode_idx].clone();
        let next_mode = modes[(mode_idx + 1) % total_modes].clone();
        let panels = cfg.effective_panels(&active_mode);
        let total_panels = panels.len().max(1);
        *panel_idx %= total_panels;
        let active_panel = panels[*panel_idx].clone();
        let next_panel = panels[(*panel_idx + 1) % total_panels].clone();
        Self {
            total_modes,
            active_mode,
            next_mode,
            total_panels,
            active_panel,
            next_panel,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn snapshot<'a>(
        &'a self,
        sys: &System,
        core_buf: &'a mut Vec<f32>,
        gpu_cores: usize,
        live_gpu_pct: f32,
        mode_idx: usize,
        panel_idx: usize,
        fcu: &'a A320FcuState,
        tick: u64,
        in_demo: bool,
    ) -> TelemetrySnapshot<'a> {
        let ai_limits = current_ai_limits();
        build_snapshot(
            sys,
            core_buf,
            gpu_cores,
            live_gpu_pct,
            ai_limits.claude,
            ai_limits.gemini,
            &self.active_mode,
            &self.next_mode,
            mode_idx,
            self.total_modes,
            &self.active_panel,
            &self.next_panel,
            panel_idx,
            self.total_panels,
            fcu,
            tick,
            in_demo,
        )
    }
}

#[allow(clippy::too_many_arguments)]
pub fn run_deck_session(
    mut deck: StreamDeck,
    cfg: &mut QDeckConfig,
    cfg_path: &Path,
    theme: &mut ResolvedTheme,
    modes: &mut Vec<String>,
    mode_idx: &mut usize,
    panel_idx: &mut usize,
    last_mtime: &mut Option<SystemTime>,
    xplane: &mut XPlaneClient,
    sys: &mut System,
    gpu_cores: &mut usize,
    live_gpu_pct: &mut f32,
    tick: &mut u64,
    max_frames: Option<u64>,
) -> Result<bool> {
    let kind = deck.kind();
    let total_keys = kind.keys();
    let cols = cols_for_kind(kind);
    let (img_w, img_h) = deck.image_size();

    let product = deck.product().unwrap_or_else(|_| format!("{:?}", kind));
    let serial = deck.serial().unwrap_or_else(|_| "UNKNOWN".to_string());
    let fw_version = deck.version().unwrap_or_else(|_| "UNKNOWN".to_string());

    println!(
        "[QDECK] Connected: {} (SN: {}, FW: {})",
        product.trim_matches(char::from(0)),
        serial.trim_matches(char::from(0)),
        fw_version.trim_matches(char::from(0))
    );
    println!(
        "[QDECK] Grid: {} keys ({} cols x {} rows) @ {}x{} px",
        total_keys,
        cols,
        total_keys / cols,
        img_w,
        img_h
    );

    let _ = deck.set_blocking(true);

    let mut demo_until: Option<Instant> = None;
    let mut prev_buttons = vec![0u8; total_keys as usize];
    let mut highlight_until: Vec<Option<Instant>> = vec![None; total_keys as usize];
    let mut core_buf: Vec<f32> = Vec::with_capacity(32);
    let mut screen_locked = is_macos_screen_locked_or_asleep();

    if screen_locked {
        println!("[QDECK] Mac display is locked/asleep — blanking Stream Deck");
        blank_and_sleep_deck(&mut deck, total_keys);
    } else {
        let layout = ActiveLayout::resolve(cfg, modes, *mode_idx, panel_idx);
        let initial_snapshot = layout.snapshot(
            sys,
            &mut core_buf,
            *gpu_cores,
            *live_gpu_pct,
            *mode_idx,
            *panel_idx,
            &xplane.state,
            *tick,
            false,
        );
        apply_mode_layout(
            &mut deck,
            cfg,
            theme,
            cols,
            total_keys,
            img_w,
            img_h,
            &initial_snapshot,
        )?;
    }

    let mut last_render = Instant::now();

    loop {
        if is_shutdown_requested() {
            println!("[QDECK] Shutdown signal received — blanking Stream Deck and exiting.");
            blank_and_sleep_deck(&mut deck, total_keys);
            return Ok(true);
        }

        let now_locked = is_macos_screen_locked_or_asleep();
        if now_locked != screen_locked {
            screen_locked = now_locked;
            if screen_locked {
                println!("[QDECK] Mac display locked/asleep — blanking Stream Deck screen");
                highlight_until.fill(None);
                blank_and_sleep_deck(&mut deck, total_keys);
            } else {
                println!("[QDECK] Mac display unlocked — restoring Stream Deck display");
                let layout = ActiveLayout::resolve(cfg, modes, *mode_idx, panel_idx);
                let snapshot = layout.snapshot(
                    sys,
                    &mut core_buf,
                    *gpu_cores,
                    *live_gpu_pct,
                    *mode_idx,
                    *panel_idx,
                    &xplane.state,
                    *tick,
                    false,
                );
                apply_mode_layout(
                    &mut deck, cfg, theme, cols, total_keys, img_w, img_h, &snapshot,
                )?;
                last_render = Instant::now();
            }
        }

        if screen_locked {
            match deck.read_buttons(Some(MAX_BLOCK_WAIT)) {
                Ok(buttons) => {
                    prev_buttons = buttons;
                }
                Err(streamdeck::Error::NoData) => {}
                Err(e) => {
                    return Err(anyhow!("Stream Deck USB read error: {e}"));
                }
            }
            continue;
        }

        let refresh_interval = Duration::from_millis(cfg.refresh_ms.max(250));
        let now = Instant::now();

        let next_telemetry_in = refresh_interval.saturating_sub(now.duration_since(last_render));
        let next_highlight_expiry = highlight_until
            .iter()
            .filter_map(|&opt| opt)
            .map(|deadline| deadline.saturating_duration_since(now))
            .min();

        let wait_duration = match next_highlight_expiry {
            Some(h_wait) => next_telemetry_in
                .min(h_wait)
                .min(MAX_BLOCK_WAIT)
                .max(Duration::from_millis(1)),
            None => next_telemetry_in
                .min(MAX_BLOCK_WAIT)
                .max(Duration::from_millis(1)),
        };

        let mut outcome = input::ButtonEventOutcome {
            newly_highlighted_keys: Vec::new(),
            mode_or_panel_switched: false,
            fcu_changed: false,
        };

        match deck.read_buttons(Some(wait_duration)) {
            Ok(buttons) => {
                outcome = process_button_edges(
                    &prev_buttons,
                    &buttons,
                    &mut highlight_until,
                    cfg,
                    cols,
                    total_keys,
                    modes,
                    mode_idx,
                    panel_idx,
                    xplane,
                    &mut demo_until,
                );
                prev_buttons = buttons;
            }
            Err(streamdeck::Error::NoData) => {}
            Err(e) => {
                return Err(anyhow!("Stream Deck USB read error: {e}"));
            }
        }

        if is_shutdown_requested() {
            println!("[QDECK] Shutdown signal received — blanking Stream Deck and exiting.");
            blank_and_sleep_deck(&mut deck, total_keys);
            return Ok(true);
        }

        let expired_highlight_keys =
            collect_expired_highlights(&mut highlight_until, Instant::now());

        let in_demo = demo_until.is_some_and(|deadline| Instant::now() < deadline);
        if !in_demo {
            demo_until = None;
        }

        if outcome.mode_or_panel_switched {
            let layout = ActiveLayout::resolve(cfg, modes, *mode_idx, panel_idx);
            let snapshot = layout.snapshot(
                sys,
                &mut core_buf,
                *gpu_cores,
                *live_gpu_pct,
                *mode_idx,
                *panel_idx,
                &xplane.state,
                *tick,
                in_demo,
            );
            apply_mode_layout(
                &mut deck, cfg, theme, cols, total_keys, img_w, img_h, &snapshot,
            )?;
            for &key_u8 in &outcome.newly_highlighted_keys {
                render_key_to_deck(
                    &mut deck, cfg, theme, cols, total_keys, img_w, img_h, key_u8, &snapshot, true,
                )?;
            }
            last_render = Instant::now();
            continue;
        }

        if outcome.fcu_changed
            || !outcome.newly_highlighted_keys.is_empty()
            || !expired_highlight_keys.is_empty()
        {
            let layout = ActiveLayout::resolve(cfg, modes, *mode_idx, panel_idx);
            let snapshot = layout.snapshot(
                sys,
                &mut core_buf,
                *gpu_cores,
                *live_gpu_pct,
                *mode_idx,
                *panel_idx,
                &xplane.state,
                *tick,
                in_demo,
            );

            if outcome.fcu_changed {
                for k in 0..total_keys {
                    if let Some(t) = cfg.tile_for_key(
                        k,
                        cols,
                        total_keys,
                        &layout.active_mode,
                        &layout.active_panel,
                    ) && t.is_a320_fcu()
                    {
                        let is_hi = highlight_until
                            .get(k as usize)
                            .and_then(|&d| d)
                            .is_some_and(|deadline| Instant::now() < deadline);
                        render_key_to_deck(
                            &mut deck, cfg, theme, cols, total_keys, img_w, img_h, k, &snapshot,
                            is_hi,
                        )?;
                    }
                }
            } else {
                for key_u8 in outcome.newly_highlighted_keys {
                    render_key_to_deck(
                        &mut deck, cfg, theme, cols, total_keys, img_w, img_h, key_u8, &snapshot,
                        true,
                    )?;
                }
            }

            for key_u8 in expired_highlight_keys {
                render_key_to_deck(
                    &mut deck, cfg, theme, cols, total_keys, img_w, img_h, key_u8, &snapshot, false,
                )?;
            }
        }

        if last_render.elapsed() >= refresh_interval {
            let current_mtime = file_mtime(cfg_path);
            if current_mtime != *last_mtime && current_mtime.is_some() {
                *last_mtime = current_mtime;
                match QDeckConfig::load_from_file(cfg_path) {
                    Ok(new_cfg) => {
                        println!(
                            "[QDECK] Hot-reloaded configuration from {}",
                            cfg_path.display()
                        );
                        let prev_mode_name = modes.get(*mode_idx).cloned().unwrap_or_default();
                        *cfg = new_cfg;
                        xplane.set_target_addr(&cfg.xplane_addr);
                        *theme = cfg.theme.resolve();
                        *modes = cfg.effective_modes();
                        *mode_idx = modes.iter().position(|m| m == &prev_mode_name).unwrap_or(0);
                        let layout = ActiveLayout::resolve(cfg, modes, *mode_idx, panel_idx);
                        let snapshot = layout.snapshot(
                            sys,
                            &mut core_buf,
                            *gpu_cores,
                            *live_gpu_pct,
                            *mode_idx,
                            *panel_idx,
                            &xplane.state,
                            *tick,
                            in_demo,
                        );
                        apply_mode_layout(
                            &mut deck, cfg, theme, cols, total_keys, img_w, img_h, &snapshot,
                        )?;
                    }
                    Err(e) => {
                        eprintln!("[QDECK] Config reload error: {}", e);
                    }
                }
            }

            let layout = ActiveLayout::resolve(cfg, modes, *mode_idx, panel_idx);

            refresh_active_telemetry(
                cfg,
                cols,
                total_keys,
                &layout.active_mode,
                &layout.active_panel,
                in_demo,
                sys,
                gpu_cores,
                live_gpu_pct,
                xplane,
            );

            let snapshot = layout.snapshot(
                sys,
                &mut core_buf,
                *gpu_cores,
                *live_gpu_pct,
                *mode_idx,
                *panel_idx,
                &xplane.state,
                *tick,
                in_demo,
            );

            for k in 0..total_keys {
                if let Some(tile) = cfg.tile_for_key(
                    k,
                    cols,
                    total_keys,
                    &layout.active_mode,
                    &layout.active_panel,
                ) && is_dynamic_telemetry_tile(&tile.tile_type)
                {
                    let is_hi = highlight_until
                        .get(k as usize)
                        .and_then(|&d| d)
                        .is_some_and(|deadline| Instant::now() < deadline);
                    render_key_to_deck(
                        &mut deck, cfg, theme, cols, total_keys, img_w, img_h, k, &snapshot, is_hi,
                    )?;
                }
            }

            last_render = Instant::now();
            *tick += 1;
            if let Some(max) = max_frames
                && *tick >= max
            {
                blank_and_sleep_deck(&mut deck, total_keys);
                return Ok(true);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn refresh_active_telemetry(
    cfg: &QDeckConfig,
    cols: u8,
    total_keys: u8,
    active_mode: &str,
    active_panel: &str,
    in_demo: bool,
    sys: &mut System,
    gpu_cores: &mut usize,
    live_gpu_pct: &mut f32,
    xplane: &mut XPlaneClient,
) {
    let mut needs_cpu = false;
    let mut needs_gpu = false;
    let mut needs_mem = false;
    let mut needs_xplane = false;

    for k in 0..total_keys {
        if let Some(t) = cfg.tile_for_key(k, cols, total_keys, active_mode, active_panel) {
            match t.tile_type.trim().to_ascii_lowercase().as_str() {
                "cpu" => needs_cpu = true,
                "gpu" => needs_gpu = true,
                "memory" | "ram" => needs_mem = true,
                "a320_fcu" | "xplane" | "fcu" | "a320" => needs_xplane = true,
                "ops_status" | "status" => {
                    needs_cpu = true;
                    needs_gpu = true;
                }
                _ => {}
            }
        }
    }

    if needs_cpu {
        sys.refresh_cpu_usage();
    }
    if needs_mem {
        sys.refresh_memory();
    }
    if needs_gpu && !in_demo {
        let (detected_cores, gpu_util) = query_macos_gpu_stats(*gpu_cores);
        *gpu_cores = detected_cores;
        *live_gpu_pct = gpu_util;
    }
    if needs_xplane {
        xplane.ensure_subscribed();
        xplane.poll_incoming(active_panel);
    }
}
