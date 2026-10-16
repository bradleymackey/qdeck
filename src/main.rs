mod config;
mod font;
mod widgets;

use anyhow::{Context, Result, anyhow};
use config::{ClickAction, QDeckConfig, ResolvedTheme};
use image::DynamicImage;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant, SystemTime};
use streamdeck::{Colour, Kind, StreamDeck, pids};
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};
use widgets::{
    TelemetrySnapshot, render_cpu_tile, render_gpu_tile, render_launcher_tile, render_memory_tile,
    render_mode_tile, render_ops_status_tile, render_standby_tile,
};

const TAP_HIGHLIGHT_DURATION: Duration = Duration::from_millis(120);

fn cols_for_kind(kind: Kind) -> u8 {
    match kind {
        Kind::Mini | Kind::RevisedMini | Kind::Module6Keys => 3,
        Kind::Original | Kind::OriginalV2 | Kind::Mk2 | Kind::Module15Keys => 5,
        Kind::Xl | Kind::Module32Keys => 8,
        Kind::Plus => 4,
    }
}

/// Queries macOS IOKit `IOAccelerator` for GPU core count and live utilization percentage.
fn query_macos_gpu_stats(fallback_cores: usize) -> (usize, f32) {
    let Ok(output) = Command::new("ioreg")
        .args(["-r", "-d", "1", "-w", "0", "-c", "IOAccelerator"])
        .output()
    else {
        return (fallback_cores, 0.0);
    };

    let text = String::from_utf8_lossy(&output.stdout);
    let mut gpu_cores = fallback_cores;
    let mut max_util: f32 = 0.0;

    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("\"gpu-core-count\" =") {
            if let Ok(c) = rest.trim().parse::<usize>() {
                if c > 0 {
                    gpu_cores = c;
                }
            }
        }
        if trimmed.starts_with("\"PerformanceStatistics\"") {
            for key in [
                "\"Device Utilization %\"=",
                "\"Renderer Utilization %\"=",
                "\"Tiler Utilization %\"=",
            ] {
                if let Some(pos) = trimmed.find(key) {
                    let after = &trimmed[pos + key.len()..];
                    let digits: String = after
                        .chars()
                        .take_while(|c| c.is_ascii_digit() || *c == '.')
                        .collect();
                    if let Ok(val) = digits.parse::<f32>() {
                        max_util = max_util.max(val);
                    }
                }
            }
        }
    }

    (gpu_cores, max_util.clamp(0.0, 100.0))
}

fn connect_deck(cfg: &QDeckConfig) -> Result<StreamDeck> {
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
            "Could not connect to Stream Deck: {e}. Make sure the official Elgato Stream Deck app is closed or not holding exclusive HID access."
        )
    })
}

fn file_mtime(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).ok().and_then(|m| m.modified().ok())
}

fn is_dynamic_telemetry_tile(tile_type: &str) -> bool {
    matches!(
        tile_type.trim().to_ascii_lowercase().as_str(),
        "cpu" | "gpu" | "memory" | "ram" | "ops_status" | "status"
    )
}

#[allow(clippy::too_many_arguments)]
fn render_key_to_deck(
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
    if let Some(tile) = cfg.tile_for_key(key_idx, cols, total_keys, snapshot.active_mode) {
        let img = match tile.tile_type.trim().to_ascii_lowercase().as_str() {
            "mode" | "mode_selector" => render_mode_tile(
                img_w as u32,
                img_h as u32,
                tile,
                theme,
                snapshot,
                is_highlighted,
            ),
            "cpu" => render_cpu_tile(
                img_w as u32,
                img_h as u32,
                tile,
                theme,
                snapshot,
                is_highlighted,
            ),
            "gpu" => render_gpu_tile(
                img_w as u32,
                img_h as u32,
                tile,
                theme,
                snapshot,
                is_highlighted,
            ),
            "memory" | "ram" => render_memory_tile(
                img_w as u32,
                img_h as u32,
                tile,
                theme,
                snapshot,
                is_highlighted,
            ),
            "ops_status" | "status" => render_ops_status_tile(
                img_w as u32,
                img_h as u32,
                tile,
                theme,
                snapshot,
                is_highlighted,
            ),
            "launcher" | "command" | "cmd" => {
                render_launcher_tile(img_w as u32, img_h as u32, tile, theme, is_highlighted)
            }
            _ => render_standby_tile(img_w as u32, img_h as u32, key_idx, theme, is_highlighted),
        };
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
fn apply_mode_layout(
    deck: &mut StreamDeck,
    cfg: &QDeckConfig,
    theme: &ResolvedTheme,
    cols: u8,
    total_keys: u8,
    img_w: usize,
    img_h: usize,
    snapshot: &TelemetrySnapshot,
) -> Result<()> {
    let _ = deck.set_brightness(cfg.brightness);
    let mut configured_keys = HashSet::new();

    for k in 0..total_keys {
        if let Some(tile) = cfg.tile_for_key(k, cols, total_keys, snapshot.active_mode) {
            configured_keys.insert(k);
            println!(
                "[QDECK] [mode={}] Key {} (row {}, col {}) -> '{}' (type='{}') | click={:?}",
                snapshot.active_mode,
                k,
                k / cols,
                k % cols,
                tile.label.as_deref().unwrap_or(&tile.tile_type),
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
fn build_snapshot<'a>(
    sys: &System,
    core_buf: &'a mut Vec<f32>,
    gpu_cores: usize,
    live_gpu_pct: f32,
    active_mode: &'a str,
    next_mode: &'a str,
    mode_index: usize,
    total_modes: usize,
    tick: u64,
    in_demo: bool,
) -> TelemetrySnapshot<'a> {
    let num_cores = sys.cpus().len().max(1);
    core_buf.clear();
    let (total_cpu_pct, gpu_pct) = if in_demo {
        let phase = (tick as f32) * 0.65;
        for c in 0..num_cores {
            let c_ratio = c as f32 / (num_cores.max(1) as f32);
            let wave = ((phase + c_ratio * 4.2).sin() * 0.5 + 0.5).powf(0.85);
            let pct = (18.0 + wave * 80.0).clamp(5.0, 98.0);
            core_buf.push(pct);
        }
        let avg_cpu = core_buf.iter().sum::<f32>() / (core_buf.len() as f32);
        let sim_gpu = (15.0 + ((phase * 0.85).sin() * 0.5 + 0.5) * 82.0).clamp(10.0, 98.0);
        (avg_cpu, sim_gpu)
    } else {
        for cpu in sys.cpus() {
            core_buf.push(cpu.cpu_usage());
        }
        (sys.global_cpu_usage(), live_gpu_pct)
    };

    let total_mem = sys.total_memory().max(1) as f32;
    let used_mem = sys.used_memory() as f32;
    let mem_used_pct = (used_mem / total_mem) * 100.0;
    let mem_used_gb = used_mem / 1_073_741_824.0;
    let mem_total_gb = total_mem / 1_073_741_824.0;
    let total_swap = sys.total_swap() as f32;
    let swap_used_pct = if total_swap > 0.0 {
        (sys.used_swap() as f32 / total_swap) * 100.0
    } else {
        0.0
    };

    TelemetrySnapshot {
        total_cpu_pct,
        core_usages: core_buf.as_slice(),
        gpu_pct,
        gpu_cores,
        mem_used_pct,
        mem_used_gb,
        mem_total_gb,
        swap_used_pct,
        active_mode,
        next_mode,
        mode_index,
        total_modes,
        tick,
        demo_mode_active: in_demo,
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let mut custom_config: Option<PathBuf> = None;
    let mut max_frames: Option<u64> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--config" | "-c" => {
                if let Some(p) = args.get(i + 1) {
                    custom_config = Some(PathBuf::from(p));
                    i += 1;
                }
            }
            "--frames" => {
                if let Some(n) = args.get(i + 1) {
                    max_frames = n.parse().ok();
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }

    let (mut cfg, cfg_path) = QDeckConfig::load(custom_config.as_deref())?;
    let mut theme = cfg.theme.resolve();
    let mut modes = cfg.effective_modes();
    let mut mode_idx = modes
        .iter()
        .position(|m| m.eq_ignore_ascii_case(&cfg.default_mode))
        .unwrap_or(0);
    let mut last_mtime = file_mtime(&cfg_path);

    println!("====================================================");
    println!("  QDECK // TACTICAL OPS CENTER");
    println!("  Config file: {}", cfg_path.display());
    println!("  Refresh interval: {}ms", cfg.refresh_ms.max(250));
    println!("  Modes: {:?} (default: '{}')", modes, modes[mode_idx]);
    println!("====================================================");

    let mut deck = connect_deck(&cfg)?;
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

    let (mut gpu_cores, mut live_gpu_pct) = query_macos_gpu_stats(16);
    println!("[QDECK] Detected GPU Cores: {}", gpu_cores);

    // Use blocking mode with timed `read_buttons(Some(wait_duration))` so the process sleeps
    // inside the kernel IOKit event wait (0% CPU between 1-second ticks) and wakes immediately on button press.
    let _ = deck.set_blocking(true);

    let mut sys = System::new_with_specifics(
        RefreshKind::nothing()
            .with_cpu(CpuRefreshKind::everything())
            .with_memory(MemoryRefreshKind::everything()),
    );
    sys.refresh_cpu_usage();
    sys.refresh_memory();

    let mut tick: u64 = 0;
    let mut demo_until: Option<Instant> = None;
    let mut prev_buttons = vec![0u8; total_keys as usize];
    let mut highlight_until: Vec<Option<Instant>> = vec![None; total_keys as usize];
    let mut core_buf: Vec<f32> = Vec::with_capacity(32);

    {
        let total_modes = modes.len().max(1);
        let active_mode = &modes[mode_idx];
        let next_mode = &modes[(mode_idx + 1) % total_modes];
        let initial_snapshot = build_snapshot(
            &sys,
            &mut core_buf,
            gpu_cores,
            live_gpu_pct,
            active_mode,
            next_mode,
            mode_idx,
            total_modes,
            tick,
            false,
        );
        apply_mode_layout(
            &mut deck,
            &cfg,
            &theme,
            cols,
            total_keys,
            img_w,
            img_h,
            &initial_snapshot,
        )?;
    }

    let mut last_render = Instant::now();

    loop {
        let refresh_interval = Duration::from_millis(cfg.refresh_ms.max(250));
        let now = Instant::now();

        // Compute how long we can sleep in kernel HID wait until either:
        // 1) An active 120ms tap highlight expires, OR
        // 2) The 1-second telemetry refresh interval is due
        let next_telemetry_in = refresh_interval.saturating_sub(now.duration_since(last_render));
        let next_highlight_expiry = highlight_until
            .iter()
            .filter_map(|&opt| opt)
            .map(|deadline| deadline.saturating_duration_since(now))
            .min();

        let wait_duration = match next_highlight_expiry {
            Some(h_wait) => next_telemetry_in.min(h_wait).max(Duration::from_millis(1)),
            None => next_telemetry_in.max(Duration::from_millis(1)),
        };

        let mut newly_highlighted_keys: Vec<u8> = Vec::new();
        let mut mode_switched = false;

        if let Ok(buttons) = deck.read_buttons(Some(wait_duration)) {
            let press_time = Instant::now();
            for (k_idx, (&prev, &curr)) in prev_buttons.iter().zip(buttons.iter()).enumerate() {
                let key_u8 = k_idx as u8;
                // Edge-trigger on key press (0 -> 1)
                if prev == 0 && curr != 0 {
                    highlight_until[k_idx] = Some(press_time + TAP_HIGHLIGHT_DURATION);
                    newly_highlighted_keys.push(key_u8);
                    let active_mode = &modes[mode_idx];
                    if let Some(tile) = cfg.tile_for_key(key_u8, cols, total_keys, active_mode) {
                        match tile.click_action() {
                            ClickAction::CycleMode => {
                                mode_idx = (mode_idx + 1) % modes.len().max(1);
                                println!("[QDECK] Mode toggled -> '{}'", modes[mode_idx]);
                                mode_switched = true;
                            }
                            ClickAction::OpenApp(app_name) => {
                                println!(
                                    "[QDECK] Click -> Opening macOS application: '{}'",
                                    app_name
                                );
                                if let Err(e) =
                                    Command::new("open").arg("-a").arg(&app_name).spawn()
                                {
                                    eprintln!("[QDECK] Failed to open app '{}': {}", app_name, e);
                                }
                            }
                            ClickAction::ShellCommand(cmd) => {
                                println!("[QDECK] Click -> Executing command: '{}'", cmd);
                                if let Err(e) = Command::new("/bin/sh").arg("-c").arg(&cmd).spawn()
                                {
                                    eprintln!("[QDECK] Failed to run command '{}': {}", cmd, e);
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

        // Expire any finished 120ms tap flashes
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

        if mode_switched {
            let total_modes = modes.len().max(1);
            let active_mode = &modes[mode_idx];
            let next_mode = &modes[(mode_idx + 1) % total_modes];
            let snapshot = build_snapshot(
                &sys,
                &mut core_buf,
                gpu_cores,
                live_gpu_pct,
                active_mode,
                next_mode,
                mode_idx,
                total_modes,
                tick,
                in_demo,
            );
            let _ = apply_mode_layout(
                &mut deck, &cfg, &theme, cols, total_keys, img_w, img_h, &snapshot,
            );
            // Keep the mode button briefly flashed after toggle
            for &key_u8 in &newly_highlighted_keys {
                let _ = render_key_to_deck(
                    &mut deck, &cfg, &theme, cols, total_keys, img_w, img_h, key_u8, &snapshot,
                    true,
                );
            }
            last_render = Instant::now();
            continue;
        }

        if !newly_highlighted_keys.is_empty() || !expired_highlight_keys.is_empty() {
            let total_modes = modes.len().max(1);
            let active_mode = &modes[mode_idx];
            let next_mode = &modes[(mode_idx + 1) % total_modes];
            let snapshot = build_snapshot(
                &sys,
                &mut core_buf,
                gpu_cores,
                live_gpu_pct,
                active_mode,
                next_mode,
                mode_idx,
                total_modes,
                tick,
                in_demo,
            );
            for key_u8 in newly_highlighted_keys {
                let _ = render_key_to_deck(
                    &mut deck, &cfg, &theme, cols, total_keys, img_w, img_h, key_u8, &snapshot,
                    true,
                );
            }
            for key_u8 in expired_highlight_keys {
                let _ = render_key_to_deck(
                    &mut deck, &cfg, &theme, cols, total_keys, img_w, img_h, key_u8, &snapshot,
                    false,
                );
            }
        }

        // Once-per-second telemetry & config check
        if last_render.elapsed() >= refresh_interval {
            // Check config file modification once per second
            let current_mtime = file_mtime(&cfg_path);
            if current_mtime != last_mtime && current_mtime.is_some() {
                last_mtime = current_mtime;
                match QDeckConfig::load_from_file(&cfg_path) {
                    Ok(new_cfg) => {
                        println!(
                            "[QDECK] Hot-reloaded configuration from {}",
                            cfg_path.display()
                        );
                        let prev_mode_name = modes.get(mode_idx).cloned().unwrap_or_default();
                        cfg = new_cfg;
                        theme = cfg.theme.resolve();
                        modes = cfg.effective_modes();
                        mode_idx = modes
                            .iter()
                            .position(|m| m == &prev_mode_name)
                            .unwrap_or(0);
                        let total_modes = modes.len().max(1);
                        let active_mode = &modes[mode_idx];
                        let next_mode = &modes[(mode_idx + 1) % total_modes];
                        let snapshot = build_snapshot(
                            &sys,
                            &mut core_buf,
                            gpu_cores,
                            live_gpu_pct,
                            active_mode,
                            next_mode,
                            mode_idx,
                            total_modes,
                            tick,
                            in_demo,
                        );
                        let _ = apply_mode_layout(
                            &mut deck, &cfg, &theme, cols, total_keys, img_w, img_h, &snapshot,
                        );
                    }
                    Err(e) => {
                        eprintln!("[QDECK] Config reload error: {}", e);
                    }
                }
            }

            let total_modes = modes.len().max(1);
            let active_mode = &modes[mode_idx];
            let next_mode = &modes[(mode_idx + 1) % total_modes];

            // Only poll the system sensors needed by the currently active mode's tiles
            let mut needs_cpu = false;
            let mut needs_gpu = false;
            let mut needs_mem = false;
            for k in 0..total_keys {
                if let Some(t) = cfg.tile_for_key(k, cols, total_keys, active_mode) {
                    match t.tile_type.trim().to_ascii_lowercase().as_str() {
                        "cpu" => needs_cpu = true,
                        "gpu" => needs_gpu = true,
                        "memory" | "ram" => needs_mem = true,
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
                let (detected_cores, gpu_util) = query_macos_gpu_stats(gpu_cores);
                gpu_cores = detected_cores;
                live_gpu_pct = gpu_util;
            }

            let snapshot = build_snapshot(
                &sys,
                &mut core_buf,
                gpu_cores,
                live_gpu_pct,
                active_mode,
                next_mode,
                mode_idx,
                total_modes,
                tick,
                in_demo,
            );

            // Only re-render dynamic telemetry tiles on the 1-second tick
            for k in 0..total_keys {
                if let Some(tile) = cfg.tile_for_key(k, cols, total_keys, active_mode) {
                    if is_dynamic_telemetry_tile(&tile.tile_type) {
                        let is_hi = highlight_until
                            .get(k as usize)
                            .and_then(|&d| d)
                            .is_some_and(|deadline| Instant::now() < deadline);
                        render_key_to_deck(
                            &mut deck, &cfg, &theme, cols, total_keys, img_w, img_h, k, &snapshot,
                            is_hi,
                        )?;
                    }
                }
            }

            last_render = Instant::now();
            tick += 1;
            if let Some(max) = max_frames {
                if tick >= max {
                    break;
                }
            }
        }
    }

    Ok(())
}
