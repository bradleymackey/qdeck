use super::TelemetrySnapshot;
use crate::config::{ResolvedTheme, TileConfig};
use crate::font::{
    draw_corner_brackets, draw_hline, draw_rect_border, draw_text_3x5, draw_text_5x7, fill_rect,
    scale_color, text_width_5x7,
};
use image::RgbImage;

/// Renders a clean, minimal CPU utilization tile with one solid bar per CPU core.
pub fn render_cpu_tile(
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
        scale_color(theme.phosphor_green, 0.20)
    } else {
        theme.background
    };
    let mut img = RgbImage::from_pixel(width, height, bg);

    if is_highlighted {
        draw_rect_border(&mut img, 0, 0, w, h, 2, theme.phosphor_green);
    }

    let warn = tile.warn_threshold.unwrap_or(theme.warn_threshold);
    let crit = tile.crit_threshold.unwrap_or(theme.crit_threshold);

    let header_color = theme.color_for_pct(telemetry.total_cpu_pct, Some(warn), Some(crit));
    let label = tile.label.as_deref().unwrap_or("CPU");
    draw_text_5x7(&mut img, 5, 5, label, 1, header_color);

    let pct_val = telemetry.total_cpu_pct.round().clamp(0.0, 100.0) as u32;
    let pct_text = format!("{}%", pct_val);
    let pct_w = text_width_5x7(&pct_text, 1);
    draw_text_5x7(&mut img, w - 5 - pct_w, 5, &pct_text, 1, header_color);

    // One solid vertical bar per CPU core
    let graph_x = 5;
    let graph_y = 16;
    let graph_w = w - 10;
    let graph_h = h - 21;
    let graph_bottom = graph_y + graph_h;

    let num_cores = telemetry.core_usages.len().max(1);
    let n = num_cores as i32;
    let gap = if n <= 16 { 1 } else { 0 };
    let total_gaps = (n - 1) * gap;
    let bar_w = ((graph_w - total_gaps) / n).max(1);
    let used_w = n * bar_w + total_gaps;
    let start_x = graph_x + (graph_w - used_w) / 2;

    for (idx, &core_pct) in telemetry.core_usages.iter().enumerate() {
        let bx = start_x + (idx as i32) * (bar_w + gap);
        let clamped = core_pct.clamp(0.0, 100.0);
        let core_color = theme.color_for_pct(clamped, Some(warn), Some(crit));

        let active_h = ((clamped / 100.0) * (graph_h as f32)).round() as i32;
        let active_h = active_h.clamp(1, graph_h);
        let top_y = graph_bottom - active_h;

        fill_rect(&mut img, bx, top_y, bar_w, active_h, core_color);
    }

    img
}

/// Computes `(cols, rows)` for a GPU core grid of `total_cores` cells.
pub fn gpu_grid_dimensions(total_cores: usize) -> (i32, i32) {
    match total_cores {
        40 | 38 => (8, 5),
        32 => (8, 4),
        30 => (6, 5),
        24 => (6, 4),
        20 => (5, 4),
        19 | 18 => (6, 3),
        16 => (4, 4),
        14 => (5, 3),
        10 => (5, 2),
        8 => (4, 2),
        n => {
            let cols = (n as f32).sqrt().ceil() as i32;
            let cols = cols.max(4);
            let rows = ((n as i32) + cols - 1) / cols;
            (cols, rows.max(1))
        }
    }
}

/// Renders a GPU utilization tile divided into a grid of GPU core cells, lighting them up
/// according to how many cores are currently under load.
pub fn render_gpu_tile(
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
        scale_color(theme.phosphor_green, 0.20)
    } else {
        theme.background
    };
    let mut img = RgbImage::from_pixel(width, height, bg);

    if is_highlighted {
        draw_rect_border(&mut img, 0, 0, w, h, 2, theme.phosphor_green);
    }

    let warn = tile.warn_threshold.unwrap_or(theme.warn_threshold);
    let crit = tile.crit_threshold.unwrap_or(theme.crit_threshold);

    let clamped_pct = telemetry.gpu_pct.clamp(0.0, 100.0);
    let active_color = theme.color_for_pct(clamped_pct, Some(warn), Some(crit));

    let label = tile.label.as_deref().unwrap_or("GPU");
    draw_text_5x7(&mut img, 5, 5, label, 1, active_color);

    let pct_val = clamped_pct.round() as u32;
    let pct_text = format!("{}%", pct_val);
    let pct_w = text_width_5x7(&pct_text, 1);
    draw_text_5x7(&mut img, w - 5 - pct_w, 5, &pct_text, 1, active_color);

    let total_cores = telemetry.gpu_cores.max(1);
    let (grid_cols, grid_rows) = gpu_grid_dimensions(total_cores);

    let area_x = 5;
    let area_y = 16;
    let area_w = w - 10;
    let area_h = h - 21;

    let gap_x = 2;
    let gap_y = 2;
    let cell_w = ((area_w - (grid_cols - 1) * gap_x) / grid_cols).max(2);
    let cell_h = ((area_h - (grid_rows - 1) * gap_y) / grid_rows).max(2);

    let used_w = grid_cols * cell_w + (grid_cols - 1) * gap_x;
    let used_h = grid_rows * cell_h + (grid_rows - 1) * gap_y;
    let start_x = area_x + (area_w - used_w) / 2;
    let start_y = area_y + (area_h - used_h) / 2;

    let active_cores = ((clamped_pct / 100.0) * (total_cores as f32)).round() as usize;
    let unlit_color = scale_color(theme.phosphor_dim, 0.18);

    for idx in 0..total_cores {
        let col = (idx as i32) % grid_cols;
        let row = (idx as i32) / grid_cols;
        let cx = start_x + col * (cell_w + gap_x);
        let cy = start_y + row * (cell_h + gap_y);

        let color = if idx < active_cores {
            active_color
        } else {
            unlit_color
        };

        fill_rect(&mut img, cx, cy, cell_w, cell_h, color);
    }

    img
}

/// Renders a Memory/Swap utilization tile (can be assigned to any key via `type = "memory"`).
pub fn render_memory_tile(
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

    let warn = tile.warn_threshold.unwrap_or(75.0);
    let crit = tile.crit_threshold.unwrap_or(90.0);
    let mem_color = theme.color_for_pct(telemetry.mem_used_pct, Some(warn), Some(crit));

    if is_highlighted {
        draw_rect_border(&mut img, 0, 0, w, h, 2, theme.phosphor_green);
        fill_rect(&mut img, 2, 2, w - 4, 11, theme.phosphor_green);
    } else {
        draw_corner_brackets(&mut img, 1, 1, w - 2, h - 2, 5, theme.phosphor_dim);
    }

    let fg_header = if is_highlighted {
        theme.background
    } else {
        mem_color
    };
    fill_rect(&mut img, 5, 6, 3, 3, fg_header);

    let label = tile.label.as_deref().unwrap_or("MEM");
    draw_text_5x7(&mut img, 10, 4, label, 1, fg_header);

    let pct_text = format!("{:.0}%", telemetry.mem_used_pct.clamp(0.0, 100.0));
    let pct_w = text_width_5x7(&pct_text, 1);
    draw_text_5x7(&mut img, w - 4 - pct_w, 4, &pct_text, 1, fg_header);

    draw_hline(&mut img, 4, 13, w - 8, theme.grid_dim);

    draw_text_3x5(&mut img, 6, 18, "RAM", 1, theme.phosphor_dim);
    let gb_str = format!(
        "{:.0}/{:.0}G",
        telemetry.mem_used_gb, telemetry.mem_total_gb
    );
    let gb_w = (gb_str.len() as i32) * 4 - 1;
    draw_text_3x5(&mut img, w - 6 - gb_w, 18, &gb_str, 1, mem_color);

    let bar_x = 6;
    let bar_w = w - 12;
    let segs = 15;
    let seg_w = 3;
    let seg_gap = 1;
    let active_segs = ((telemetry.mem_used_pct / 100.0) * (segs as f32)).round() as i32;
    for s in 0..segs {
        let sx = bar_x + s * (seg_w + seg_gap);
        if sx + seg_w <= bar_x + bar_w {
            let col = if s < active_segs {
                let seg_pct = ((s + 1) as f32 / segs as f32) * 100.0;
                theme.color_for_pct(seg_pct.min(telemetry.mem_used_pct), Some(warn), Some(crit))
            } else {
                theme.bar_track
            };
            fill_rect(&mut img, sx, 26, seg_w, 12, col);
        }
    }

    let swp_color = theme.color_for_pct(telemetry.swap_used_pct, Some(warn), Some(crit));
    draw_text_3x5(&mut img, 6, 43, "SWP", 1, theme.phosphor_dim);
    let swp_segs = ((telemetry.swap_used_pct / 100.0) * (segs as f32)).round() as i32;
    for s in 0..segs {
        let sx = bar_x + s * (seg_w + seg_gap);
        if sx + seg_w <= bar_x + bar_w {
            let col = if s < swp_segs {
                swp_color
            } else {
                theme.bar_track
            };
            fill_rect(&mut img, sx, 50, seg_w, 8, col);
        }
    }

    let footer_y = h - 7;
    draw_text_3x5(
        &mut img,
        5,
        footer_y,
        if is_highlighted { "OPENING" } else { "ACT:MON" },
        1,
        if is_highlighted {
            theme.phosphor_green
        } else {
            theme.phosphor_dim
        },
    );

    img
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
