use crate::config::{ResolvedTheme, TileConfig};
use crate::font::{
    draw_hline, draw_rect_border, draw_text_3x5, draw_text_5x7, fill_rect, scale_color,
    text_width_5x7,
};
use crate::xplane::A320FcuState;
use image::RgbImage;

/// Airbus Overhead APU Start Korry Pushbutton (AVAIL / START + N1%).
pub(super) fn render_apu_start(
    img: &mut RgbImage,
    w: i32,
    h: i32,
    tile: &TileConfig,
    fcu: &A320FcuState,
    theme: &ResolvedTheme,
) {
    let label = tile.label.as_deref().unwrap_or("APU STR");
    let (top_str, top_col, border_col) = if fcu.apu_avail {
        (
            "AVAIL".to_string(),
            theme.phosphor_green,
            theme.phosphor_green,
        )
    } else if fcu.apu_start_on || fcu.apu_n1_pct > 1.0 {
        let s = if fcu.apu_n1_pct > 1.0 {
            format!("N {:>2.0}%", fcu.apu_n1_pct.clamp(0.0, 100.0))
        } else {
            "START".to_string()
        };
        (s, theme.amber, theme.amber)
    } else {
        (
            "OFF".to_string(),
            scale_color(theme.phosphor_dim, 0.45),
            theme.grid_dim,
        )
    };

    draw_rect_border(img, 4, 4, w - 8, h - 8, 1, border_col);
    draw_hline(img, 5, 33, w - 10, theme.grid_dim);

    if fcu.apu_avail || fcu.apu_start_on {
        fill_rect(img, 8, 8, w - 16, 21, scale_color(top_col, 0.14));
        draw_rect_border(img, 8, 8, w - 16, 21, 1, scale_color(top_col, 0.65));
    }
    let tw = text_width_5x7(&top_str, 1);
    draw_text_5x7(img, ((w - tw) / 2).max(6), 15, &top_str, 1, top_col);

    let lw = text_width_5x7(label, 1);
    let leg_col = if fcu.apu_avail {
        theme.phosphor_green
    } else if fcu.apu_start_on {
        theme.amber
    } else {
        theme.phosphor_dim
    };
    draw_text_5x7(img, ((w - lw) / 2).max(5), 46, label, 1, leg_col);
}

/// Airbus Pedestal Engine Mode Selector (NORM / IGN-STR / CRANK).
pub(super) fn render_eng_mode_selector(
    img: &mut RgbImage,
    w: i32,
    h: i32,
    tile: &TileConfig,
    fcu: &A320FcuState,
    theme: &ResolvedTheme,
) {
    let label = tile.label.as_deref().unwrap_or("ENG MOD");
    let (mode_str, main_col, sub_str) = match fcu.eng_mode {
        1 => ("IGN/STR", theme.amber, "START MODE"),
        -1 => ("CRANK", theme.amber, "DRY CRANK"),
        _ => ("NORM", theme.phosphor_green, "NORMAL"),
    };

    draw_rect_border(img, 3, 3, w - 6, h - 6, 1, theme.grid_dim);
    draw_text_5x7(img, 6, 7, label, 1, theme.phosphor_dim);

    fill_rect(img, 6, 22, w - 12, 25, scale_color(main_col, 0.12));
    draw_rect_border(img, 6, 22, w - 12, 25, 1, scale_color(main_col, 0.55));

    let vw = text_width_5x7(mode_str, 1);
    draw_text_5x7(img, ((w - vw) / 2).max(6), 31, mode_str, 1, main_col);

    let sw = (sub_str.len() as i32) * 4 - 1;
    draw_text_3x5(img, (w - sw) / 2, h - 13, sub_str, 1, theme.phosphor_dim);
}
