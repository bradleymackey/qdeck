mod apu;
mod brakes;
mod fcu;
mod korry;
mod surfaces;

use super::TelemetrySnapshot;
use crate::config::{ResolvedTheme, TileConfig};
use crate::font::{draw_rect_border, scale_color};
use apu::{render_apu_start, render_eng_mode_selector};
use brakes::{render_autobrake_control, render_brake_fan_control, render_park_brake_control};
use fcu::{render_fcu_digital_window, render_master_alert};
use image::RgbImage;
use korry::{render_korry_three_bar, render_korry_two_window, render_nose_light_switch};
use surfaces::{render_flaps_control, render_landing_gear, render_speedbrake_control};

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
            render_landing_gear(&mut img, w, h, fcu, theme);
        }
        "flaps_up" | "flaps_dn" | "flaps_down" => {
            render_flaps_control(&mut img, w, h, &ctrl, fcu, theme);
        }
        "spdbrk_dn" | "spdbrk_retract" | "spdbrk_up" | "spdbrk_extend" => {
            render_speedbrake_control(&mut img, w, h, &ctrl, fcu, theme);
        }
        "park_brake" | "parking_brake" => {
            render_park_brake_control(&mut img, w, h, tile, fcu, theme);
        }
        "brake_fan" | "brk_fan" => {
            render_brake_fan_control(&mut img, w, h, tile, fcu, theme);
        }
        "abrk_lo" | "abrk_med" | "abrk_max" => {
            render_autobrake_control(&mut img, w, h, &ctrl, tile, fcu, theme);
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
