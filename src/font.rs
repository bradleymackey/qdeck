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
pub fn draw_text_5x7(
    img: &mut RgbImage,
    x: i32,
    y: i32,
    text: &str,
    scale: i32,
    color: Rgb<u8>,
) {
    let scale = scale.max(1);
    let mut cursor_x = x;
    for ch in text.chars() {
        let glyph = glyph_5x7(ch.to_ascii_uppercase());
        for (row_idx, row_bits) in glyph.iter().enumerate() {
            for col_idx in 0..5 {
                if (row_bits >> (4 - col_idx)) & 1 == 1 {
                    fill_rect(
                        img,
                        cursor_x + (col_idx as i32) * scale,
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
pub fn draw_text_3x5(
    img: &mut RgbImage,
    x: i32,
    y: i32,
    text: &str,
    scale: i32,
    color: Rgb<u8>,
) {
    let scale = scale.max(1);
    let mut cursor_x = x;
    for ch in text.chars() {
        let glyph = glyph_3x5(ch.to_ascii_uppercase());
        for (row_idx, row_bits) in glyph.iter().enumerate() {
            for col_idx in 0..3 {
                if (row_bits >> (2 - col_idx)) & 1 == 1 {
                    fill_rect(
                        img,
                        cursor_x + (col_idx as i32) * scale,
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

fn glyph_5x7(c: char) -> [u8; 7] {
    match c {
        '0' => [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
        '1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        '2' => [0b01110, 0b10001, 0b00001, 0b00110, 0b01000, 0b10000, 0b11111],
        '3' => [0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110],
        '4' => [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
        '5' => [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110],
        '6' => [0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
        '7' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
        '8' => [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
        '9' => [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100],
        'A' => [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'B' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110],
        'C' => [0b01111, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b01111],
        'D' => [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110],
        'E' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
        'F' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000],
        'G' => [0b01111, 0b10000, 0b10000, 0b10011, 0b10001, 0b10001, 0b01111],
        'H' => [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'I' => [0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        'J' => [0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100],
        'K' => [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001],
        'L' => [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111],
        'M' => [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001],
        'N' => [0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001],
        'O' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'P' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000],
        'Q' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101],
        'R' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
        'S' => [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110],
        'T' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
        'U' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'V' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
        'W' => [0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001],
        'X' => [0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001],
        'Y' => [0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100],
        'Z' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111],
        '%' => [0b11001, 0b11010, 0b00010, 0b00100, 0b01000, 0b01011, 0b10011],
        ':' => [0b00000, 0b00100, 0b00100, 0b00000, 0b00100, 0b00100, 0b00000],
        '.' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b01100],
        '-' => [0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000],
        '/' => [0b00001, 0b00010, 0b00010, 0b00100, 0b01000, 0b01000, 0b10000],
        '!' => [0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00000, 0b00100],
        '[' => [0b01110, 0b01000, 0b01000, 0b01000, 0b01000, 0b01000, 0b01110],
        ']' => [0b01110, 0b00010, 0b00010, 0b00010, 0b00010, 0b00010, 0b01110],
        _ => [0b00000; 7],
    }
}

fn glyph_3x5(c: char) -> [u8; 5] {
    match c {
        '0' => [0b111, 0b101, 0b101, 0b101, 0b111],
        '1' => [0b010, 0b110, 0b010, 0b010, 0b111],
        '2' => [0b111, 0b001, 0b111, 0b100, 0b111],
        '3' => [0b111, 0b001, 0b111, 0b001, 0b111],
        '4' => [0b101, 0b101, 0b111, 0b001, 0b001],
        '5' => [0b111, 0b100, 0b111, 0b001, 0b111],
        '6' => [0b111, 0b100, 0b111, 0b101, 0b111],
        '7' => [0b111, 0b001, 0b010, 0b010, 0b010],
        '8' => [0b111, 0b101, 0b111, 0b101, 0b111],
        '9' => [0b111, 0b101, 0b111, 0b001, 0b111],
        'A' => [0b010, 0b101, 0b111, 0b101, 0b101],
        'B' => [0b110, 0b101, 0b110, 0b101, 0b110],
        'C' => [0b111, 0b100, 0b100, 0b100, 0b111],
        'D' => [0b110, 0b101, 0b101, 0b101, 0b110],
        'E' => [0b111, 0b100, 0b110, 0b100, 0b111],
        'F' => [0b111, 0b100, 0b110, 0b100, 0b100],
        'G' => [0b111, 0b100, 0b101, 0b101, 0b111],
        'H' => [0b101, 0b101, 0b111, 0b101, 0b101],
        'I' => [0b111, 0b010, 0b010, 0b010, 0b111],
        'J' => [0b001, 0b001, 0b001, 0b101, 0b111],
        'K' => [0b101, 0b101, 0b110, 0b101, 0b101],
        'L' => [0b100, 0b100, 0b100, 0b100, 0b111],
        'M' => [0b101, 0b111, 0b101, 0b101, 0b101],
        'N' => [0b111, 0b101, 0b101, 0b101, 0b101],
        'O' => [0b111, 0b101, 0b101, 0b101, 0b111],
        'P' => [0b111, 0b101, 0b111, 0b100, 0b100],
        'Q' => [0b111, 0b101, 0b101, 0b111, 0b001],
        'R' => [0b111, 0b101, 0b110, 0b101, 0b101],
        'S' => [0b111, 0b100, 0b111, 0b001, 0b111],
        'T' => [0b111, 0b010, 0b010, 0b010, 0b010],
        'U' => [0b101, 0b101, 0b101, 0b101, 0b111],
        'V' => [0b101, 0b101, 0b101, 0b101, 0b010],
        'W' => [0b101, 0b101, 0b101, 0b111, 0b101],
        'X' => [0b101, 0b101, 0b010, 0b101, 0b101],
        'Y' => [0b101, 0b101, 0b010, 0b010, 0b010],
        'Z' => [0b111, 0b001, 0b010, 0b100, 0b111],
        '%' => [0b101, 0b001, 0b010, 0b100, 0b101],
        ':' => [0b000, 0b010, 0b000, 0b010, 0b000],
        '.' => [0b000, 0b000, 0b000, 0b000, 0b010],
        '-' => [0b000, 0b000, 0b111, 0b000, 0b000],
        '/' => [0b001, 0b001, 0b010, 0b100, 0b100],
        '>' => [0b100, 0b010, 0b001, 0b010, 0b100],
        _ => [0b000; 5],
    }
}
