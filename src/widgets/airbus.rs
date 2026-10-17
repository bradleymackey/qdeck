use super::TelemetrySnapshot;
use crate::config::{ResolvedTheme, TileConfig};
use crate::font::{
    draw_hline, draw_rect_border, draw_text_3x5, draw_text_5x7, draw_triangle_down, fill_rect,
    scale_color, text_width_5x7,
};
use crate::xplane::A320FcuState;
use image::RgbImage;

/// Renders an Airbus A320/A330 control tile across Panels 1-4 (`fcu`, `apu`, `flt`, `sys`).
pub fn render_a320_fcu_tile(
    width: u32,
    height: u32,
    tile: &TileConfig,
    theme: &ResolvedTheme,
    telemetry: &TelemetrySnapshot,
    is_highlighted: bool,
) -> RgbImage {
    let w = width as i32;
    let h = height as i32;
    let fcu = telemetry.fcu;
    let ctrl = tile
        .control
        .as_deref()
        .unwrap_or(&tile.tile_type)
        .trim()
        .to_ascii_lowercase();

    let mut img = RgbImage::from_pixel(
        width,
        height,
        if is_highlighted {
            scale_color(theme.phosphor_green, 0.20)
        } else {
            theme.background
        },
    );

    match ctrl.as_str() {
        "master_warn" | "warn" | "master_caut" | "caut" | "caution" => {
            render_master_alert(&mut img, w, h, &ctrl, fcu, theme);
        }
        "spd" | "speed" | "spd_mach" | "hdg" | "heading" | "alt" | "altitude" | "trk"
        | "hdg_trk" | "vs" | "vs_zero" => {
            render_fcu_digital_window(&mut img, w, h, &ctrl, tile, fcu, theme);
        }
        "apu_start" | "apu_str" => {
            render_apu_start(&mut img, w, h, tile, fcu, theme);
        }
        "eng_mode" | "ign_start" => {
            render_eng_mode_selector(&mut img, w, h, tile, fcu, theme);
        }
        "gear" | "ldg_gear" => {
            render_landing_gear(&mut img, w, h, tile, fcu, theme);
        }
        "flaps_up" | "flaps_dn" | "flaps_down" => {
            render_flaps_control(&mut img, w, h, &ctrl, tile, fcu, theme);
        }
        "spdbrk_dn" | "spdbrk_retract" | "spdbrk_up" | "spdbrk_extend" => {
            render_speedbrake_control(&mut img, w, h, &ctrl, tile, fcu, theme);
        }
        "nose_light" | "taxi_light" => {
            render_nose_light_switch(&mut img, w, h, tile, fcu, theme);
        }
        "ap1" | "ap_1" | "ap2" | "ap_2" | "athr" | "a/thr" | "autothrottle" | "fd" | "fd1"
        | "flight_director" | "loc" | "localizer" | "exped" | "expedite" | "appr" | "approach" => {
            render_korry_three_bar(&mut img, w, h, &ctrl, tile, fcu, theme);
        }
        _ => {
            render_korry_two_window(&mut img, w, h, &ctrl, tile, fcu, theme);
        }
    }

    if is_highlighted {
        draw_rect_border(&mut img, 0, 0, w, h, 2, theme.phosphor_green);
    }

    img
}

/// Glare-Shield Master Warning (Red) & Master Caution (Amber).
fn render_master_alert(
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
fn render_fcu_digital_window(
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

/// Airbus Overhead APU Start Korry Pushbutton (AVAIL / START + N1%).
fn render_apu_start(
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
fn render_eng_mode_selector(
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

/// Airbus Center Instrument Panel 3-Green-Triangle Landing Gear Indicator (`LDG GEAR`).
fn render_landing_gear(
    img: &mut RgbImage,
    w: i32,
    h: i32,
    tile: &TileConfig,
    fcu: &A320FcuState,
    theme: &ResolvedTheme,
) {
    let label = tile.label.as_deref().unwrap_or("LDG GEAR");
    let border_col = if fcu.gear_down {
        scale_color(theme.phosphor_green, 0.65)
    } else {
        theme.grid_dim
    };
    draw_rect_border(img, 4, 4, w - 8, h - 8, 1, border_col);
    draw_hline(img, 5, 34, w - 10, theme.grid_dim);

    if fcu.gear_down {
        // 3 green triangles: Nose (center-top), Left, Right
        let tri_col = theme.phosphor_green;
        draw_triangle_down(img, 20, 14, 5, tri_col);
        draw_triangle_down(img, w / 2, 11, 5, tri_col);
        draw_triangle_down(img, w - 20, 14, 5, tri_col);
        draw_text_3x5(img, (w - 15) / 2, 25, "DN", 1, theme.phosphor_green);
    } else {
        let up_txt = "UP / RET";
        let tw = text_width_5x7(up_txt, 1);
        draw_text_5x7(
            img,
            ((w - tw) / 2).max(6),
            16,
            up_txt,
            1,
            scale_color(theme.phosphor_dim, 0.65),
        );
    }

    let leg_col = if fcu.gear_down {
        theme.phosphor_green
    } else {
        theme.phosphor_dim
    };
    let lw = text_width_5x7(label, 1);
    draw_text_5x7(img, ((w - lw) / 2).max(5), 46, label, 1, leg_col);
}

/// Airbus Flaps Notch Control (`FLAPS UP` / `FLAPS DN`) showing live flap setting (`0`, `1`, `2`, `3`, `FULL`).
fn render_flaps_control(
    img: &mut RgbImage,
    w: i32,
    h: i32,
    ctrl: &str,
    tile: &TileConfig,
    fcu: &A320FcuState,
    theme: &ResolvedTheme,
) {
    let default_label = if ctrl == "flaps_up" {
        "FLAPS UP"
    } else {
        "FLAPS DN"
    };
    let label = tile.label.as_deref().unwrap_or(default_label);

    let notch = (fcu.flap_ratio.clamp(0.0, 1.0) * 4.0).round() as u8;
    let (notch_str, col) = match notch {
        0 => ("FLAP 0", theme.phosphor_green),
        1 => ("FLAP 1", theme.amber),
        2 => ("FLAP 2", theme.amber),
        3 => ("FLAP 3", theme.amber),
        _ => ("FULL", theme.amber),
    };

    draw_rect_border(img, 4, 4, w - 8, h - 8, 1, scale_color(col, 0.55));
    draw_hline(img, 5, 33, w - 10, theme.grid_dim);

    fill_rect(img, 8, 9, w - 16, 19, scale_color(col, 0.12));
    let tw = text_width_5x7(notch_str, 1);
    draw_text_5x7(img, ((w - tw) / 2).max(6), 15, notch_str, 1, col);

    let lw = text_width_5x7(label, 1);
    draw_text_5x7(img, ((w - lw) / 2).max(5), 46, label, 1, theme.phosphor_dim);
}

/// Airbus Speedbrake / Ground Spoiler Control (`SPDBRK -` / `SPDBRK +`).
fn render_speedbrake_control(
    img: &mut RgbImage,
    w: i32,
    h: i32,
    ctrl: &str,
    tile: &TileConfig,
    fcu: &A320FcuState,
    theme: &ResolvedTheme,
) {
    let default_label = if ctrl.contains("dn") || ctrl.contains("retract") {
        "SPDBRK -"
    } else {
        "SPDBRK +"
    };
    let label = tile.label.as_deref().unwrap_or(default_label);

    let (status_str, col) = if fcu.spdbrk_ratio < -0.1 {
        ("ARMED".to_string(), theme.phosphor_green)
    } else if fcu.spdbrk_ratio <= 0.05 {
        ("RET".to_string(), theme.phosphor_green)
    } else if fcu.spdbrk_ratio >= 0.95 {
        ("FULL".to_string(), theme.amber)
    } else {
        (
            format!("{:.0}%", fcu.spdbrk_ratio * 100.0),
            theme.amber,
        )
    };

    draw_rect_border(img, 4, 4, w - 8, h - 8, 1, scale_color(col, 0.55));
    draw_hline(img, 5, 33, w - 10, theme.grid_dim);

    fill_rect(img, 8, 9, w - 16, 19, scale_color(col, 0.12));
    let tw = text_width_5x7(&status_str, 1);
    draw_text_5x7(img, ((w - tw) / 2).max(6), 15, &status_str, 1, col);

    let lw = text_width_5x7(label, 1);
    draw_text_5x7(img, ((w - lw) / 2).max(5), 46, label, 1, theme.phosphor_dim);
}

/// Airbus Overhead 3-Position Nose Light Switch (`OFF` / `TAXI` / `T.O`).
fn render_nose_light_switch(
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
fn render_korry_two_window(
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
        // Panel 3 (Brakes, Autobrake, T.O Config, ECAM F/CTL & WHEEL)
        "park_brake" | "parking_brake" => ("PARK BRK", fcu.park_brake_on, "PARK", "REL", true),
        "brake_fan" | "brk_fan" => ("BRK FAN", fcu.brake_fan_on, "ON", "OFF", false),
        "abrk_lo" => ("A/BRK LO", fcu.autobrake_mode == 1, "ON", "LO", false),
        "abrk_med" => ("A/BRK MD", fcu.autobrake_mode == 2, "ON", "MED", false),
        "abrk_max" => ("A/BRK MX", fcu.autobrake_mode == 3, "ON", "MAX", true),
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
fn render_korry_three_bar(
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
