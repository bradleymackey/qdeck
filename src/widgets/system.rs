use super::TelemetrySnapshot;
use super::gauge::{GaugeTile, format_binary_bytes, render_gauge_tile};
use crate::config::{ResolvedTheme, TileConfig};
use crate::font::{
    draw_corner_brackets, draw_hline, draw_rect_border, draw_text_3x5, draw_text_5x7, fill_rect,
    scale_color,
};
use image::RgbImage;

/// Renders a CPU utilization tile (`type = "cpu"`) in the shared gauge layout:
/// - Header: `CPU` label + `{n}C` core-count badge + status dot
/// - Row 1: `UTIL` overall utilization percentage + 15-segment progress bar
/// - Row 2: `PEAK | LOAD` box showing the busiest core's % and the 1-minute load average
pub fn render_cpu_tile(
    width: u32,
    height: u32,
    tile: &TileConfig,
    theme: &ResolvedTheme,
    telemetry: &TelemetrySnapshot,
    is_highlighted: bool,
) -> RgbImage {
    let warn = tile.warn_threshold.unwrap_or(theme.warn_threshold);
    let crit = tile.crit_threshold.unwrap_or(theme.crit_threshold);

    let total = telemetry.total_cpu_pct.clamp(0.0, 100.0);
    let accent = theme.color_for_pct(total, Some(warn), Some(crit));

    let peak = telemetry
        .core_usages
        .iter()
        .copied()
        .fold(0.0_f32, f32::max)
        .clamp(0.0, 100.0);
    let peak_col = theme.color_for_pct(peak, Some(warn), Some(crit));

    let badge = format!("{}C", telemetry.core_usages.len());
    let peak_str = format!("{}%", peak.round() as u32);
    let load_str = format_load_avg(telemetry.load_avg_1m);

    render_gauge_tile(
        width,
        height,
        theme,
        is_highlighted,
        &GaugeTile {
            header: tile.label.as_deref().unwrap_or("CPU"),
            badge: Some(&badge),
            accent,
            active: true,
            row_tag: "UTIL",
            pct: Some(total),
            left_tag: "PEAK",
            right_tag: "LOAD",
            left_value: &peak_str,
            right_value: &load_str,
            box_accent: peak_col,
            left_color: peak_col,
            right_color: theme.phosphor_green,
        },
    )
}

/// Renders a GPU utilization tile (`type = "gpu"`) in the shared gauge layout:
/// - Header: `GPU` label + `{n}C` GPU core-count badge + status dot
/// - Row 1: `UTIL` device utilization percentage + 15-segment progress bar
/// - Row 2: `VRAM | ALLOC` box showing GPU memory in use and total GPU-allocated memory
pub fn render_gpu_tile(
    width: u32,
    height: u32,
    tile: &TileConfig,
    theme: &ResolvedTheme,
    telemetry: &TelemetrySnapshot,
    is_highlighted: bool,
) -> RgbImage {
    let warn = tile.warn_threshold.unwrap_or(theme.warn_threshold);
    let crit = tile.crit_threshold.unwrap_or(theme.crit_threshold);

    let pct = telemetry.gpu_pct.clamp(0.0, 100.0);
    let accent = theme.color_for_pct(pct, Some(warn), Some(crit));

    let badge = format!("{}C", telemetry.gpu_cores.max(1));
    let vram_str = format_binary_bytes(telemetry.gpu_mem_used_bytes);
    let alloc_str = if telemetry.gpu_mem_alloc_bytes > 0 {
        format_binary_bytes(telemetry.gpu_mem_alloc_bytes)
    } else {
        "--".to_string()
    };

    render_gauge_tile(
        width,
        height,
        theme,
        is_highlighted,
        &GaugeTile {
            header: tile.label.as_deref().unwrap_or("GPU"),
            badge: Some(&badge),
            accent,
            active: true,
            row_tag: "UTIL",
            pct: Some(pct),
            left_tag: "VRAM",
            right_tag: "ALLOC",
            left_value: &vram_str,
            right_value: &alloc_str,
            box_accent: accent,
            left_color: accent,
            right_color: theme.phosphor_green,
        },
    )
}

/// Renders a Memory utilization tile (`type = "memory"`) in the shared gauge layout:
/// - Header: `MEM` label + `{used}/{total}G` RAM badge + status dot
/// - Row 1: `RAM` utilization percentage + 15-segment progress bar
/// - Row 2: `SWAP | SIZE` box showing swap used and total swap size
pub fn render_memory_tile(
    width: u32,
    height: u32,
    tile: &TileConfig,
    theme: &ResolvedTheme,
    telemetry: &TelemetrySnapshot,
    is_highlighted: bool,
) -> RgbImage {
    let warn = tile.warn_threshold.unwrap_or(75.0);
    let crit = tile.crit_threshold.unwrap_or(90.0);

    let used_pct = if telemetry.demo_mode_active {
        let phase = (telemetry.tick as f32) * 0.65;
        (25.0 + ((phase * 0.7 + 0.8).sin() * 0.5 + 0.5) * 68.0).clamp(10.0, 96.0)
    } else {
        telemetry.mem_used_pct.clamp(0.0, 100.0)
    };
    let mem_col = theme.color_for_pct(used_pct, Some(warn), Some(crit));

    let used_gb = telemetry.mem_used_gb.round().max(0.0) as u32;
    let total_gb = telemetry.mem_total_gb.round().max(1.0) as u32;
    let badge = format!("{}/{}G", used_gb, total_gb);

    let swap_pct = telemetry.swap_used_pct.clamp(0.0, 100.0);
    let swap_col = if telemetry.swap_total_bytes > 0 {
        theme.color_for_pct(swap_pct, Some(warn), Some(crit))
    } else {
        theme.phosphor_green
    };
    let swap_used_str = format_binary_bytes(telemetry.swap_used_bytes);
    let swap_total_str = format_binary_bytes(telemetry.swap_total_bytes);

    render_gauge_tile(
        width,
        height,
        theme,
        is_highlighted,
        &GaugeTile {
            header: tile.label.as_deref().unwrap_or("MEM"),
            badge: Some(&badge),
            accent: mem_col,
            active: true,
            row_tag: "RAM",
            pct: Some(used_pct),
            left_tag: "SWAP",
            right_tag: "SIZE",
            left_value: &swap_used_str,
            right_value: &swap_total_str,
            box_accent: swap_col,
            left_color: swap_col,
            right_color: theme.phosphor_green,
        },
    )
}

/// Formats a load average into at most 4 characters (e.g. `"0.0"`, `"5.2"`, `"12"`).
fn format_load_avg(load: f32) -> String {
    let l = load.max(0.0);
    if l < 9.95 {
        format!("{:.1}", l)
    } else {
        format!("{:.0}", l.min(999.0))
    }
}

/// Renders an ops-center command/status tile showing system state and heartbeat.
pub fn render_ops_status_tile(
    width: u32,
    height: u32,
    tile: &TileConfig,
    theme: &ResolvedTheme,
    telemetry: &TelemetrySnapshot,
    is_highlighted: bool,
) -> RgbImage {
    let w = width as i32;
    let h = height as i32;
    let bg = if is_highlighted {
        scale_color(theme.phosphor_green, 0.22)
    } else {
        theme.background
    };
    let mut img = RgbImage::from_pixel(width, height, bg);

    let peak_core = telemetry
        .core_usages
        .iter()
        .copied()
        .fold(0.0_f32, f32::max);
    let max_metric = telemetry
        .total_cpu_pct
        .max(peak_core * 0.92)
        .max(telemetry.gpu_pct);
    let status_color = theme.color_for_pct(max_metric, tile.warn_threshold, tile.crit_threshold);

    if is_highlighted {
        draw_rect_border(&mut img, 0, 0, w, h, 2, theme.phosphor_green);
        fill_rect(&mut img, 2, 2, w - 4, 12, theme.phosphor_green);
    } else {
        draw_corner_brackets(&mut img, 1, 1, w - 2, h - 2, 6, theme.phosphor_dim);
    }

    let title = tile.label.as_deref().unwrap_or("QDECK");
    draw_text_5x7(
        &mut img,
        6,
        5,
        title,
        1,
        if is_highlighted {
            theme.background
        } else {
            theme.phosphor_green
        },
    );

    let pulse_on = telemetry.tick.is_multiple_of(2);
    let pulse_col = if is_highlighted {
        theme.background
    } else if pulse_on {
        status_color
    } else {
        theme.grid_dim
    };
    fill_rect(&mut img, w - 10, 6, 4, 4, pulse_col);

    draw_hline(&mut img, 5, 15, w - 10, theme.grid_dim);
    draw_text_5x7(&mut img, 6, 20, "OPS CTR", 1, theme.phosphor_dim);

    let (state_txt, state_col) = if telemetry.demo_mode_active {
        ("SIM LOAD", theme.amber)
    } else if max_metric >= theme.crit_threshold {
        ("ALERT!", theme.red)
    } else if max_metric >= theme.warn_threshold {
        ("ATTN", theme.amber)
    } else {
        ("ONLINE", theme.phosphor_green)
    };

    fill_rect(&mut img, 6, 33, w - 12, 13, scale_color(state_col, 0.14));
    draw_hline(&mut img, 6, 33, w - 12, scale_color(state_col, 0.5));
    draw_hline(&mut img, 6, 45, w - 12, scale_color(state_col, 0.5));
    draw_text_5x7(&mut img, 9, 36, state_txt, 1, state_col);

    let cores_txt = format!("{}C/{}G", telemetry.core_usages.len(), telemetry.gpu_cores);
    draw_text_3x5(&mut img, 6, h - 16, &cores_txt, 1, theme.phosphor_dim);
    draw_text_3x5(&mut img, 6, h - 9, "KEY:TEST", 1, theme.phosphor_dim);

    img
}
