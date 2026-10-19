use crate::config::ResolvedTheme;
use crate::font::{
    draw_hline, draw_rect_border, draw_text_3x5, draw_text_5x7, fill_rect, scale_color,
    text_width_5x7,
};
use image::{Rgb, RgbImage};

/// Declarative description of a standard tactical gauge tile:
///
/// ```text
/// HEADER      BADGE ■
/// ───────────────────
/// ROW_TAG         43%
/// ▮▮▮▮▮▮▯▯▯▯▯▯▯▯▯      (15-segment bar)
/// LEFT_TAG  RIGHT_TAG
/// ┃ LEFT  │   RIGHT ┃  (split readout box)
/// ```
pub struct GaugeTile<'a> {
    pub header: &'a str,
    pub badge: Option<&'a str>,
    /// Colour of the header, percentage, bar and status dot.
    pub accent: Rgb<u8>,
    /// Whether live telemetry is available (controls status dot and box intensity).
    pub active: bool,
    pub row_tag: &'a str,
    /// Utilization percentage (`None` renders `--%` with an unlit bar).
    pub pct: Option<f32>,
    pub left_tag: &'a str,
    pub right_tag: &'a str,
    pub left_value: &'a str,
    pub right_value: &'a str,
    /// Colour of the readout box tint and left edge marker.
    pub box_accent: Rgb<u8>,
    pub left_color: Rgb<u8>,
    pub right_color: Rgb<u8>,
}

/// Renders a [`GaugeTile`] onto a fresh `width × height` image.
pub fn render_gauge_tile(
    width: u32,
    height: u32,
    theme: &ResolvedTheme,
    is_highlighted: bool,
    spec: &GaugeTile,
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

    // Header: label, optional badge, status dot, divider
    draw_text_5x7(&mut img, 5, 5, spec.header, 1, spec.accent);
    if let Some(badge) = spec.badge {
        let bw = (badge.chars().count() as i32) * 4 - 1;
        draw_text_3x5(&mut img, w - 12 - bw, 6, badge, 1, theme.phosphor_dim);
    }
    let dot_col = if spec.active {
        spec.accent
    } else {
        theme.grid_dim
    };
    fill_rect(&mut img, w - 9, 7, 4, 4, dot_col);
    draw_hline(&mut img, 5, 14, w - 10, theme.grid_dim);

    // Row 1: tag + percentage
    draw_text_5x7(&mut img, 5, 18, spec.row_tag, 1, theme.phosphor_dim);
    let pct = spec.pct.map(|p| p.clamp(0.0, 100.0));
    let pct_str = match pct {
        Some(p) => format!("{}%", p.round() as u32),
        None => "--%".to_string(),
    };
    let pw = text_width_5x7(&pct_str, 1);
    draw_text_5x7(&mut img, w - 5 - pw, 18, &pct_str, 1, spec.accent);

    // 15-segment tactical progress bar (15 * 3px + 14 * 1px = 59px wide)
    let segs = 15;
    let seg_w = 3;
    let seg_gap = 1;
    let bar_h = 9;
    let start_x = 6;
    let unlit_col = scale_color(theme.phosphor_dim, 0.18);
    let active_segs = match pct {
        Some(p) if p >= 0.5 => (((p / 100.0) * (segs as f32)).round() as i32).clamp(1, segs),
        _ => 0,
    };
    for s in 0..segs {
        let sx = start_x + s * (seg_w + seg_gap);
        let col = if s < active_segs {
            spec.accent
        } else {
            unlit_col
        };
        fill_rect(&mut img, sx, 28, seg_w, bar_h, col);
    }

    // Row 2: split readout box
    draw_text_3x5(&mut img, 5, 43, spec.left_tag, 1, theme.phosphor_dim);
    let rw = (spec.right_tag.chars().count() as i32) * 4 - 1;
    draw_text_3x5(
        &mut img,
        w - 5 - rw,
        43,
        spec.right_tag,
        1,
        theme.phosphor_dim,
    );

    let box_x = 4;
    let box_y = 50;
    let box_w = w - 8;
    let box_h = 15;
    let edge_col = scale_color(theme.phosphor_dim, 0.45);

    fill_rect(
        &mut img,
        box_x,
        box_y,
        box_w,
        box_h,
        scale_color(spec.box_accent, if spec.active { 0.12 } else { 0.06 }),
    );
    fill_rect(&mut img, box_x, box_y, 2, box_h, spec.box_accent);
    draw_hline(&mut img, box_x, box_y, box_w, edge_col);
    draw_hline(&mut img, box_x, box_y + box_h - 1, box_w, edge_col);
    fill_rect(&mut img, w / 2, box_y + 3, 1, box_h - 6, theme.grid_dim);

    draw_text_5x7(&mut img, 8, box_y + 4, spec.left_value, 1, spec.left_color);
    let vw = text_width_5x7(spec.right_value, 1);
    draw_text_5x7(
        &mut img,
        w - 6 - vw,
        box_y + 4,
        spec.right_value,
        1,
        spec.right_color,
    );

    img
}

/// Formats a byte count in binary units into a compact 2..=4 character string
/// (e.g. `"0M"`, `"512M"`, `"1.8G"`, `"16G"`).
pub fn format_binary_bytes(bytes: u64) -> String {
    const MB: f64 = 1_048_576.0;
    const GB: f64 = 1_073_741_824.0;

    if bytes == 0 {
        return "0M".to_string();
    }
    let b = bytes as f64;
    if b >= GB * 0.95 {
        let gb = b / GB;
        if gb >= 9.95 {
            format!("{:.0}G", gb.min(999.0))
        } else {
            format!("{:.1}G", gb)
        }
    } else {
        let mb = (b / MB).round();
        format!("{:.0}M", mb.clamp(1.0, 999.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_binary_bytes() {
        assert_eq!(format_binary_bytes(0), "0M");
        assert_eq!(format_binary_bytes(536_870_912), "512M");
        assert_eq!(format_binary_bytes(1_939_865_600), "1.8G");
        assert_eq!(format_binary_bytes(3_221_225_472), "3.0G");
        assert_eq!(format_binary_bytes(16_684_777_472), "16G");
    }
}
