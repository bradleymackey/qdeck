pub mod ai_limits;
pub mod airbus;
pub mod icons;
pub mod navigation;
pub mod system;

pub use ai_limits::{render_claude_limits_tile, render_gemini_limits_tile};
pub use airbus::render_a320_fcu_tile;
pub use navigation::{
    render_launcher_tile, render_mode_tile, render_panel_tile, render_standby_tile,
};
pub use system::{
    gpu_grid_dimensions, render_cpu_tile, render_gpu_tile, render_memory_tile,
    render_ops_status_tile,
};

use crate::config::{ResolvedTheme, TileConfig};
use crate::telemetry::AiProviderLimits;
use crate::xplane::A320FcuState;
use image::RgbImage;

pub struct TelemetrySnapshot<'a> {
    pub total_cpu_pct: f32,
    pub core_usages: &'a [f32],
    pub gpu_pct: f32,
    pub gpu_cores: usize,
    pub mem_used_pct: f32,
    pub mem_used_gb: f32,
    pub mem_total_gb: f32,
    pub swap_used_pct: f32,
    pub claude_limits: AiProviderLimits,
    pub gemini_limits: AiProviderLimits,
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
        "claude" | "claude_limits" => {
            render_claude_limits_tile(width, height, tile, theme, snapshot, is_highlighted)
        }
        "gemini" | "gemini_limits" => {
            render_gemini_limits_tile(width, height, tile, theme, snapshot, is_highlighted)
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

    #[test]
    fn test_all_builtin_tiles_render_72x72_non_blank() {
        let cfg = QDeckConfig::builtin_default();
        let theme = cfg.theme.resolve();
        let fcu = A320FcuState::default();
        let cores = [12.0, 35.0, 68.0, 91.0, 20.0, 15.0, 40.0, 55.0];
        let snap = TelemetrySnapshot {
            total_cpu_pct: 42.0,
            core_usages: &cores,
            gpu_pct: 55.0,
            gpu_cores: 40,
            mem_used_pct: 62.0,
            mem_used_gb: 20.0,
            mem_total_gb: 32.0,
            swap_used_pct: 10.0,
            claude_limits: AiProviderLimits {
                session_pct: 5.0,
                weekly_pct: 1.0,
                available: true,
            },
            gemini_limits: AiProviderLimits {
                session_pct: 1.0,
                weekly_pct: 58.0,
                available: true,
            },
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
