use crate::xplane::state::A320FcuState;

/// Panel 4 (`sys`): Exterior Lights, Anti-Ice, ADIRS, Seatbelts, Baro STD & TCAS.
pub(super) fn trigger_sys_control<F>(state: &mut A320FcuState, norm: &str, send_cmd: &mut F) -> bool
where
    F: FnMut(&str),
{
    match norm {
        "strobe" => {
            if state.strobe_on {
                send_cmd("laminar/A333/toggle_switch/strobe_pos_off");
            } else {
                send_cmd("laminar/A333/toggle_switch/strobe_pos_on");
            }
            send_cmd("sim/lights/strobe_lights_toggle");
            state.strobe_on = !state.strobe_on;
            true
        }
        "land_lights" | "landing_lights" => {
            send_cmd("sim/lights/landing_lights_toggle");
            state.land_lights_on = !state.land_lights_on;
            true
        }
        "nose_light" | "taxi_light" => {
            match state.nose_light_mode {
                0 => {
                    send_cmd("laminar/A333/switch/lighting/taxi_light_taxi");
                    send_cmd("sim/lights/taxi_lights_on");
                    state.nose_light_mode = 1;
                }
                1 => {
                    send_cmd("laminar/A333/switch/lighting/taxi_light_to");
                    state.nose_light_mode = 2;
                }
                _ => {
                    send_cmd("laminar/A333/switch/lighting/taxi_light_off");
                    send_cmd("sim/lights/taxi_lights_off");
                    state.nose_light_mode = 0;
                }
            }
            true
        }
        "wing_ai" => {
            send_cmd("sim/ice/wing_heat_tog");
            state.wing_ai_on = !state.wing_ai_on;
            true
        }
        "eng1_ai" => {
            send_cmd("sim/ice/inlet_heat0_tog");
            state.eng1_ai_on = !state.eng1_ai_on;
            true
        }
        "eng2_ai" => {
            send_cmd("sim/ice/inlet_heat1_tog");
            state.eng2_ai_on = !state.eng2_ai_on;
            true
        }
        "probe_heat" => {
            send_cmd("laminar/A333/buttons/probe_window_heat_toggle");
            send_cmd("sim/ice/pitot_heat0_tog");
            send_cmd("sim/ice/pitot_heat1_tog");
            state.probe_heat_on = !state.probe_heat_on;
            true
        }
        "nav_light" | "nav_lights" => {
            if state.nav_light_on {
                send_cmd("laminar/A333/toggle_switch/nav_light_pos_dn");
            } else {
                send_cmd("laminar/A333/toggle_switch/nav_light_pos_up");
            }
            send_cmd("sim/lights/nav_lights_toggle");
            state.nav_light_on = !state.nav_light_on;
            true
        }
        "adirs" => {
            send_cmd("laminar/A333/adirs_instant_align");
            send_cmd("laminar/A333/ai/adirs_quick_start");
            state.adirs_aligned = true;
            true
        }
        "seatbelt" | "seatbelts" => {
            if state.seatbelt_on {
                send_cmd("laminar/A333/switches/seatbelt_signs_dn");
            } else {
                send_cmd("laminar/A333/switches/seatbelt_signs_up");
            }
            send_cmd("sim/systems/seatbelt_sign_toggle");
            state.seatbelt_on = !state.seatbelt_on;
            true
        }
        "baro_std" => {
            if state.baro_std {
                send_cmd("laminar/A333/push/baro/capt_std");
            } else {
                send_cmd("laminar/A333/pull/baro/capt_std");
            }
            send_cmd("sim/instruments/barometer_std");
            state.baro_std = !state.baro_std;
            true
        }
        "tcas" => {
            send_cmd("laminar/A333/ai/transponder_quick_start");
            send_cmd("laminar/A333/transponder/ta_ra_right");
            state.tcas_tara = !state.tcas_tara;
            true
        }
        "ecam_all" => {
            send_cmd("laminar/A333/button/ecam/all_mode");
            state.ecam_page = "ALL";
            true
        }
        _ => false,
    }
}
