use super::TelemetrySnapshot;
use crate::config::{ResolvedTheme, TileConfig};
use crate::font::{
    draw_hline, draw_rect_border, draw_text_3x5, draw_text_5x7, fill_rect, scale_color,
};
use crate::telemetry::AudioEndpoint;
use image::RgbImage;

/// Renders a tactical Audio I/O tile (`type = "audio"`) displaying the active macOS
/// default Output (`OUT`) and Input (`IN`) audio devices and their transport badges.
pub fn render_audio_tile(
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

    let audio = &telemetry.audio_devices;
    let available = audio.available;
    let header_color = if available {
        theme.phosphor_green
    } else {
        theme.phosphor_dim
    };

    let header_label = tile.label.as_deref().unwrap_or("AUDIO");
    draw_text_5x7(&mut img, 5, 5, header_label, 1, header_color);

    let dot_col = if available {
        header_color
    } else {
        theme.grid_dim
    };
    fill_rect(&mut img, w - 9, 7, 4, 4, dot_col);
    draw_hline(&mut img, 5, 14, w - 10, theme.grid_dim);

    // Row 1: Output Device ("OUT")
    draw_audio_endpoint_row(&mut img, w, 17, "OUT", &audio.output, available, theme);

    // Row 2: Input Device ("IN")
    draw_audio_endpoint_row(&mut img, w, 44, "IN", &audio.input, available, theme);

    img
}

fn draw_audio_endpoint_row(
    img: &mut RgbImage,
    w: i32,
    label_y: i32,
    direction_tag: &str,
    endpoint: &AudioEndpoint,
    available: bool,
    theme: &ResolvedTheme,
) {
    let box_y = label_y + 7;
    draw_text_3x5(img, 5, label_y, direction_tag, 1, theme.phosphor_dim);

    if available && !endpoint.transport.is_empty() {
        let bw = (endpoint.transport.len() as i32) * 4 - 1;
        draw_text_3x5(
            img,
            w - 5 - bw,
            label_y,
            &endpoint.transport,
            1,
            theme.phosphor_dim,
        );
    }

    let has_device = available && !endpoint.short_name.is_empty() && endpoint.short_name != "NONE";
    let row_col = if has_device {
        theme.phosphor_green
    } else {
        theme.phosphor_dim
    };

    let box_x = 4;
    let box_w = w - 8;
    let box_h = 15;

    fill_rect(
        img,
        box_x,
        box_y,
        box_w,
        box_h,
        scale_color(row_col, if has_device { 0.12 } else { 0.06 }),
    );
    fill_rect(img, box_x, box_y, 2, box_h, row_col);
    draw_hline(
        img,
        box_x,
        box_y,
        box_w,
        scale_color(theme.phosphor_dim, 0.45),
    );
    draw_hline(
        img,
        box_x,
        box_y + box_h - 1,
        box_w,
        scale_color(theme.phosphor_dim, 0.45),
    );

    let name_to_draw: String = if has_device {
        endpoint.short_name.chars().take(10).collect()
    } else {
        "NONE".to_string()
    };

    draw_text_5x7(img, 8, box_y + 4, &name_to_draw, 1, row_col);
}
