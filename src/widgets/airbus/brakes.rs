use crate::config::{ResolvedTheme, TileConfig};
use crate::font::{draw_hline, draw_rect_border, draw_text_5x7, scale_color, text_width_5x7};
use crate::widgets::icons::{draw_autobrake_meter, draw_brake_icon, draw_fan_icon};
use crate::xplane::A320FcuState;
use image::RgbImage;

/// Airbus Parking Brake (`park_brake`) with brake rotor/caliper icon `((O))`.
pub(super) fn render_park_brake_control(
    img: &mut RgbImage,
    w: i32,
    h: i32,
    tile: &TileConfig,
    fcu: &A320FcuState,
    theme: &ResolvedTheme,
) {
    let label = tile.label.as_deref().unwrap_or("PARK BRK");
    let col = if fcu.park_brake_on {
        theme.amber
    } else {
        theme.phosphor_green
    };
    let border_col = if fcu.park_brake_on {
        scale_color(theme.amber, 0.65)
    } else {
        theme.grid_dim
    };

    draw_rect_border(img, 4, 4, w - 8, h - 8, 1, border_col);
    draw_hline(img, 5, 33, w - 10, theme.grid_dim);

    draw_brake_icon(img, 20, 18, col);
    let txt = if fcu.park_brake_on { "PARK" } else { "FREE" };
    draw_text_5x7(img, 34, 15, txt, 1, col);

    let lw = text_width_5x7(label, 1);
    draw_text_5x7(
        img,
        ((w - lw) / 2).max(5),
        46,
        label,
        1,
        if fcu.park_brake_on {
            theme.amber
        } else {
            theme.phosphor_dim
        },
    );
}

/// Airbus Brake Fan (`brake_fan`) with 4-blade impeller icon.
pub(super) fn render_brake_fan_control(
    img: &mut RgbImage,
    w: i32,
    h: i32,
    tile: &TileConfig,
    fcu: &A320FcuState,
    theme: &ResolvedTheme,
) {
    let label = tile.label.as_deref().unwrap_or("BRK FAN");
    let col = if fcu.brake_fan_on {
        theme.phosphor_green
    } else {
        scale_color(theme.phosphor_dim, 0.55)
    };
    let border_col = if fcu.brake_fan_on {
        scale_color(theme.phosphor_green, 0.65)
    } else {
        theme.grid_dim
    };

    draw_rect_border(img, 4, 4, w - 8, h - 8, 1, border_col);
    draw_hline(img, 5, 33, w - 10, theme.grid_dim);

    draw_fan_icon(img, 21, 18, col);
    let txt = if fcu.brake_fan_on { "ON" } else { "OFF" };
    draw_text_5x7(img, 35, 15, txt, 1, col);

    let lw = text_width_5x7(label, 1);
    draw_text_5x7(
        img,
        ((w - lw) / 2).max(5),
        46,
        label,
        1,
        if fcu.brake_fan_on {
            theme.phosphor_green
        } else {
            theme.phosphor_dim
        },
    );
}

/// Airbus Autobrake (`abrk_lo`, `abrk_med`, `abrk_max`) with 3-bar deceleration meter.
pub(super) fn render_autobrake_control(
    img: &mut RgbImage,
    w: i32,
    h: i32,
    ctrl: &str,
    tile: &TileConfig,
    fcu: &A320FcuState,
    theme: &ResolvedTheme,
) {
    let (level, default_label, short_txt) = match ctrl {
        "abrk_lo" => (1u8, "A/BRK LO", "LO"),
        "abrk_med" => (2u8, "A/BRK MD", "MED"),
        _ => (3u8, "A/BRK MX", "MAX"),
    };
    let is_on = fcu.autobrake_mode == level;
    let accent = if level == 3 && is_on {
        theme.amber
    } else {
        theme.phosphor_green
    };
    let border_col = if is_on {
        scale_color(accent, 0.65)
    } else {
        theme.grid_dim
    };

    draw_rect_border(img, 4, 4, w - 8, h - 8, 1, border_col);
    draw_hline(img, 5, 33, w - 10, theme.grid_dim);

    let meter_col = if is_on {
        accent
    } else {
        scale_color(theme.phosphor_dim, 0.60)
    };
    draw_autobrake_meter(
        img,
        12,
        13,
        level,
        meter_col,
        scale_color(theme.phosphor_dim, 0.22),
    );

    let status_txt = if is_on { "ON" } else { short_txt };
    draw_text_5x7(img, 34, 15, status_txt, 1, meter_col);

    let label = tile.label.as_deref().unwrap_or(default_label);
    let lw = text_width_5x7(label, 1);
    draw_text_5x7(
        img,
        ((w - lw) / 2).max(5),
        46,
        label,
        1,
        if is_on { accent } else { theme.phosphor_dim },
    );
}
