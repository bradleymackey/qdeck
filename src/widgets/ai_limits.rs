use super::TelemetrySnapshot;
use crate::config::{ResolvedTheme, TileConfig};
use crate::font::{
    draw_hline, draw_rect_border, draw_text_5x7, fill_rect, scale_color, text_width_5x7,
};
use crate::telemetry::AiProviderLimits;
use image::RgbImage;

/// Renders a Claude usage limits tile showing both 5-hour Session (`SES`) and 7-day Weekly (`WK`) utilization.
pub fn render_claude_limits_tile(
    width: u32,
    height: u32,
    tile: &TileConfig,
    theme: &ResolvedTheme,
    telemetry: &TelemetrySnapshot,
    is_highlighted: bool,
) -> RgbImage {
    render_dual_limit_tile(
        width,
        height,
        tile,
        theme,
        "CLAUDE",
        telemetry.claude_limits,
        telemetry.tick,
        telemetry.demo_mode_active,
        is_highlighted,
    )
}

/// Renders a Google Gemini usage limits tile showing both Session (`SES`) and Weekly (`WK`) utilization.
pub fn render_gemini_limits_tile(
    width: u32,
    height: u32,
    tile: &TileConfig,
    theme: &ResolvedTheme,
    telemetry: &TelemetrySnapshot,
    is_highlighted: bool,
) -> RgbImage {
    render_dual_limit_tile(
        width,
        height,
        tile,
        theme,
        "GEMINI",
        telemetry.gemini_limits,
        telemetry.tick,
        telemetry.demo_mode_active,
        is_highlighted,
    )
}

#[allow(clippy::too_many_arguments)]
fn render_dual_limit_tile(
    width: u32,
    height: u32,
    tile: &TileConfig,
    theme: &ResolvedTheme,
    default_label: &str,
    limits: AiProviderLimits,
    tick: u64,
    demo_mode_active: bool,
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

    let (ses_pct, wk_pct, available) = if demo_mode_active {
        let phase = (tick as f32) * 0.65;
        let sim_ses = (20.0 + ((phase * 0.9).sin() * 0.5 + 0.5) * 75.0).clamp(5.0, 96.0);
        let sim_wk = (30.0 + ((phase * 0.6 + 1.2).sin() * 0.5 + 0.5) * 65.0).clamp(10.0, 95.0);
        (sim_ses, sim_wk, true)
    } else {
        (
            limits.session_pct.clamp(0.0, 100.0),
            limits.weekly_pct.clamp(0.0, 100.0),
            limits.available,
        )
    };

    let peak_pct = ses_pct.max(wk_pct);
    let header_color = if available {
        theme.color_for_pct(peak_pct, Some(warn), Some(crit))
    } else {
        theme.phosphor_dim
    };

    let label = tile.label.as_deref().unwrap_or(default_label);
    draw_text_5x7(&mut img, 5, 5, label, 1, header_color);

    // Top-right live status indicator dot
    let dot_col = if available {
        header_color
    } else {
        theme.grid_dim
    };
    fill_rect(&mut img, w - 9, 7, 4, 4, dot_col);
    draw_hline(&mut img, 5, 14, w - 10, theme.grid_dim);

    // Row 1: Session limit ("SES")
    draw_limit_row(
        &mut img, w, 18, 28, "SES", ses_pct, available, warn, crit, theme,
    );

    // Row 2: Weekly limit ("WK")
    draw_limit_row(
        &mut img, w, 44, 54, "WK", wk_pct, available, warn, crit, theme,
    );

    img
}

#[allow(clippy::too_many_arguments)]
fn draw_limit_row(
    img: &mut RgbImage,
    w: i32,
    text_y: i32,
    bar_y: i32,
    tag: &str,
    pct: f32,
    available: bool,
    warn: f32,
    crit: f32,
    theme: &ResolvedTheme,
) {
    draw_text_5x7(img, 5, text_y, tag, 1, theme.phosphor_dim);

    let row_col = if available {
        theme.color_for_pct(pct, Some(warn), Some(crit))
    } else {
        theme.phosphor_dim
    };

    let pct_str = if available {
        format!("{}%", pct.round().clamp(0.0, 100.0) as u32)
    } else {
        "--%".to_string()
    };
    let pw = text_width_5x7(&pct_str, 1);
    draw_text_5x7(img, w - 5 - pw, text_y, &pct_str, 1, row_col);

    // 15-segment tactical progress bar (15 * 3px + 14 * 1px = 59px wide, centered at x=6)
    let segs = 15;
    let seg_w = 3;
    let seg_gap = 1;
    let bar_h = 9;
    let start_x = 6;
    let unlit_col = scale_color(theme.phosphor_dim, 0.18);

    let active_segs = if !available || pct < 0.5 {
        0
    } else {
        (((pct / 100.0) * (segs as f32)).round() as i32).clamp(1, segs)
    };

    for s in 0..segs {
        let sx = start_x + s * (seg_w + seg_gap);
        let col = if s < active_segs { row_col } else { unlit_col };
        fill_rect(img, sx, bar_y, seg_w, bar_h, col);
    }
}
