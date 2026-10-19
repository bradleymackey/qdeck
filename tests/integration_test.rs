use qdeck::config::{ClickAction, QDeckConfig};
use qdeck::telemetry::{
    AirportStatus, AudioDevices, AudioEndpoint, ClaudeLimits, DiskVolume, DisksState, GeminiLimits,
};
use qdeck::widgets::{TelemetrySnapshot, render_tile_image};
use qdeck::xplane::A320FcuState;
use qdeck::xplane::commands::dispatch_control;

#[test]
fn test_full_deck_matrix_across_all_modes_and_panels() {
    let cfg = QDeckConfig::builtin_default();
    let theme = cfg.theme.resolve();
    let mut fcu = A320FcuState::default();
    let cores = [10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0];
    let audio_devices = AudioDevices {
        output: AudioEndpoint {
            raw_name: "Bradley’s APP 2".to_string(),
            short_name: "APP 2".to_string(),
            transport: "BT".to_string(),
        },
        input: AudioEndpoint {
            raw_name: "Brio 300".to_string(),
            short_name: "BRIO 300".to_string(),
            transport: "USB".to_string(),
        },
        available: true,
    };
    let disks = DisksState {
        volumes: vec![
            DiskVolume {
                raw_name: "Macintosh HD".to_string(),
                short_name: "MAC HD".to_string(),
                mount_point: "/".to_string(),
                total_bytes: 2_000_000_000_000,
                used_bytes: 867_000_000_000,
                avail_bytes: 1_133_000_000_000,
                used_pct: 43.4,
            },
            DiskVolume {
                raw_name: "BANK-1".to_string(),
                short_name: "BANK-1".to_string(),
                mount_point: "/Volumes/BANK-1".to_string(),
                total_bytes: 2_000_000_000_000,
                used_bytes: 939_000_000_000,
                avail_bytes: 1_061_000_000_000,
                used_pct: 47.0,
            },
        ],
        active_index: 0,
        available: true,
    };

    let modes = cfg.effective_modes();
    assert_eq!(modes, vec!["dashboard", "xplane"]);

    let panels = cfg.effective_panels("xplane");
    assert_eq!(panels, vec!["fcu", "apu", "flt", "sys"]);

    for (m_idx, mode) in modes.iter().enumerate() {
        let mode_panels = cfg.effective_panels(mode);
        for (p_idx, panel) in mode_panels.iter().enumerate() {
            for key_idx in 0..15u8 {
                if let Some(tile) = cfg.tile_for_key(key_idx, 5, 15, mode, panel) {
                    if let ClickAction::XPlaneControl {
                        control,
                        custom_cmd,
                    } = tile.click_action()
                    {
                        dispatch_control(&mut fcu, &control, custom_cmd.as_deref(), |_| {});
                    }
                    let snap = TelemetrySnapshot {
                        total_cpu_pct: 45.0,
                        load_avg_1m: 3.4,
                        core_usages: &cores,
                        gpu_pct: 50.0,
                        gpu_cores: 40,
                        gpu_mem_used_bytes: 1_618_493_440,
                        gpu_mem_alloc_bytes: 16_684_777_472,
                        mem_used_pct: 55.0,
                        mem_used_gb: 18.0,
                        mem_total_gb: 32.0,
                        swap_used_pct: 60.0,
                        swap_used_bytes: 1_939_865_600,
                        swap_total_bytes: 3_221_225_472,
                        claude_limits: ClaudeLimits {
                            session_pct: 12.0,
                            weekly_pct: 34.0,
                            available: true,
                        },
                        gemini_limits: GeminiLimits {
                            gemini: qdeck::telemetry::QuotaWindowPair {
                                session_pct: 5.0,
                                weekly_pct: 3.0,
                                available: true,
                            },
                            argon: qdeck::telemetry::QuotaWindowPair {
                                session_pct: 0.0,
                                weekly_pct: 66.0,
                                available: true,
                            },
                            claude_gpt: qdeck::telemetry::QuotaWindowPair {
                                session_pct: 0.0,
                                weekly_pct: 3.0,
                                available: true,
                            },
                            page_index: (m_idx + p_idx) as u8 % 3,
                            available: true,
                        },
                        airport: AirportStatus {
                            page_index: (m_idx + p_idx) as u8 % 3,
                            ..AirportStatus::default()
                        },
                        audio_devices: &audio_devices,
                        disks: &disks,
                        active_mode: mode,
                        next_mode: &modes[(m_idx + 1) % modes.len()],
                        mode_index: m_idx,
                        total_modes: modes.len(),
                        active_panel: panel,
                        next_panel: &mode_panels[(p_idx + 1) % mode_panels.len()],
                        panel_index: p_idx,
                        total_panels: mode_panels.len(),
                        fcu: &fcu,
                        tick: 4,
                        demo_mode_active: false,
                    };
                    let normal_img = render_tile_image(72, 72, key_idx, tile, &theme, &snap, false);
                    let hi_img = render_tile_image(72, 72, key_idx, tile, &theme, &snap, true);
                    assert_eq!(normal_img.dimensions(), (72, 72));
                    assert_eq!(hi_img.dimensions(), (72, 72));
                    assert_ne!(normal_img.as_raw(), hi_img.as_raw());
                }
            }
        }
    }
}
