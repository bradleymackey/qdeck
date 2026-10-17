use crate::font::{draw_hline, draw_rect_border, draw_triangle_down, fill_rect, put_px};
use image::{Rgb, RgbImage};

/// Draws an upward-pointing triangle (`▲`) centered at `cx` with apex at `top_y` and height `half_w + 1`.
pub fn draw_triangle_up(img: &mut RgbImage, cx: i32, top_y: i32, half_w: i32, color: Rgb<u8>) {
    for row in 0..=half_w {
        draw_hline(img, cx - row, top_y + row, row * 2 + 1, color);
    }
}

/// Draws a bold upward arrow (`↑`) centered horizontally at `cx`, starting at `top_y` with total height `height`.
pub fn draw_arrow_up(
    img: &mut RgbImage,
    cx: i32,
    top_y: i32,
    height: i32,
    head_half_w: i32,
    color: Rgb<u8>,
) {
    let head_h = head_half_w.max(2);
    draw_triangle_up(img, cx, top_y, head_h, color);
    let stem_top = top_y + head_h + 1;
    let stem_h = (height - head_h - 1).max(2);
    fill_rect(img, cx - 1, stem_top, 3, stem_h, color);
}

/// Draws a bold downward arrow (`↓`) centered horizontally at `cx`, starting at `top_y` with total height `height`.
pub fn draw_arrow_down(
    img: &mut RgbImage,
    cx: i32,
    top_y: i32,
    height: i32,
    head_half_w: i32,
    color: Rgb<u8>,
) {
    let head_h = head_half_w.max(2);
    let stem_h = (height - head_h - 1).max(2);
    fill_rect(img, cx - 1, top_y, 3, stem_h, color);
    draw_triangle_down(img, cx, top_y + stem_h, head_h, color);
}

/// Draws a 4-notch Airbus flap position progress bar (`1`, `2`, `3`, `FULL`).
pub fn draw_flap_notches(
    img: &mut RgbImage,
    x: i32,
    y: i32,
    notch: u8,
    active_col: Rgb<u8>,
    track_col: Rgb<u8>,
) {
    let seg_w = 8;
    let seg_h = 4;
    let gap = 2;
    for i in 0..4u8 {
        let sx = x + (i as i32) * (seg_w + gap);
        let col = if i < notch { active_col } else { track_col };
        fill_rect(img, sx, y, seg_w, seg_h, col);
    }
}

/// Draws a wing cross-section with a deployed speedbrake/spoiler panel angled upward according to `ratio`.
pub fn draw_speedbrake_icon(
    img: &mut RgbImage,
    x: i32,
    y: i32,
    ratio: f32,
    active_col: Rgb<u8>,
    dim_col: Rgb<u8>,
) {
    // Horizontal wing chord line
    fill_rect(img, x, y + 8, 16, 3, dim_col);
    // Hinged spoiler flap rising from (x + 5, y + 7)
    let clamped = ratio.clamp(0.0, 1.0);
    let rise = (clamped * 7.0).round() as i32;
    for step in 0..8 {
        let px = x + 4 + step;
        let py = y + 7 - ((step * rise) / 7);
        put_px(img, px, py, active_col);
        put_px(img, px, py - 1, active_col);
    }
}

/// Draws a brake rotor & caliper `((O))` icon centered at `(cx, cy)`.
pub fn draw_brake_icon(img: &mut RgbImage, cx: i32, cy: i32, color: Rgb<u8>) {
    // Inner disc
    draw_rect_border(img, cx - 4, cy - 4, 9, 9, 1, color);
    fill_rect(img, cx - 1, cy - 1, 3, 3, color);
    // Left & right brake shoes `(  )`
    fill_rect(img, cx - 7, cy - 3, 2, 7, color);
    fill_rect(img, cx + 6, cy - 3, 2, 7, color);
}

/// Draws a 4-blade impeller/fan icon centered at `(cx, cy)`.
pub fn draw_fan_icon(img: &mut RgbImage, cx: i32, cy: i32, color: Rgb<u8>) {
    // Center hub
    fill_rect(img, cx - 1, cy - 1, 3, 3, color);
    // Top, bottom, left, right blades
    fill_rect(img, cx - 1, cy - 5, 3, 3, color);
    fill_rect(img, cx - 1, cy + 3, 3, 3, color);
    fill_rect(img, cx - 5, cy - 1, 3, 3, color);
    fill_rect(img, cx + 3, cy - 1, 3, 3, color);
}

/// Draws a 3-bar deceleration meter (`level` = 1 for LO, 2 for MED, 3 for MAX).
pub fn draw_autobrake_meter(
    img: &mut RgbImage,
    x: i32,
    y: i32,
    level: u8,
    active_col: Rgb<u8>,
    dim_col: Rgb<u8>,
) {
    for i in 0..3u8 {
        let bx = x + (i as i32) * 5;
        let bh = 4 + (i as i32) * 3;
        let by = y + (10 - bh);
        let col = if i < level { active_col } else { dim_col };
        fill_rect(img, bx, by, 3, bh, col);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_icons_render_non_empty() {
        let mut img = RgbImage::from_pixel(72, 72, Rgb([0, 0, 0]));
        let green = Rgb([0, 255, 65]);
        let dim = Rgb([0, 90, 25]);
        draw_arrow_up(&mut img, 15, 10, 12, 4, green);
        draw_arrow_down(&mut img, 35, 10, 12, 4, green);
        draw_flap_notches(&mut img, 10, 30, 2, green, dim);
        draw_speedbrake_icon(&mut img, 10, 40, 0.75, green, dim);
        draw_brake_icon(&mut img, 50, 40, green);
        draw_fan_icon(&mut img, 50, 55, green);
        draw_autobrake_meter(&mut img, 10, 55, 2, green, dim);

        let lit = img.pixels().filter(|p| p.0 != [0, 0, 0]).count();
        assert!(lit > 80);
    }
}
