use image::{Rgb, RgbImage};

/// Put a single pixel safely within image bounds.
#[inline]
pub fn put_px(img: &mut RgbImage, x: i32, y: i32, color: Rgb<u8>) {
    if x >= 0 && y >= 0 && (x as u32) < img.width() && (y as u32) < img.height() {
        img.put_pixel(x as u32, y as u32, color);
    }
}

/// Fill an axis-aligned rectangle `[x..x+w, y..y+h)`.
pub fn fill_rect(img: &mut RgbImage, x: i32, y: i32, w: i32, h: i32, color: Rgb<u8>) {
    for py in y..(y + h) {
        for px in x..(x + w) {
            put_px(img, px, py, color);
        }
    }
}

/// Draw a horizontal line `[x..x+w]` at `y`.
pub fn draw_hline(img: &mut RgbImage, x: i32, y: i32, w: i32, color: Rgb<u8>) {
    for px in x..(x + w) {
        put_px(img, px, y, color);
    }
}

/// Draw a downward-pointing filled triangle (used for Airbus landing gear LOCKED DOWN indicators).
/// `cx` is the horizontal center, `top_y` is the top edge, and `half_w` is the half-width at the top.
pub fn draw_triangle_down(img: &mut RgbImage, cx: i32, top_y: i32, half_w: i32, color: Rgb<u8>) {
    for row in 0..=half_w {
        let span = half_w - row;
        draw_hline(img, cx - span, top_y + row, span * 2 + 1, color);
    }
}

/// Draw tactical ops-center corner brackets around a rectangle.
pub fn draw_corner_brackets(
    img: &mut RgbImage,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    len: i32,
    color: Rgb<u8>,
) {
    let x2 = x + w - 1;
    let y2 = y + h - 1;

    // Top-left
    draw_hline(img, x, y, len, color);
    for dy in 0..len {
        put_px(img, x, y + dy, color);
    }

    // Top-right
    draw_hline(img, x2 - len + 1, y, len, color);
    for dy in 0..len {
        put_px(img, x2, y + dy, color);
    }

    // Bottom-left
    draw_hline(img, x, y2, len, color);
    for dy in 0..len {
        put_px(img, x, y2 - dy, color);
    }

    // Bottom-right
    draw_hline(img, x2 - len + 1, y2, len, color);
    for dy in 0..len {
        put_px(img, x2, y2 - dy, color);
    }
}

/// Draw a full rectangular border of `thickness` pixels.
pub fn draw_rect_border(
    img: &mut RgbImage,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    thickness: i32,
    color: Rgb<u8>,
) {
    for t in 0..thickness {
        draw_hline(img, x + t, y + t, w - 2 * t, color);
        draw_hline(img, x + t, y + h - 1 - t, w - 2 * t, color);
        for py in (y + t)..(y + h - t) {
            put_px(img, x + t, py, color);
            put_px(img, x + w - 1 - t, py, color);
        }
    }
}

/// Scale an RGB color by a brightness factor (0.0 ..= 1.0).
pub fn scale_color(c: Rgb<u8>, factor: f32) -> Rgb<u8> {
    let f = factor.clamp(0.0, 1.0);
    Rgb([
        (c.0[0] as f32 * f).round() as u8,
        (c.0[1] as f32 * f).round() as u8,
        (c.0[2] as f32 * f).round() as u8,
    ])
}
