pub mod ai_limits;
pub mod airbus;
pub mod airport;
pub mod audio;
pub mod disk;
pub mod gauge;
pub mod icons;
pub mod navigation;
pub mod system;

pub use ai_limits::{render_claude_limits_tile, render_gemini_limits_tile};
pub use airbus::render_a320_fcu_tile;
pub use airport::render_airport_tile;
pub use audio::render_audio_tile;
pub use disk::render_disk_tile;
pub use gauge::{GaugeTile, format_binary_bytes, render_gauge_tile};
pub use navigation::{
    render_launcher_tile, render_mode_tile, render_panel_tile, render_standby_tile,
};
pub use system::{render_cpu_tile, render_gpu_tile, render_memory_tile, render_ops_status_tile};

use crate::config::{ResolvedTheme, TileConfig};
use crate::telemetry::{AirportStatus, AudioDevices, ClaudeLimits, DisksState, GeminiLimits};
use crate::xplane::A320FcuState;
use image::RgbImage;

pub struct TelemetrySnapshot<'a> {
    pub total_cpu_pct: f32,
    pub load_avg_1m: f32,
    pub core_usages: &'a [f32],
    pub gpu_pct: f32,
    pub gpu_cores: usize,
    pub gpu_mem_used_bytes: u64,
    pub gpu_mem_alloc_bytes: u64,
    pub mem_used_pct: f32,
    pub mem_used_gb: f32,
    pub mem_total_gb: f32,
    pub swap_used_pct: f32,
    pub swap_used_bytes: u64,
    pub swap_total_bytes: u64,
    pub claude_limits: ClaudeLimits,
    pub gemini_limits: GeminiLimits,
    pub airport: AirportStatus,
    pub audio_devices: &'a AudioDevices,
    pub disks: &'a DisksState,
    pub active_mode: &'a str,
    pub next_mode: &'a str,
    pub mode_index: usize,
    pub total_modes: usize,
    pub active_panel: &'a str,
    pub next_panel: &'a str,
    pub panel_index: usize,
    pub total_panels: usize,
    pub fcu: &'a A320FcuState,
    pub tick: u64,
    pub demo_mode_active: bool,
}

/// Dispatches rendering for a configured `TileConfig` to its specialized renderer.
pub fn render_tile_image(
    width: u32,
    height: u32,
    key_idx: u8,
    tile: &TileConfig,
    theme: &ResolvedTheme,
    snapshot: &TelemetrySnapshot,
    is_highlighted: bool,
) -> RgbImage {
    match tile.tile_type.trim().to_ascii_lowercase().as_str() {
        "mode" | "mode_selector" => {
            render_mode_tile(width, height, tile, theme, snapshot, is_highlighted)
        }
        "panel" | "panel_selector" | "submode" => {
            render_panel_tile(width, height, tile, theme, snapshot, is_highlighted)
        }
        "cpu" => render_cpu_tile(width, height, tile, theme, snapshot, is_highlighted),
        "gpu" => render_gpu_tile(width, height, tile, theme, snapshot, is_highlighted),
        "memory" | "ram" => {
            render_memory_tile(width, height, tile, theme, snapshot, is_highlighted)
        }
        "disk" | "storage" | "volume" => {
            render_disk_tile(width, height, tile, theme, snapshot, is_highlighted)
        }
        "claude" | "claude_limits" => {
            render_claude_limits_tile(width, height, tile, theme, snapshot, is_highlighted)
        }
        "gemini" | "gemini_limits" => {
            render_gemini_limits_tile(width, height, tile, theme, snapshot, is_highlighted)
        }
        "airport" => render_airport_tile(width, height, tile, theme, snapshot, is_highlighted),
        "audio" | "audio_io" | "sound" => {
            render_audio_tile(width, height, tile, theme, snapshot, is_highlighted)
        }
        "a320_fcu" | "xplane" | "fcu" | "a320" => {
            render_a320_fcu_tile(width, height, tile, theme, snapshot, is_highlighted)
        }
        "ops_status" | "status" => {
            render_ops_status_tile(width, height, tile, theme, snapshot, is_highlighted)
        }
        "launcher" | "command" | "cmd" => {
            render_launcher_tile(width, height, tile, theme, is_highlighted)
        }
        _ => render_standby_tile(width, height, key_idx, theme, is_highlighted),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::QDeckConfig;
    use crate::telemetry::{AudioEndpoint, DiskVolume};
    use std::sync::LazyLock;

    static NO_AUDIO: LazyLock<AudioDevices> = LazyLock::new(AudioDevices::default);
    static NO_DISKS: LazyLock<DisksState> = LazyLock::new(DisksState::default);

    /// A plausible dashboard snapshot carrying the given airport status.
    pub(super) fn snapshot_with_airport(
        fcu: &A320FcuState,
        airport: AirportStatus,
    ) -> TelemetrySnapshot<'_> {
        TelemetrySnapshot {
            total_cpu_pct: 42.0,
            load_avg_1m: 2.0,
            core_usages: &[12.0, 35.0, 68.0, 91.0],
            gpu_pct: 55.0,
            gpu_cores: 40,
            gpu_mem_used_bytes: 0,
            gpu_mem_alloc_bytes: 0,
            mem_used_pct: 62.0,
            mem_used_gb: 20.0,
            mem_total_gb: 32.0,
            swap_used_pct: 10.0,
            swap_used_bytes: 0,
            swap_total_bytes: 0,
            claude_limits: ClaudeLimits::default(),
            gemini_limits: GeminiLimits::default(),
            airport,
            audio_devices: &NO_AUDIO,
            disks: &NO_DISKS,
            active_mode: "dashboard",
            next_mode: "xplane",
            mode_index: 0,
            total_modes: 2,
            active_panel: "fcu",
            next_panel: "apu",
            panel_index: 0,
            total_panels: 4,
            fcu,
            tick: 1,
            demo_mode_active: false,
        }
    }

    #[test]
    fn test_all_builtin_tiles_render_72x72_non_blank() {
        let cfg = QDeckConfig::builtin_default();
        let theme = cfg.theme.resolve();
        let fcu = A320FcuState::default();
        let cores = [12.0, 35.0, 68.0, 91.0, 20.0, 15.0, 40.0, 55.0];
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
            volumes: vec![DiskVolume {
                raw_name: "Macintosh HD".to_string(),
                short_name: "MAC HD".to_string(),
                mount_point: "/".to_string(),
                total_bytes: 2_000_000_000_000,
                used_bytes: 867_000_000_000,
                avail_bytes: 1_133_000_000_000,
                used_pct: 43.4,
            }],
            active_index: 0,
            available: true,
        };
        let snap = TelemetrySnapshot {
            total_cpu_pct: 42.0,
            load_avg_1m: 2.15,
            core_usages: &cores,
            gpu_pct: 55.0,
            gpu_cores: 40,
            gpu_mem_used_bytes: 1_618_493_440,
            gpu_mem_alloc_bytes: 16_684_777_472,
            mem_used_pct: 62.0,
            mem_used_gb: 20.0,
            mem_total_gb: 32.0,
            swap_used_pct: 60.0,
            swap_used_bytes: 1_939_865_600,
            swap_total_bytes: 3_221_225_472,
            claude_limits: ClaudeLimits {
                session_pct: 5.0,
                weekly_pct: 1.0,
                available: true,
            },
            gemini_limits: GeminiLimits {
                gemini: crate::telemetry::QuotaWindowPair {
                    session_pct: 5.0,
                    weekly_pct: 3.0,
                    available: true,
                },
                argon: crate::telemetry::QuotaWindowPair {
                    session_pct: 0.0,
                    weekly_pct: 66.0,
                    available: true,
                },
                claude_gpt: crate::telemetry::QuotaWindowPair {
                    session_pct: 0.0,
                    weekly_pct: 3.0,
                    available: true,
                },
                page_index: 0,
                available: true,
            },
            airport: AirportStatus::default(),
            audio_devices: &audio_devices,
            disks: &disks,
            active_mode: "xplane",
            next_mode: "dashboard",
            mode_index: 1,
            total_modes: 2,
            active_panel: "flt",
            next_panel: "sys",
            panel_index: 2,
            total_panels: 4,
            fcu: &fcu,
            tick: 1,
            demo_mode_active: false,
        };

        for (idx, tile) in cfg.tiles.iter().enumerate() {
            let img = render_tile_image(72, 72, (idx % 15) as u8, tile, &theme, &snap, false);
            assert_eq!(img.width(), 72);
            assert_eq!(img.height(), 72);
            let non_black_pixels = img.pixels().filter(|p| p.0 != [0, 0, 0]).count();
            assert!(
                non_black_pixels > 20,
                "Tile {:?} rendered fewer than 20 lit pixels",
                tile.control.as_deref().unwrap_or(&tile.tile_type)
            );
        }
    }
}
