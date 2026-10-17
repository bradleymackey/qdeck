use crate::config::{ResolvedTheme, TileConfig};
use crate::font::{
    draw_hline, draw_rect_border, draw_text_3x5, draw_text_5x7, fill_rect, scale_color,
    text_width_5x7,
};
use crate::xplane::A320FcuState;
use image::RgbImage;

/// Airbus Overhead 3-Position Nose Light Switch (`OFF` / `TAXI` / `T.O`).
pub(super) fn render_nose_light_switch(
    img: &mut RgbImage,
    w: i32,
    h: i32,
    tile: &TileConfig,
    fcu: &A320FcuState,
    theme: &ResolvedTheme,
) {
    let label = tile.label.as_deref().unwrap_or("NOSE LT");
    let (top_txt, col, is_active) = match fcu.nose_light_mode {
        1 => ("TAXI", theme.phosphor_green, true),
        2 => ("T.O", theme.amber, true),
        _ => ("OFF", scale_color(theme.phosphor_dim, 0.45), false),
    };

    let border_col = if is_active {
        scale_color(col, 0.65)
    } else {
        theme.grid_dim
    };
    draw_rect_border(img, 4, 4, w - 8, h - 8, 1, border_col);
    draw_hline(img, 5, 33, w - 10, theme.grid_dim);

    if is_active {
        fill_rect(img, 10, 9, w - 20, 19, scale_color(col, 0.14));
        draw_rect_border(img, 10, 9, w - 20, 19, 1, scale_color(col, 0.60));
    }
    let tw = text_width_5x7(top_txt, 1);
    draw_text_5x7(img, ((w - tw) / 2).max(6), 15, top_txt, 1, col);

    let leg_col = if is_active { col } else { theme.phosphor_dim };
    let lw = text_width_5x7(label, 1);
    draw_text_5x7(img, ((w - lw) / 2).max(5), 46, label, 1, leg_col);
}

/// Airbus 2-Window Korry Switches across Panels 2 (`apu`), 3 (`flt`), and 4 (`sys`).
pub(super) fn render_korry_two_window(
    img: &mut RgbImage,
    w: i32,
    h: i32,
    ctrl: &str,
    tile: &TileConfig,
    fcu: &A320FcuState,
    theme: &ResolvedTheme,
) {
    let (default_label, is_on, top_on_txt, top_off_txt, use_amber_when_on) = match ctrl {
        // Panel 2 (APU / Electrical / Fuel / Bleed)
        "bat1" | "battery1" => ("BAT 1", fcu.bat1_on, "ON", "OFF", false),
        "bat2" | "battery2" => ("BAT 2", fcu.bat2_on, "ON", "OFF", false),
        "ext_pwr" | "external_power" | "gpu" => ("EXT PWR", fcu.ext_pwr_on, "ON", "AVAIL", false),
        "apu_master" | "apu_mst" => ("APU MST", fcu.apu_master_on, "ON", "OFF", false),
        "apu_bleed" | "apu_bld" => ("APU BLD", fcu.apu_bleed_on, "ON", "OFF", false),
        "apu_gen" => ("APU GEN", fcu.apu_gen_on, "ON", "OFF", false),
        "fuel_pumps" | "fuel" | "pumps" => ("FUEL PMP", fcu.fuel_pumps_on, "ON", "OFF", false),
        "packs" | "pack" => ("PACK 1/2", fcu.packs_on, "ON", "OFF", false),
        "beacon" => ("BEACON", fcu.beacon_on, "ON", "OFF", true),
        "ecam_apu" => ("ECAM APU", fcu.ecam_page == "APU", "PAGE", "ECAM", false),
        "ecam_eng" => ("ECAM ENG", fcu.ecam_page == "ENG", "PAGE", "ECAM", false),
        // Panel 3 (T.O Config, ECAM F/CTL & WHEEL)
        "to_config" | "to_cfg" => ("T.O CFG", fcu.to_config_ok, "NORM", "TEST", false),
        "ecam_fctl" => ("ECAM F/C", fcu.ecam_page == "F/CTL", "PAGE", "ECAM", false),
        "ecam_whl" | "ecam_wheel" => ("ECAM WHL", fcu.ecam_page == "WHEEL", "PAGE", "ECAM", false),
        // Panel 4 (Lights, Anti-Ice, ADIRS, Seatbelts, Baro STD, TCAS, ECAM ALL)
        "strobe" => ("STROBE", fcu.strobe_on, "ON", "AUTO", false),
        "land_lights" | "landing_lights" => ("LAND LT", fcu.land_lights_on, "ON", "RET", false),
        "wing_ai" => ("WING A/I", fcu.wing_ai_on, "ON", "OFF", false),
        "eng1_ai" => ("ENG1 A/I", fcu.eng1_ai_on, "ON", "OFF", false),
        "eng2_ai" => ("ENG2 A/I", fcu.eng2_ai_on, "ON", "OFF", false),
        "probe_heat" => ("PRB HEAT", fcu.probe_heat_on, "ON", "AUTO", false),
        "nav_light" | "nav_lights" => ("NAV LT", fcu.nav_light_on, "ON", "OFF", false),
        "adirs" => ("ADIRS", fcu.adirs_aligned, "NAV", "ALGN", false),
        "seatbelt" | "seatbelts" => ("SEATBLT", fcu.seatbelt_on, "ON", "OFF", false),
        "baro_std" => ("BARO STD", fcu.baro_std, "STD", "QNH", true),
        "tcas" => ("TCAS", fcu.tcas_tara, "TA/RA", "STBY", false),
        "ecam_all" => ("ECAM ALL", fcu.ecam_page == "ALL", "PAGE", "ECAM", false),
        _ => ("OVHD", false, "ON", "OFF", false),
    };

    let label = tile.label.as_deref().unwrap_or(default_label);
    let active_accent = if use_amber_when_on {
        theme.amber
    } else {
        theme.phosphor_green
    };
    let outer_col = if is_on {
        scale_color(active_accent, 0.65)
    } else {
        theme.grid_dim
    };

    draw_rect_border(img, 4, 4, w - 8, h - 8, 1, outer_col);
    draw_hline(img, 5, 33, w - 10, theme.grid_dim);

    let top_txt = if is_on { top_on_txt } else { top_off_txt };
    let top_col = if is_on {
        active_accent
    } else if top_off_txt == "AVAIL" {
        theme.amber
    } else {
        scale_color(theme.phosphor_dim, 0.45)
    };

    if is_on {
        fill_rect(img, 10, 9, w - 20, 19, scale_color(top_col, 0.14));
        draw_rect_border(img, 10, 9, w - 20, 19, 1, scale_color(top_col, 0.60));
    }
    let tw = text_width_5x7(top_txt, 1);
    draw_text_5x7(img, ((w - tw) / 2).max(6), 15, top_txt, 1, top_col);

    let legend_col = if is_on {
        active_accent
    } else {
        theme.phosphor_dim
    };
    if label.len() <= 8 {
        let lw = text_width_5x7(label, 1);
        draw_text_5x7(img, ((w - lw) / 2).max(5), 46, label, 1, legend_col);
    } else {
        let lw = (label.len() as i32) * 4 - 1;
        draw_text_3x5(img, ((w - lw) / 2).max(5), 48, label, 1, legend_col);
    }
}

/// Classic Airbus A320 Korry 3-Bar Pushbuttons (AP 1, AP 2, A/THR, FD 1, LOC, EXPED, APPR).
pub(super) fn render_korry_three_bar(
    img: &mut RgbImage,
    w: i32,
    h: i32,
    ctrl: &str,
    tile: &TileConfig,
    fcu: &A320FcuState,
    theme: &ResolvedTheme,
) {
    let (default_label, engaged) = match ctrl {
        "ap1" | "ap_1" => ("AP 1", fcu.ap1_on),
        "ap2" | "ap_2" => ("AP 2", fcu.ap2_on),
        "athr" | "a/thr" | "autothrottle" => ("A/THR", fcu.athr_on),
        "fd" | "fd1" | "flight_director" => ("FD 1", fcu.fd_on),
        "loc" | "localizer" => ("LOC", fcu.loc_on),
        "exped" | "expedite" => ("EXPED", fcu.exped_on),
        "appr" | "approach" => ("APPR", fcu.appr_on),
        _ => ("FCU", false),
    };

    let label = tile.label.as_deref().unwrap_or(default_label);
    let outer_col = if engaged {
        scale_color(theme.phosphor_green, 0.65)
    } else {
        theme.grid_dim
    };

    draw_rect_border(img, 4, 4, w - 8, h - 8, 1, outer_col);
    draw_hline(img, 5, 31, w - 10, theme.grid_dim);

    let bar_color = if engaged {
        theme.phosphor_green
    } else {
        scale_color(theme.phosphor_dim, 0.16)
    };
    let bar_w = 13;
    let bar_h = 5;
    let bar_gap = 4;
    let total_bars_w = 3 * bar_w + 2 * bar_gap;
    let start_x = (w - total_bars_w) / 2;
    for b in 0..3 {
        let bx = start_x + b * (bar_w + bar_gap);
        fill_rect(img, bx, 15, bar_w, bar_h, bar_color);
    }

    let legend_col = if engaged {
        theme.phosphor_green
    } else {
        theme.phosphor_dim
    };
    let lw = text_width_5x7(label, 1);
    draw_text_5x7(img, ((w - lw) / 2).max(6), 45, label, 1, legend_col);
}
