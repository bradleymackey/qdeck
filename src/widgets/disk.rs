use super::TelemetrySnapshot;
use super::gauge::{GaugeTile, render_gauge_tile};
use crate::config::{ResolvedTheme, TileConfig};
use crate::telemetry::format_compact_bytes;
use image::RgbImage;

/// Renders a tactical Disk Usage tile (`type = "disk"`) displaying the currently selected
/// mounted volume (`1/N`), its utilization percentage, a 15-segment bar, and `USED` / `FREE` capacity.
pub fn render_disk_tile(
    width: u32,
    height: u32,
    tile: &TileConfig,
    theme: &ResolvedTheme,
    telemetry: &TelemetrySnapshot,
    is_highlighted: bool,
) -> RgbImage {
    let warn = tile.warn_threshold.unwrap_or(75.0);
    let crit = tile.crit_threshold.unwrap_or(90.0);

    let active_vol = telemetry.disks.active_volume();
    let available = active_vol.is_some();

    let (vol_name, used_pct, used_str, free_str, badge) =
        if let Some((vol, idx, total)) = active_vol {
            let pct = if telemetry.demo_mode_active {
                let phase = (telemetry.tick as f32) * 0.65;
                (30.0 + ((phase * 0.75).sin() * 0.5 + 0.5) * 65.0).clamp(10.0, 96.0)
            } else {
                vol.used_pct.clamp(0.0, 100.0)
            };
            (
                vol.short_name.as_str(),
                pct,
                format_compact_bytes(vol.used_bytes),
                format_compact_bytes(vol.avail_bytes),
                format!("{}/{}", idx + 1, total.max(1)),
            )
        } else {
            (
                "NONE",
                0.0,
                "--G".to_string(),
                "--G".to_string(),
                "0/0".to_string(),
            )
        };

    let vol_col = if available {
        theme.color_for_pct(used_pct, Some(warn), Some(crit))
    } else {
        theme.phosphor_dim
    };
    let free_col = if available {
        theme.phosphor_green
    } else {
        theme.phosphor_dim
    };

    render_gauge_tile(
        width,
        height,
        theme,
        is_highlighted,
        &GaugeTile {
            header: tile.label.as_deref().unwrap_or("DISK"),
            badge: Some(&badge),
            accent: vol_col,
            active: available,
            row_tag: vol_name,
            pct: available.then_some(used_pct),
            left_tag: "USED",
            right_tag: "FREE",
            left_value: &used_str,
            right_value: &free_str,
            box_accent: vol_col,
            left_color: vol_col,
            right_color: free_col,
        },
    )
}
