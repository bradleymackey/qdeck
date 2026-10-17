use crate::config::ResolvedTheme;
use crate::font::{
    draw_hline, draw_rect_border, draw_text_3x5, draw_text_5x7, draw_triangle_down, scale_color,
    text_width_5x7,
};
use crate::widgets::icons::{
    draw_arrow_down, draw_arrow_up, draw_flap_notches, draw_speedbrake_icon, draw_triangle_up,
};
use crate::xplane::A320FcuState;
use image::RgbImage;

/// Airbus Center Instrument Panel 3-Triangle Landing Gear Indicator (`LDG GEAR`)
/// showing actual Left Main (`[1]`), Nose (`[0]`), and Right Main (`[2]`) gear states.
pub(super) fn render_landing_gear(
    img: &mut RgbImage,
    w: i32,
    h: i32,
    fcu: &A320FcuState,
    theme: &ResolvedTheme,
) {
    let in_transit = fcu.is_gear_in_transit();
    let all_down = fcu.gear_down;
    let border_col = if in_transit {
        theme.red
    } else if all_down {
        scale_color(theme.phosphor_green, 0.65)
    } else {
        theme.grid_dim
    };
    draw_rect_border(img, 4, 4, w - 8, h - 8, 1, border_col);
    draw_hline(img, 5, 35, w - 10, theme.grid_dim);

    // Draw each of the 3 struts: Left Main [1], Nose [0], Right Main [2]
    let struts = [
        (20, 14, fcu.gear_deploy[1]),
        (w / 2, 10, fcu.gear_deploy[0]),
        (w - 20, 14, fcu.gear_deploy[2]),
    ];
    for &(cx, ty, ratio) in &struts {
        if ratio >= 0.98 {
            // Locked down: bright green downward triangle
            draw_triangle_down(img, cx, ty, 6, theme.phosphor_green);
        } else if ratio > 0.02 {
            // Unlocked / in transit: red downward triangle (Airbus UNLK light)
            draw_triangle_down(img, cx, ty, 6, theme.red);
        } else {
            // Fully retracted up: dim upward triangle
            draw_triangle_up(img, cx, ty - 2, 5, scale_color(theme.phosphor_dim, 0.70));
        }
    }

    if in_transit {
        draw_text_3x5(img, (w - 15) / 2, 25, "UNLK", 1, theme.red);
    } else if all_down {
        draw_text_3x5(img, (w - 15) / 2, 25, "DOWN", 1, theme.phosphor_green);
    } else {
        draw_text_3x5(
            img,
            (w - 7) / 2,
            24,
            "UP",
            1,
            scale_color(theme.phosphor_dim, 0.70),
        );
    }

    let leg_col = if in_transit {
        theme.amber
    } else if all_down {
        theme.phosphor_green
    } else {
        theme.phosphor_dim
    };
    draw_text_5x7(img, 12, 46, "GEAR", 1, leg_col);
    if all_down {
        draw_arrow_down(img, 48, 43, 12, 4, theme.phosphor_green);
    } else if in_transit {
        if fcu.gear_handle_down {
            draw_arrow_down(img, 48, 43, 12, 4, theme.amber);
        } else {
            draw_arrow_up(img, 48, 43, 12, 4, theme.amber);
        }
    } else {
        draw_arrow_up(img, 48, 43, 12, 4, theme.phosphor_dim);
    }
}

/// Airbus Flaps Control (`flaps_up` / `flaps_dn`) with prominent Up/Down arrows and 4-notch visual bar.
pub(super) fn render_flaps_control(
    img: &mut RgbImage,
    w: i32,
    h: i32,
    ctrl: &str,
    fcu: &A320FcuState,
    theme: &ResolvedTheme,
) {
    let is_up = ctrl == "flaps_up";
    let notch = (fcu.flap_ratio.clamp(0.0, 1.0) * 4.0).round() as u8;
    let (notch_str, col) = match notch {
        0 => ("0", theme.phosphor_green),
        1 => ("1", theme.amber),
        2 => ("2", theme.amber),
        3 => ("3", theme.amber),
        _ => ("FULL", theme.amber),
    };

    draw_rect_border(img, 4, 4, w - 8, h - 8, 1, scale_color(col, 0.55));
    draw_hline(img, 5, 36, w - 10, theme.grid_dim);

    // Top-left: prominent directional arrow icon (↑ for retract, ↓ for extend)
    let arrow_col = if is_up {
        theme.phosphor_green
    } else {
        theme.amber
    };
    if is_up {
        draw_arrow_up(img, 16, 10, 14, 5, arrow_col);
    } else {
        draw_arrow_down(img, 16, 10, 14, 5, arrow_col);
    }

    // Top-right: current flap notch readout
    let disp = if notch == 4 {
        "FULL".to_string()
    } else {
        format!("FLP {notch_str}")
    };
    draw_text_5x7(img, 28, 13, &disp, 1, col);

    // Visual 4-notch flap bar along bottom of top window
    draw_flap_notches(
        img,
        17,
        28,
        notch,
        col,
        scale_color(theme.phosphor_dim, 0.25),
    );

    // Bottom legend: "FLAPS" + crisp Up/Down arrow icon
    draw_text_5x7(img, 10, 47, "FLAPS", 1, theme.phosphor_dim);
    if is_up {
        draw_arrow_up(img, 52, 44, 13, 4, arrow_col);
    } else {
        draw_arrow_down(img, 52, 44, 13, 4, arrow_col);
    }
}

/// Airbus Speedbrake / Ground Spoiler Control (`spdbrk_dn` / `spdbrk_up`) with wing spoiler icon & arrows.
pub(super) fn render_speedbrake_control(
    img: &mut RgbImage,
    w: i32,
    h: i32,
    ctrl: &str,
    fcu: &A320FcuState,
    theme: &ResolvedTheme,
) {
    let is_retract = ctrl.contains("dn") || ctrl.contains("retract");
    let (status_str, col) = if fcu.spdbrk_ratio < -0.1 {
        ("ARM".to_string(), theme.phosphor_green)
    } else if fcu.spdbrk_ratio <= 0.05 {
        ("RET".to_string(), theme.phosphor_green)
    } else if fcu.spdbrk_ratio >= 0.95 {
        ("FULL".to_string(), theme.amber)
    } else {
        (format!("{:.0}%", fcu.spdbrk_ratio * 100.0), theme.amber)
    };

    draw_rect_border(img, 4, 4, w - 8, h - 8, 1, scale_color(col, 0.55));
    draw_hline(img, 5, 35, w - 10, theme.grid_dim);

    // Top-left: wing cross-section with hinged spoiler flap
    draw_speedbrake_icon(
        img,
        10,
        14,
        fcu.spdbrk_ratio.max(0.0),
        col,
        theme.phosphor_dim,
    );

    // Top-right: live spoiler deployment readout
    let tw = text_width_5x7(&status_str, 1);
    draw_text_5x7(img, (48 - tw / 2).max(30), 16, &status_str, 1, col);

    // Bottom legend: "SPDBRK" + crisp Down/Up arrow icon
    let arrow_col = if is_retract {
        theme.phosphor_green
    } else {
        theme.amber
    };
    draw_text_5x7(img, 7, 47, "SPDBRK", 1, theme.phosphor_dim);
    if is_retract {
        draw_arrow_down(img, 55, 44, 13, 4, arrow_col);
    } else {
        draw_arrow_up(img, 55, 44, 13, 4, arrow_col);
    }
}
