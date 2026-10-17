use super::TelemetrySnapshot;
use crate::config::{ResolvedTheme, TileConfig};
use crate::font::{
    draw_corner_brackets, draw_hline, draw_rect_border, draw_text_3x5, draw_text_5x7, fill_rect,
    put_px, scale_color, text_width_5x7,
};
use image::RgbImage;

/// Renders the top-left Mode Selector tile showing the active mode (e.g. DASHBOARD / XPLANE).
pub fn render_mode_tile(
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

    if is_highlighted {
        draw_rect_border(&mut img, 0, 0, w, h, 2, theme.phosphor_green);
    }

    let label = tile.label.as_deref().unwrap_or("MODE");
    draw_text_5x7(&mut img, 5, 5, label, 1, theme.phosphor_dim);

    let idx_text = format!(
        "{}/{}",
        telemetry.mode_index + 1,
        telemetry.total_modes.max(1)
    );
    let idx_w = (idx_text.len() as i32) * 4 - 1;
    draw_text_3x5(&mut img, w - 5 - idx_w, 6, &idx_text, 1, theme.phosphor_dim);

    let mode_upper = telemetry.active_mode.to_ascii_uppercase();
    let box_y = 21;
    let box_h = 23;
    fill_rect(
        &mut img,
        4,
        box_y,
        w - 8,
        box_h,
        scale_color(theme.phosphor_green, 0.12),
    );
    draw_rect_border(&mut img, 4, box_y, w - 8, box_h, 1, theme.phosphor_green);

    if mode_upper.len() <= 10 {
        let tw = text_width_5x7(&mode_upper, 1);
        let tx = ((w - tw) / 2).max(6);
        draw_text_5x7(
            &mut img,
            tx,
            box_y + 8,
            &mode_upper,
            1,
            theme.phosphor_green,
        );
    } else {
        let clipped: String = mode_upper.chars().take(14).collect();
        let tw = (clipped.len() as i32) * 4 - 1;
        let tx = ((w - tw) / 2).max(6);
        draw_text_3x5(&mut img, tx, box_y + 9, &clipped, 1, theme.phosphor_green);
    }

    let n_modes = telemetry.total_modes.clamp(1, 8) as i32;
    let pip_w = 6;
    let pip_gap = 3;
    let pips_total_w = n_modes * pip_w + (n_modes - 1) * pip_gap;
    let pip_start_x = (w - pips_total_w) / 2;
    for m in 0..n_modes {
        let px = pip_start_x + m * (pip_w + pip_gap);
        let col = if (m as usize) == telemetry.mode_index {
            theme.phosphor_green
        } else {
            scale_color(theme.phosphor_dim, 0.30)
        };
        fill_rect(&mut img, px, 50, pip_w, 3, col);
    }

    let next_upper: String = telemetry
        .next_mode
        .to_ascii_uppercase()
        .chars()
        .take(11)
        .collect();
    let next_str = format!(">{}", next_upper);
    let nw = (next_str.len() as i32) * 4 - 1;
    draw_text_3x5(
        &mut img,
        ((w - nw) / 2).max(5),
        h - 10,
        &next_str,
        1,
        theme.phosphor_dim,
    );

    img
}

/// Renders the secondary Panel Selector tile (cycling through FCU, APU, FLT, SYS).
pub fn render_panel_tile(
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
        scale_color(theme.amber, 0.22)
    } else {
        theme.background
    };
    let mut img = RgbImage::from_pixel(width, height, bg);

    if is_highlighted {
        draw_rect_border(&mut img, 0, 0, w, h, 2, theme.amber);
    }

    let label = tile.label.as_deref().unwrap_or("PANEL");
    draw_text_5x7(&mut img, 5, 5, label, 1, theme.phosphor_dim);

    let idx_text = format!(
        "{}/{}",
        telemetry.panel_index + 1,
        telemetry.total_panels.max(1)
    );
    let idx_w = (idx_text.len() as i32) * 4 - 1;
    draw_text_3x5(&mut img, w - 5 - idx_w, 6, &idx_text, 1, theme.phosphor_dim);

    let panel_upper = telemetry.active_panel.to_ascii_uppercase();
    let accent = if matches!(panel_upper.as_str(), "APU" | "SYS") {
        theme.amber
    } else {
        theme.phosphor_green
    };

    let box_y = 21;
    let box_h = 23;
    fill_rect(&mut img, 4, box_y, w - 8, box_h, scale_color(accent, 0.12));
    draw_rect_border(&mut img, 4, box_y, w - 8, box_h, 1, accent);

    let tw = text_width_5x7(&panel_upper, 1);
    let tx = ((w - tw) / 2).max(6);
    draw_text_5x7(&mut img, tx, box_y + 8, &panel_upper, 1, accent);

    let n_panels = telemetry.total_panels.clamp(1, 8) as i32;
    let pip_w = 8;
    let pip_gap = 4;
    let pips_total_w = n_panels * pip_w + (n_panels - 1) * pip_gap;
    let pip_start_x = (w - pips_total_w) / 2;
    for p in 0..n_panels {
        let px = pip_start_x + p * (pip_w + pip_gap);
        let col = if (p as usize) == telemetry.panel_index {
            accent
        } else {
            scale_color(theme.phosphor_dim, 0.30)
        };
        fill_rect(&mut img, px, 50, pip_w, 3, col);
    }

    let next_upper: String = telemetry
        .next_panel
        .to_ascii_uppercase()
        .chars()
        .take(11)
        .collect();
    let next_str = format!(">{}", next_upper);
    let nw = (next_str.len() as i32) * 4 - 1;
    draw_text_3x5(
        &mut img,
        ((w - nw) / 2).max(5),
        h - 10,
        &next_str,
        1,
        theme.phosphor_dim,
    );

    img
}

/// Renders a custom launcher/command tile (`type = "launcher"`).
pub fn render_launcher_tile(
    width: u32,
    height: u32,
    tile: &TileConfig,
    theme: &ResolvedTheme,
    is_highlighted: bool,
) -> RgbImage {
    let w = width as i32;
    let h = height as i32;
    let bg = if is_highlighted {
        scale_color(theme.phosphor_green, 0.25)
    } else {
        theme.background
    };
    let mut img = RgbImage::from_pixel(width, height, bg);

    if is_highlighted {
        draw_rect_border(&mut img, 0, 0, w, h, 2, theme.phosphor_green);
        fill_rect(&mut img, 2, 2, w - 4, 11, theme.phosphor_green);
        draw_text_3x5(&mut img, 6, 5, "ACTIVE", 1, theme.background);
    } else {
        draw_corner_brackets(&mut img, 1, 1, w - 2, h - 2, 6, theme.phosphor_green);
        draw_text_3x5(&mut img, 6, 6, "EXEC", 1, theme.phosphor_dim);
    }
    draw_hline(&mut img, 5, 14, w - 10, theme.grid_dim);

    let label = tile.label.as_deref().unwrap_or("CMD");
    let lw = text_width_5x7(label, 1);
    let lx = ((w - lw) / 2).max(4);
    draw_text_5x7(&mut img, lx, (h / 2) - 3, label, 1, theme.phosphor_green);

    draw_text_3x5(
        &mut img,
        6,
        h - 9,
        if is_highlighted { "RUNNING" } else { "READY" },
        1,
        if is_highlighted {
            theme.phosphor_green
        } else {
            theme.phosphor_dim
        },
    );
    img
}

/// Renders a subtle tactical ops-center standby cell for unconfigured keys.
pub fn render_standby_tile(
    width: u32,
    height: u32,
    key_index: u8,
    theme: &ResolvedTheme,
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
    } else {
        draw_corner_brackets(&mut img, 2, 2, w - 4, h - 4, 4, theme.grid_dim);
    }

    let cx = w / 2;
    let cy = h / 2;
    let reticle_col = if is_highlighted {
        theme.phosphor_green
    } else {
        theme.grid_dim
    };
    put_px(
        &mut img,
        cx,
        cy,
        if is_highlighted {
            theme.phosphor_green
        } else {
            scale_color(theme.phosphor_dim, 0.35)
        },
    );
    put_px(&mut img, cx - 2, cy, reticle_col);
    put_px(&mut img, cx + 2, cy, reticle_col);
    put_px(&mut img, cx, cy - 2, reticle_col);
    put_px(&mut img, cx, cy + 2, reticle_col);

    let slot_txt = format!("{:02}", key_index + 1);
    draw_text_3x5(
        &mut img,
        6,
        6,
        &slot_txt,
        1,
        if is_highlighted {
            theme.phosphor_green
        } else {
            scale_color(theme.phosphor_dim, 0.45)
        },
    );

    img
}
