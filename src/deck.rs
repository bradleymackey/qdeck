use crate::config::{ClickAction, QDeckConfig, ResolvedTheme};
use crate::macos::{is_macos_screen_locked_or_asleep, is_shutdown_requested};
use crate::telemetry::{build_snapshot, is_dynamic_telemetry_tile, query_macos_gpu_stats};
use crate::widgets::{TelemetrySnapshot, render_standby_tile, render_tile_image};
use crate::xplane::XPlaneClient;
use anyhow::{Context, Result, anyhow};
use image::DynamicImage;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant, SystemTime};
use streamdeck::{Colour, Kind, StreamDeck, pids};
use sysinfo::System;

const TAP_HIGHLIGHT_DURATION: Duration = Duration::from_millis(120);
const MAX_BLOCK_WAIT: Duration = Duration::from_millis(150);

/// Returns the number of columns on the given Stream Deck hardware model.
pub fn cols_for_kind(kind: Kind) -> u8 {
    match kind {
        Kind::Mini | Kind::RevisedMini | Kind::Module6Keys => 3,
        Kind::Original | Kind::OriginalV2 | Kind::Mk2 | Kind::Module15Keys => 5,
        Kind::Xl | Kind::Module32Keys => 8,
        Kind::Plus => 4,
    }
}

/// Blanks all keys to pure black (`#000000`) and turns Stream Deck brightness down to `0%`.
pub fn blank_and_sleep_deck(deck: &mut StreamDeck, total_keys: u8) {
    let black = Colour { r: 0, g: 0, b: 0 };
    for k in 0..total_keys {
        let _ = deck.set_button_rgb(k, &black);
    }
    let _ = deck.set_brightness(0);
}

pub fn connect_deck(cfg: &QDeckConfig) -> Result<StreamDeck> {
    if let Some(pid) = cfg.pid {
        return StreamDeck::connect(cfg.vid, pid, cfg.serial.clone()).with_context(|| {
            format!(
                "Failed to connect to StreamDeck (VID {:04x}, PID {:04x})",
                cfg.vid, pid
            )
        });
    }

    if let Ok(found) = StreamDeck::probe() {
        for item in found.into_iter().flatten() {
            let (kind, pid) = item;
            if let Ok(deck) = StreamDeck::connect(cfg.vid, pid, cfg.serial.clone()) {
                println!(
                    "[QDECK] Auto-detected Stream Deck {:?} (VID: {:04x}, PID: {:04x})",
                    kind, cfg.vid, pid
                );
                return Ok(deck);
            }
        }
    }

    StreamDeck::connect(cfg.vid, pids::MK2, cfg.serial.clone()).map_err(|e| {
        anyhow!(
            "Could not connect to Stream Deck: {e}. Make sure the Stream Deck is plugged in and the official Elgato app is closed."
        )
    })
}

pub fn file_mtime(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).ok().and_then(|m| m.modified().ok())
}

#[allow(clippy::too_many_arguments)]
pub fn render_key_to_deck(
    deck: &mut StreamDeck,
    cfg: &QDeckConfig,
    theme: &ResolvedTheme,
    cols: u8,
    total_keys: u8,
    img_w: usize,
    img_h: usize,
    key_idx: u8,
    snapshot: &TelemetrySnapshot,
    is_highlighted: bool,
) -> Result<()> {
    if let Some(tile) = cfg.tile_for_key(
        key_idx,
        cols,
        total_keys,
        snapshot.active_mode,
        snapshot.active_panel,
    ) {
        let img = render_tile_image(
            img_w as u32,
            img_h as u32,
            key_idx,
            tile,
            theme,
            snapshot,
            is_highlighted,
        );
        deck.set_button_image(key_idx, DynamicImage::ImageRgb8(img))?;
    } else if cfg.background_mode == "blank" && !is_highlighted {
        let bg = Colour {
            r: theme.background.0[0],
            g: theme.background.0[1],
            b: theme.background.0[2],
        };
        deck.set_button_rgb(key_idx, &bg)?;
    } else {
        let standby_img =
            render_standby_tile(img_w as u32, img_h as u32, key_idx, theme, is_highlighted);
        deck.set_button_image(key_idx, DynamicImage::ImageRgb8(standby_img))?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn apply_mode_layout(
    deck: &mut StreamDeck,
    cfg: &QDeckConfig,
    theme: &ResolvedTheme,
    cols: u8,
    total_keys: u8,
    img_w: usize,
    img_h: usize,
    snapshot: &TelemetrySnapshot,
) -> Result<()> {
    deck.set_brightness(cfg.brightness)?;
    let mut configured_keys = HashSet::new();

    for k in 0..total_keys {
        if let Some(tile) = cfg.tile_for_key(
            k,
            cols,
            total_keys,
            snapshot.active_mode,
            snapshot.active_panel,
        ) {
            configured_keys.insert(k);
            println!(
                "[QDECK] [mode={}/panel={}] Key {} (row {}, col {}) -> '{}' (type='{}') | click={:?}",
                snapshot.active_mode,
                snapshot.active_panel,
                k,
                k / cols,
                k % cols,
                tile.label
                    .as_deref()
                    .or(tile.control.as_deref())
                    .unwrap_or(&tile.tile_type),
                tile.tile_type,
                tile.click_action()
            );
            render_key_to_deck(
                deck, cfg, theme, cols, total_keys, img_w, img_h, k, snapshot, false,
            )?;
        }
    }

    for k in 0..total_keys {
        if !configured_keys.contains(&k) {
            if cfg.background_mode == "blank" {
                let bg = Colour {
                    r: theme.background.0[0],
                    g: theme.background.0[1],
                    b: theme.background.0[2],
                };
                deck.set_button_rgb(k, &bg)?;
            } else {
                let standby_img = render_standby_tile(img_w as u32, img_h as u32, k, theme, false);
                deck.set_button_image(k, DynamicImage::ImageRgb8(standby_img))?;
            }
        }
    }
    Ok(())
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
        let total_modes = modes.len().max(1);
        let active_mode = &modes[*mode_idx];
        let next_mode = &modes[(*mode_idx + 1) % total_modes];
        let panels = cfg.effective_panels(active_mode);
        let total_panels = panels.len().max(1);
        *panel_idx %= total_panels;
        let active_panel = &panels[*panel_idx];
        let next_panel = &panels[(*panel_idx + 1) % total_panels];

        let initial_snapshot = build_snapshot(
            sys,
            &mut core_buf,
            *gpu_cores,
            *live_gpu_pct,
            active_mode,
            next_mode,
            *mode_idx,
            total_modes,
            active_panel,
            next_panel,
            *panel_idx,
            total_panels,
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
                let total_modes = modes.len().max(1);
                let active_mode = &modes[*mode_idx];
                let next_mode = &modes[(*mode_idx + 1) % total_modes];
                let panels = cfg.effective_panels(active_mode);
                let total_panels = panels.len().max(1);
                *panel_idx %= total_panels;
                let active_panel = &panels[*panel_idx];
                let next_panel = &panels[(*panel_idx + 1) % total_panels];

                let snapshot = build_snapshot(
                    sys,
                    &mut core_buf,
                    *gpu_cores,
                    *live_gpu_pct,
                    active_mode,
                    next_mode,
                    *mode_idx,
                    total_modes,
                    active_panel,
                    next_panel,
                    *panel_idx,
                    total_panels,
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

        let mut newly_highlighted_keys: Vec<u8> = Vec::new();
        let mut mode_or_panel_switched = false;
        let mut fcu_changed = false;

        match deck.read_buttons(Some(wait_duration)) {
            Ok(buttons) => {
                let press_time = Instant::now();
                for (k_idx, (&prev, &curr)) in prev_buttons.iter().zip(buttons.iter()).enumerate() {
                    let key_u8 = k_idx as u8;
                    if prev == 0 && curr != 0 {
                        highlight_until[k_idx] = Some(press_time + TAP_HIGHLIGHT_DURATION);
                        newly_highlighted_keys.push(key_u8);
                        let active_mode = &modes[*mode_idx];
                        let panels = cfg.effective_panels(active_mode);
                        let total_panels = panels.len().max(1);
                        *panel_idx %= total_panels;
                        let active_panel = &panels[*panel_idx];

                        if let Some(tile) =
                            cfg.tile_for_key(key_u8, cols, total_keys, active_mode, active_panel)
                        {
                            match tile.click_action() {
                                ClickAction::CycleMode => {
                                    *mode_idx = (*mode_idx + 1) % modes.len().max(1);
                                    println!("[QDECK] Mode toggled -> '{}'", modes[*mode_idx]);
                                    mode_or_panel_switched = true;
                                }
                                ClickAction::CyclePanel => {
                                    *panel_idx = (*panel_idx + 1) % total_panels;
                                    println!(
                                        "[QDECK] Panel toggled -> '{}/{}'",
                                        active_mode, panels[*panel_idx]
                                    );
                                    mode_or_panel_switched = true;
                                }
                                ClickAction::XPlaneControl {
                                    control,
                                    custom_cmd,
                                } => {
                                    println!(
                                        "[QDECK] [X-Plane] Triggered control '{}' (panel='{}')",
                                        control, active_panel
                                    );
                                    xplane.trigger_a320_control(&control, custom_cmd.as_deref());
                                    fcu_changed = true;
                                }
                                ClickAction::OpenApp(app_name) => {
                                    println!(
                                        "[QDECK] Click -> Opening macOS application: '{}'",
                                        app_name
                                    );
                                    if let Err(e) =
                                        Command::new("open").arg("-a").arg(&app_name).spawn()
                                    {
                                        eprintln!(
                                            "[QDECK] Failed to open app '{}': {}",
                                            app_name, e
                                        );
                                    }
                                }
                                ClickAction::ShellCommand(cmd) => {
                                    println!("[QDECK] Click -> Executing command: '{}'", cmd);
                                    if let Err(e) =
                                        Command::new("/bin/sh").arg("-c").arg(&cmd).spawn()
                                    {
                                        eprintln!(
                                            "[QDECK] Failed to run command '{}': {}",
                                            cmd, e
                                        );
                                    }
                                }
                                ClickAction::TriggerDemoSpike => {
                                    println!(
                                        "[QDECK] Click -> Triggering 4s simulated Amber/Red load spike demo!"
                                    );
                                    demo_until = Some(Instant::now() + Duration::from_secs(4));
                                }
                                ClickAction::None => {}
                            }
                        }
                    }
                }
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

        let now = Instant::now();
        let mut expired_highlight_keys: Vec<u8> = Vec::new();
        for (k_idx, slot) in highlight_until.iter_mut().enumerate() {
            if let Some(deadline) = *slot {
                if now >= deadline {
                    *slot = None;
                    expired_highlight_keys.push(k_idx as u8);
                }
            }
        }

        let in_demo = demo_until.is_some_and(|deadline| Instant::now() < deadline);
        if !in_demo {
            demo_until = None;
        }

        if mode_or_panel_switched {
            let total_modes = modes.len().max(1);
            let active_mode = &modes[*mode_idx];
            let next_mode = &modes[(*mode_idx + 1) % total_modes];
            let panels = cfg.effective_panels(active_mode);
            let total_panels = panels.len().max(1);
            *panel_idx %= total_panels;
            let active_panel = &panels[*panel_idx];
            let next_panel = &panels[(*panel_idx + 1) % total_panels];

            let snapshot = build_snapshot(
                sys,
                &mut core_buf,
                *gpu_cores,
                *live_gpu_pct,
                active_mode,
                next_mode,
                *mode_idx,
                total_modes,
                active_panel,
                next_panel,
                *panel_idx,
                total_panels,
                &xplane.state,
                *tick,
                in_demo,
            );
            apply_mode_layout(
                &mut deck, cfg, theme, cols, total_keys, img_w, img_h, &snapshot,
            )?;
            for &key_u8 in &newly_highlighted_keys {
                render_key_to_deck(
                    &mut deck, cfg, theme, cols, total_keys, img_w, img_h, key_u8, &snapshot, true,
                )?;
            }
            last_render = Instant::now();
            continue;
        }

        if fcu_changed || !newly_highlighted_keys.is_empty() || !expired_highlight_keys.is_empty() {
            let total_modes = modes.len().max(1);
            let active_mode = &modes[*mode_idx];
            let next_mode = &modes[(*mode_idx + 1) % total_modes];
            let panels = cfg.effective_panels(active_mode);
            let total_panels = panels.len().max(1);
            *panel_idx %= total_panels;
            let active_panel = &panels[*panel_idx];
            let next_panel = &panels[(*panel_idx + 1) % total_panels];

            let snapshot = build_snapshot(
                sys,
                &mut core_buf,
                *gpu_cores,
                *live_gpu_pct,
                active_mode,
                next_mode,
                *mode_idx,
                total_modes,
                active_panel,
                next_panel,
                *panel_idx,
                total_panels,
                &xplane.state,
                *tick,
                in_demo,
            );

            if fcu_changed {
                for k in 0..total_keys {
                    if let Some(t) =
                        cfg.tile_for_key(k, cols, total_keys, active_mode, active_panel)
                    {
                        if t.is_a320_fcu() {
                            let is_hi = highlight_until
                                .get(k as usize)
                                .and_then(|&d| d)
                                .is_some_and(|deadline| Instant::now() < deadline);
                            render_key_to_deck(
                                &mut deck, cfg, theme, cols, total_keys, img_w, img_h, k,
                                &snapshot, is_hi,
                            )?;
                        }
                    }
                }
            } else {
                for key_u8 in newly_highlighted_keys {
                    render_key_to_deck(
                        &mut deck, cfg, theme, cols, total_keys, img_w, img_h, key_u8, &snapshot,
                        true,
                    )?;
                }
            }

            for key_u8 in expired_highlight_keys {
                render_key_to_deck(
                    &mut deck, cfg, theme, cols, total_keys, img_w, img_h, key_u8, &snapshot,
                    false,
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
                        *mode_idx = modes
                            .iter()
                            .position(|m| m == &prev_mode_name)
                            .unwrap_or(0);
                        let total_modes = modes.len().max(1);
                        let active_mode = &modes[*mode_idx];
                        let next_mode = &modes[(*mode_idx + 1) % total_modes];
                        let panels = cfg.effective_panels(active_mode);
                        let total_panels = panels.len().max(1);
                        *panel_idx %= total_panels;
                        let active_panel = &panels[*panel_idx];
                        let next_panel = &panels[(*panel_idx + 1) % total_panels];

                        let snapshot = build_snapshot(
                            sys,
                            &mut core_buf,
                            *gpu_cores,
                            *live_gpu_pct,
                            active_mode,
                            next_mode,
                            *mode_idx,
                            total_modes,
                            active_panel,
                            next_panel,
                            *panel_idx,
                            total_panels,
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

            let total_modes = modes.len().max(1);
            let active_mode = &modes[*mode_idx];
            let next_mode = &modes[(*mode_idx + 1) % total_modes];
            let panels = cfg.effective_panels(active_mode);
            let total_panels = panels.len().max(1);
            *panel_idx %= total_panels;
            let active_panel = &panels[*panel_idx];
            let next_panel = &panels[(*panel_idx + 1) % total_panels];

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

            let snapshot = build_snapshot(
                sys,
                &mut core_buf,
                *gpu_cores,
                *live_gpu_pct,
                active_mode,
                next_mode,
                *mode_idx,
                total_modes,
                active_panel,
                next_panel,
                *panel_idx,
                total_panels,
                &xplane.state,
                *tick,
                in_demo,
            );

            for k in 0..total_keys {
                if let Some(tile) = cfg.tile_for_key(k, cols, total_keys, active_mode, active_panel)
                {
                    if is_dynamic_telemetry_tile(&tile.tile_type) {
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
            }

            last_render = Instant::now();
            *tick += 1;
            if let Some(max) = max_frames {
                if *tick >= max {
                    blank_and_sleep_deck(&mut deck, total_keys);
                    return Ok(true);
                }
            }
        }
    }
}
