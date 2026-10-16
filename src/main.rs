mod config;
mod font;
mod widgets;
mod xplane;

use anyhow::{Context, Result, anyhow};
use config::{ClickAction, QDeckConfig, ResolvedTheme};
use image::DynamicImage;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant, SystemTime};
use streamdeck::{Colour, Kind, StreamDeck, pids};
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};
use widgets::{
    TelemetrySnapshot, render_a320_fcu_tile, render_cpu_tile, render_gpu_tile,
    render_launcher_tile, render_memory_tile, render_mode_tile, render_ops_status_tile,
    render_standby_tile,
};
use xplane::{A320FcuState, XPlaneClient};

const TAP_HIGHLIGHT_DURATION: Duration = Duration::from_millis(120);
const LAUNCH_AGENT_LABEL: &str = "local.qdeck";

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
            "Could not connect to Stream Deck: {e}. Make sure the Stream Deck is plugged in and the official Elgato app is closed."
        )
    })
}

fn file_mtime(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).ok().and_then(|m| m.modified().ok())
}

fn is_dynamic_telemetry_tile(tile_type: &str) -> bool {
    matches!(
        tile_type.trim().to_ascii_lowercase().as_str(),
        "cpu"
            | "gpu"
            | "memory"
            | "ram"
            | "ops_status"
            | "status"
            | "a320_fcu"
            | "xplane"
            | "fcu"
            | "a320"
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
            "a320_fcu" | "xplane" | "fcu" | "a320" => render_a320_fcu_tile(
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
    deck.set_brightness(cfg.brightness)?;
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
fn build_snapshot<'a>(
    sys: &System,
    core_buf: &'a mut Vec<f32>,
    gpu_cores: usize,
    live_gpu_pct: f32,
    active_mode: &'a str,
    next_mode: &'a str,
    mode_index: usize,
    total_modes: usize,
    fcu: &'a A320FcuState,
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
        fcu,
        tick,
        demo_mode_active: in_demo,
    }
}

fn launch_agent_plist_path() -> Result<PathBuf> {
    let home = env::var("HOME").context("HOME environment variable not set")?;
    Ok(PathBuf::from(home)
        .join("Library")
        .join("LaunchAgents")
        .join(format!("{LAUNCH_AGENT_LABEL}.plist")))
}

fn install_launch_agent() -> Result<()> {
    let home = env::var("HOME").context("HOME environment variable not set")?;
    let exe_path = env::current_exe().context("Failed to resolve current executable path")?;
    let plist_path = launch_agent_plist_path()?;
    let log_path = PathBuf::from(&home)
        .join("Library")
        .join("Logs")
        .join("qdeck.log");
    let work_dir = exe_path
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .unwrap_or_else(|| Path::new(&home));

    if let Some(parent) = plist_path.parent() {
        fs::create_dir_all(parent)?;
    }
    if let Some(log_parent) = log_path.parent() {
        let _ = fs::create_dir_all(log_parent);
    }

    let plist_content = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>{label}</string>
  <key>ProgramArguments</key>
  <array>
    <string>{exe}</string>
  </array>
  <key>WorkingDirectory</key>
  <string>{workdir}</string>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
  <key>ThrottleInterval</key>
  <integer>2</integer>
  <key>ProcessType</key>
  <string>Interactive</string>
  <key>StandardOutPath</key>
  <string>{log}</string>
  <key>StandardErrorPath</key>
  <string>{log}</string>
</dict>
</plist>
"#,
        label = LAUNCH_AGENT_LABEL,
        exe = exe_path.display(),
        workdir = work_dir.display(),
        log = log_path.display()
    );

    fs::write(&plist_path, plist_content)
        .with_context(|| format!("Failed to write {}", plist_path.display()))?;

    let _ = Command::new("launchctl")
        .args(["unload", "-w"])
        .arg(&plist_path)
        .status();
    let status = Command::new("launchctl")
        .args(["load", "-w"])
        .arg(&plist_path)
        .status()
        .context("Failed to execute launchctl load")?;

    if !status.success() {
        return Err(anyhow!(
            "launchctl load exited with status {:?} for {}",
            status.code(),
            plist_path.display()
        ));
    }

    println!(
        "[QDECK] Installed & started macOS LaunchAgent: {}",
        plist_path.display()
    );
    println!("[QDECK] Binary: {}", exe_path.display());
    println!("[QDECK] Logs:   {}", log_path.display());
    Ok(())
}

fn uninstall_launch_agent() -> Result<()> {
    let plist_path = launch_agent_plist_path()?;
    if plist_path.exists() {
        let _ = Command::new("launchctl")
            .args(["unload", "-w"])
            .arg(&plist_path)
            .status();
        fs::remove_file(&plist_path)?;
        println!(
            "[QDECK] Removed macOS LaunchAgent: {}",
            plist_path.display()
        );
    } else {
        println!(
            "[QDECK] LaunchAgent not installed at {}",
            plist_path.display()
        );
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn run_deck_session(
    mut deck: StreamDeck,
    cfg: &mut QDeckConfig,
    cfg_path: &Path,
    theme: &mut ResolvedTheme,
    modes: &mut Vec<String>,
    mode_idx: &mut usize,
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

    {
        let total_modes = modes.len().max(1);
        let active_mode = &modes[*mode_idx];
        let next_mode = &modes[(*mode_idx + 1) % total_modes];
        let initial_snapshot = build_snapshot(
            sys,
            &mut core_buf,
            *gpu_cores,
            *live_gpu_pct,
            active_mode,
            next_mode,
            *mode_idx,
            total_modes,
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
        let refresh_interval = Duration::from_millis(cfg.refresh_ms.max(250));
        let now = Instant::now();

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
        let mut fcu_changed = false;

        match deck.read_buttons(Some(wait_duration)) {
            Ok(buttons) => {
                let press_time = Instant::now();
                for (k_idx, (&prev, &curr)) in prev_buttons.iter().zip(buttons.iter()).enumerate() {
                    let key_u8 = k_idx as u8;
                    // Edge-trigger on key press (0 -> 1)
                    if prev == 0 && curr != 0 {
                        highlight_until[k_idx] = Some(press_time + TAP_HIGHLIGHT_DURATION);
                        newly_highlighted_keys.push(key_u8);
                        let active_mode = &modes[*mode_idx];
                        if let Some(tile) = cfg.tile_for_key(key_u8, cols, total_keys, active_mode)
                        {
                            match tile.click_action() {
                                ClickAction::CycleMode => {
                                    *mode_idx = (*mode_idx + 1) % modes.len().max(1);
                                    println!("[QDECK] Mode toggled -> '{}'", modes[*mode_idx]);
                                    mode_switched = true;
                                }
                                ClickAction::XPlaneControl {
                                    control,
                                    custom_cmd,
                                } => {
                                    println!(
                                        "[QDECK] [X-Plane A320] Triggered FCU control '{}' -> UDP {}",
                                        control, cfg.xplane_addr
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
            let active_mode = &modes[*mode_idx];
            let next_mode = &modes[(*mode_idx + 1) % total_modes];
            let snapshot = build_snapshot(
                sys,
                &mut core_buf,
                *gpu_cores,
                *live_gpu_pct,
                active_mode,
                next_mode,
                *mode_idx,
                total_modes,
                &xplane.state,
                *tick,
                in_demo,
            );
            apply_mode_layout(&mut deck, cfg, theme, cols, total_keys, img_w, img_h, &snapshot)?;
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
            let snapshot = build_snapshot(
                sys,
                &mut core_buf,
                *gpu_cores,
                *live_gpu_pct,
                active_mode,
                next_mode,
                *mode_idx,
                total_modes,
                &xplane.state,
                *tick,
                in_demo,
            );

            if fcu_changed {
                for k in 0..total_keys {
                    if let Some(t) = cfg.tile_for_key(k, cols, total_keys, active_mode) {
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

        // Once-per-second telemetry & config check
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
                        let snapshot = build_snapshot(
                            sys,
                            &mut core_buf,
                            *gpu_cores,
                            *live_gpu_pct,
                            active_mode,
                            next_mode,
                            *mode_idx,
                            total_modes,
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

            let mut needs_cpu = false;
            let mut needs_gpu = false;
            let mut needs_mem = false;
            let mut needs_xplane = false;
            for k in 0..total_keys {
                if let Some(t) = cfg.tile_for_key(k, cols, total_keys, active_mode) {
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
                xplane.poll_incoming();
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
                &xplane.state,
                *tick,
                in_demo,
            );

            for k in 0..total_keys {
                if let Some(tile) = cfg.tile_for_key(k, cols, total_keys, active_mode) {
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
                    return Ok(true);
                }
            }
        }
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let mut custom_config: Option<PathBuf> = None;
    let mut max_frames: Option<u64> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--install-service" => {
                return install_launch_agent();
            }
            "--uninstall-service" => {
                return uninstall_launch_agent();
            }
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
    println!("  QDECK // TACTICAL OPS CENTER (RELEASE SERVICE)");
    println!("  Config file: {}", cfg_path.display());
    println!("  Refresh interval: {}ms", cfg.refresh_ms.max(250));
    println!("  Modes: {:?} (default: '{}')", modes, modes[mode_idx]);
    println!("  X-Plane UDP Endpoint: {}", cfg.xplane_addr);
    println!("====================================================");

    let mut xplane = XPlaneClient::new(&cfg.xplane_addr);
    let (mut gpu_cores, mut live_gpu_pct) = query_macos_gpu_stats(16);
    println!("[QDECK] Detected GPU Cores: {}", gpu_cores);

    let mut sys = System::new_with_specifics(
        RefreshKind::nothing()
            .with_cpu(CpuRefreshKind::everything())
            .with_memory(MemoryRefreshKind::everything()),
    );
    sys.refresh_cpu_usage();
    sys.refresh_memory();

    let mut tick: u64 = 0;
    let mut logged_waiting = false;

    loop {
        match connect_deck(&cfg) {
            Ok(deck) => {
                logged_waiting = false;
                match run_deck_session(
                    deck,
                    &mut cfg,
                    &cfg_path,
                    &mut theme,
                    &mut modes,
                    &mut mode_idx,
                    &mut last_mtime,
                    &mut xplane,
                    &mut sys,
                    &mut gpu_cores,
                    &mut live_gpu_pct,
                    &mut tick,
                    max_frames,
                ) {
                    Ok(true) => break,
                    Ok(false) => {}
                    Err(e) => {
                        eprintln!(
                            "[QDECK] Stream Deck disconnected ({e}). Reconnecting in 2s..."
                        );
                        thread::sleep(Duration::from_secs(2));
                    }
                }
            }
            Err(e) => {
                if !logged_waiting {
                    eprintln!("[QDECK] {e} — retrying every 3s...");
                    logged_waiting = true;
                }
                thread::sleep(Duration::from_secs(3));
            }
        }
    }

    Ok(())
}
