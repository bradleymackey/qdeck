pub mod glyphs;
pub mod primitives;

pub use glyphs::{glyph_3x5, glyph_5x7};
pub use primitives::{
    draw_corner_brackets, draw_hline, draw_rect_border, draw_triangle_down, fill_rect, put_px,
    scale_color,
};

use image::{Rgb, RgbImage};

/// Measure width in pixels of a string rendered with the 5x7 pixel font at `scale`.
pub fn text_width_5x7(text: &str, scale: i32) -> i32 {
    let count = text.chars().count() as i32;
    if count == 0 {
        0
    } else {
        count * (5 * scale) + (count - 1) * scale
    }
}

/// Draw crisp pixel text using a classic 5x7 ops-terminal bitmap font.
pub fn draw_text_5x7(img: &mut RgbImage, x: i32, y: i32, text: &str, scale: i32, color: Rgb<u8>) {
    let scale = scale.max(1);
    let mut cursor_x = x;
    for ch in text.chars() {
        let glyph = glyph_5x7(ch.to_ascii_uppercase());
        for (row_idx, row_bits) in glyph.iter().enumerate() {
            for col_idx in 0..5 {
                if (row_bits >> (4 - col_idx)) & 1 == 1 {
                    fill_rect(
                        img,
                        cursor_x + col_idx * scale,
                        y + (row_idx as i32) * scale,
                        scale,
                        scale,
                        color,
                    );
                }
            }
        }
        cursor_x += 6 * scale;
    }
}

/// Draw compact 3x5 micro pixel text (ideal for dense telemetry labels/numbers).
pub fn draw_text_3x5(img: &mut RgbImage, x: i32, y: i32, text: &str, scale: i32, color: Rgb<u8>) {
    let scale = scale.max(1);
    let mut cursor_x = x;
    for ch in text.chars() {
        let glyph = glyph_3x5(ch.to_ascii_uppercase());
        for (row_idx, row_bits) in glyph.iter().enumerate() {
            for col_idx in 0..3 {
                if (row_bits >> (2 - col_idx)) & 1 == 1 {
                    fill_rect(
                        img,
                        cursor_x + col_idx * scale,
                        y + (row_idx as i32) * scale,
                        scale,
                        scale,
                        color,
                    );
                }
            }
        }
        cursor_x += 4 * scale;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_width_5x7() {
        assert_eq!(text_width_5x7("", 1), 0);
        assert_eq!(text_width_5x7("A", 1), 5);
        assert_eq!(text_width_5x7("AB", 1), 11);
        assert_eq!(text_width_5x7("CPU", 1), 17);
        assert_eq!(text_width_5x7("CPU", 2), 34);
    }

    #[test]
    fn test_scale_color() {
        let green = Rgb([0, 255, 65]);
        assert_eq!(scale_color(green, 0.0), Rgb([0, 0, 0]));
        assert_eq!(scale_color(green, 1.0), green);
        assert_eq!(scale_color(green, 2.0), green);
        let half = scale_color(green, 0.5);
        assert_eq!(half, Rgb([0, 128, 33]));
    }

    #[test]
    fn test_draw_primitives_out_of_bounds_safe() {
        let mut img = RgbImage::from_pixel(16, 16, Rgb([0, 0, 0]));
        let c = Rgb([0, 255, 65]);
        put_px(&mut img, -5, -5, c);
        put_px(&mut img, 100, 100, c);
        fill_rect(&mut img, -2, -2, 8, 8, c);
        draw_rect_border(&mut img, 0, 0, 16, 16, 1, c);
        draw_triangle_down(&mut img, 8, 4, 3, c);
        draw_text_5x7(&mut img, 1, 1, "A320+", 1, c);
        draw_text_3x5(&mut img, 1, 9, ">FLT", 1, c);
        assert_eq!(*img.get_pixel(0, 0), c);
        assert_eq!(*img.get_pixel(8, 4), c);
    }
}
