use crate::config::{ResolvedTheme, TileConfig};
use crate::font::{
    draw_rect_border, draw_text_3x5, draw_text_5x7, fill_rect, scale_color, text_width_5x7,
};
use crate::xplane::A320FcuState;
use image::RgbImage;

/// Glare-Shield Master Warning (Red) & Master Caution (Amber).
pub(super) fn render_master_alert(
    img: &mut RgbImage,
    w: i32,
    h: i32,
    ctrl: &str,
    fcu: &A320FcuState,
    theme: &ResolvedTheme,
) {
    let is_warn = ctrl.contains("warn");
    let active = if is_warn {
        fcu.master_warn
    } else {
        fcu.master_caut
    };
    let alert_col = if is_warn { theme.red } else { theme.amber };

    if active {
        fill_rect(img, 3, 3, w - 6, h - 6, scale_color(alert_col, 0.22));
        draw_rect_border(img, 2, 2, w - 4, h - 4, 2, alert_col);
    } else {
        draw_rect_border(img, 4, 4, w - 8, h - 8, 1, theme.grid_dim);
    }

    let fg = if active {
        alert_col
    } else {
        theme.phosphor_dim
    };
    let line1 = "MASTER";
    let line2 = if is_warn { "WARN" } else { "CAUT" };
    let w1 = text_width_5x7(line1, 1);
    let w2 = text_width_5x7(line2, 1);
    draw_text_5x7(img, (w - w1) / 2, 22, line1, 1, fg);
    draw_text_5x7(img, (w - w2) / 2, 38, line2, 1, fg);
}

/// Airbus FCU Digital LCD Windows (SPD, HDG, ALT, FPA, V/S).
pub(super) fn render_fcu_digital_window(
    img: &mut RgbImage,
    w: i32,
    h: i32,
    ctrl: &str,
    tile: &TileConfig,
    fcu: &A320FcuState,
    theme: &ResolvedTheme,
) {
    let (hdr, val_str, is_managed) = match ctrl {
        "spd" | "speed" | "spd_mach" => {
            let s = if fcu.spd_managed {
                "---".to_string()
            } else {
                format!("{:03.0}", fcu.spd_kts.clamp(100.0, 399.0))
            };
            ("SPD", s, fcu.spd_managed)
        }
        "hdg" | "heading" => {
            let s = if fcu.hdg_managed {
                "---".to_string()
            } else {
                format!("{:03.0}", fcu.hdg_deg.rem_euclid(360.0))
            };
            (
                if fcu.trk_fpa_mode { "TRK" } else { "HDG" },
                s,
                fcu.hdg_managed,
            )
        }
        "alt" | "altitude" => {
            let s = format!("{:05.0}", fcu.alt_ft.clamp(0.0, 49000.0));
            ("ALT", s, fcu.alt_managed)
        }
        "trk" | "hdg_trk" => {
            let mode_str = if fcu.trk_fpa_mode {
                "TRK/FPA"
            } else {
                "HDG/V/S"
            };
            ("FPA", mode_str.to_string(), !fcu.trk_fpa_mode)
        }
        _ => {
            let v = fcu.vs_fpm.round() as i32;
            let s = if v == 0 {
                "0000".to_string()
            } else {
                format!("{:+05}", v)
            };
            ("V/S", s, v == 0)
        }
    };

    let label = tile.label.as_deref().unwrap_or(hdr);
    let main_col = if is_managed {
        theme.phosphor_green
    } else {
        theme.amber
    };

    draw_rect_border(img, 3, 3, w - 6, h - 6, 1, theme.grid_dim);
    draw_text_5x7(img, 7, 7, label, 1, theme.phosphor_dim);

    if is_managed {
        fill_rect(img, w - 13, 8, 5, 5, theme.phosphor_green);
    } else {
        draw_text_3x5(img, w - 18, 8, "SEL", 1, theme.amber);
    }

    fill_rect(img, 6, 22, w - 12, 25, scale_color(main_col, 0.10));
    draw_rect_border(img, 6, 22, w - 12, 25, 1, scale_color(main_col, 0.45));

    let vw = text_width_5x7(&val_str, 1);
    draw_text_5x7(img, ((w - vw) / 2).max(8), 31, &val_str, 1, main_col);

    let sub = if is_managed { "MANAGED" } else { "SELECTED" };
    let sw = (sub.len() as i32) * 4 - 1;
    draw_text_3x5(img, (w - sw) / 2, h - 13, sub, 1, theme.phosphor_dim);
}
