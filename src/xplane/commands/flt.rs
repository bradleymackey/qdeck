use crate::xplane::state::A320FcuState;

/// Panel 3 (`flt`): Landing Gear, Autobrake, Flaps, Spoilers & Flight Control surfaces.
pub(super) fn trigger_flt_control<F>(state: &mut A320FcuState, norm: &str, send_cmd: &mut F) -> bool
where
    F: FnMut(&str),
{
    match norm {
        "gear" | "ldg_gear" => {
            send_cmd("sim/flight_controls/landing_gear_toggle");
            state.gear_handle_down = !state.gear_handle_down;
            let xplane_live = state.last_rx.is_some_and(|t| t.elapsed().as_secs() < 5);
            if !xplane_live {
                state.set_all_gear_deployed(state.gear_handle_down);
            }
            true
        }
        "park_brake" | "parking_brake" => {
            if state.park_brake_on {
                send_cmd("laminar/A333/switch/parking_brake_left");
            } else {
                send_cmd("laminar/A333/switch/parking_brake_right");
            }
            send_cmd("sim/flight_controls/brakes_toggle_max");
            state.park_brake_on = !state.park_brake_on;
            true
        }
        "brake_fan" | "brk_fan" => {
            send_cmd("laminar/A333/buttons/gear/brake_fan_tog");
            state.brake_fan_on = !state.brake_fan_on;
            true
        }
        "abrk_lo" => {
            send_cmd("sim/flight_controls/brakes_1_auto");
            state.autobrake_mode = if state.autobrake_mode == 1 { 0 } else { 1 };
            true
        }
        "abrk_med" => {
            send_cmd("sim/flight_controls/brakes_2_auto");
            state.autobrake_mode = if state.autobrake_mode == 2 { 0 } else { 2 };
            true
        }
        "abrk_max" => {
            send_cmd("sim/flight_controls/brakes_rto_auto");
            state.autobrake_mode = if state.autobrake_mode == 3 { 0 } else { 3 };
            true
        }
        "flaps_up" => {
            send_cmd("sim/flight_controls/flaps_up");
            state.flap_ratio = (state.flap_ratio - 0.25).clamp(0.0, 1.0);
            true
        }
        "flaps_dn" | "flaps_down" => {
            send_cmd("sim/flight_controls/flaps_down");
            state.flap_ratio = (state.flap_ratio + 0.25).clamp(0.0, 1.0);
            true
        }
        "spdbrk_dn" | "spdbrk_retract" => {
            send_cmd("sim/flight_controls/speed_brakes_down_one");
            state.spdbrk_ratio = (state.spdbrk_ratio - 0.25).clamp(-0.5, 1.0);
            true
        }
        "spdbrk_up" | "spdbrk_extend" => {
            send_cmd("sim/flight_controls/speed_brakes_up_one");
            state.spdbrk_ratio = (state.spdbrk_ratio + 0.25).clamp(-0.5, 1.0);
            true
        }
        "to_config" | "to_cfg" => {
            send_cmd("laminar/A333/button/ecam/to_config_test");
            state.to_config_ok = true;
            true
        }
        "ecam_fctl" => {
            send_cmd("laminar/A333/button/ecam/f_ctl_mode");
            state.ecam_page = "F/CTL";
            true
        }
        "ecam_whl" | "ecam_wheel" => {
            send_cmd("laminar/A333/button/ecam/wheel_mode");
            state.ecam_page = "WHEEL";
            true
        }
        _ => false,
    }
}
