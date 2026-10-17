use anyhow::Result;
use qdeck::config::QDeckConfig;
use qdeck::deck::{connect_deck, file_mtime, run_deck_session};
use qdeck::macos::{
    install_launch_agent, install_signal_handlers, is_shutdown_requested, uninstall_launch_agent,
};
use qdeck::telemetry::query_macos_gpu_stats;
use qdeck::xplane::XPlaneClient;
use std::env;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};

fn main() -> Result<()> {
    install_signal_handlers();

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
    let mut panel_idx: usize = 0;
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

    while !is_shutdown_requested() {
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
                    &mut panel_idx,
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
                        if is_shutdown_requested() {
                            break;
                        }
                        eprintln!("[QDECK] Stream Deck disconnected ({e}). Reconnecting in 2s...");
                        thread::sleep(Duration::from_secs(2));
                    }
                }
            }
            Err(e) => {
                if !logged_waiting {
                    eprintln!("[QDECK] {e} — retrying every 3s...");
                    logged_waiting = true;
                }
                for _ in 0..20 {
                    if is_shutdown_requested() {
                        break;
                    }
                    thread::sleep(Duration::from_millis(150));
                }
            }
        }
    }

    Ok(())
}
