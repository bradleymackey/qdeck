use crate::xplane::state::A320FcuState;

/// Panel 1 (`fcu`): Airbus Glare-Shield & Flight Control Unit controls.
pub(super) fn trigger_fcu_control<F>(state: &mut A320FcuState, norm: &str, send_cmd: &mut F) -> bool
where
    F: FnMut(&str),
{
    match norm {
        "ap1" | "ap_1" => {
            send_cmd("sim/autopilot/servos_toggle");
            send_cmd("toliss_airbus/ap1_push");
            send_cmd("AirbusFBW/AP1button");
            let was_on = state.ap1_on;
            state.ap1_on = !was_on;
            if was_on && !state.ap2_on {
                state.master_warn = true;
            }
            true
        }
        "ap2" | "ap_2" => {
            send_cmd("sim/autopilot/servos2_toggle");
            send_cmd("toliss_airbus/ap2_push");
            send_cmd("AirbusFBW/AP2button");
            let was_on = state.ap2_on;
            state.ap2_on = !was_on;
            if was_on && !state.ap1_on {
                state.master_warn = true;
            }
            true
        }
        "athr" | "a/thr" | "autothrottle" => {
            send_cmd("laminar/A333/autopilot/a_thr_toggle");
            send_cmd("sim/autopilot/autothrottle_toggle");
            send_cmd("toliss_airbus/athr_push");
            send_cmd("AirbusFBW/ATHRbutton");
            let was_on = state.athr_on;
            state.athr_on = !was_on;
            if was_on {
                state.master_caut = true;
            }
            true
        }
        "fd" | "fd1" | "flight_director" => {
            send_cmd("sim/autopilot/fdir_toggle");
            send_cmd("toliss_airbus/fd1_push");
            send_cmd("AirbusFBW/FD1button");
            state.fd_on = !state.fd_on;
            true
        }
        "loc" | "localizer" => {
            send_cmd("sim/autopilot/NAV");
            send_cmd("AirbusFBW/LOCbutton");
            state.loc_on = !state.loc_on;
            if state.loc_on {
                state.appr_on = false;
            }
            true
        }
        "appr" | "approach" => {
            send_cmd("sim/autopilot/approach");
            send_cmd("AirbusFBW/APPRbutton");
            state.appr_on = !state.appr_on;
            if state.appr_on {
                state.loc_on = true;
            }
            true
        }
        "exped" | "expedite" => {
            send_cmd("sim/autopilot/level_change");
            send_cmd("AirbusFBW/EXPEDbutton");
            state.exped_on = !state.exped_on;
            true
        }
        "hdg" | "heading" => {
            if state.hdg_managed {
                send_cmd("laminar/A333/autopilot/heading_knob_pull");
                send_cmd("sim/autopilot/heading");
                send_cmd("AirbusFBW/PullHDGSel");
            } else {
                send_cmd("laminar/A333/autopilot/heading_knob_push");
                send_cmd("sim/autopilot/NAV");
                send_cmd("AirbusFBW/PushHDGSel");
            }
            state.hdg_managed = !state.hdg_managed;
            true
        }
        "alt" | "altitude" => {
            if state.alt_managed {
                send_cmd("laminar/A333/autopilot/altitude_knob_pull");
                send_cmd("sim/autopilot/altitude_hold");
                send_cmd("AirbusFBW/PullAltitude");
            } else {
                send_cmd("laminar/A333/autopilot/altitude_knob_push");
                send_cmd("AirbusFBW/PushAltitude");
            }
            state.alt_managed = !state.alt_managed;
            true
        }
        "spd" | "speed" | "spd_mach" => {
            if state.spd_managed {
                send_cmd("laminar/A333/autopilot/speed_knob_pull");
                send_cmd("sim/autopilot/airspeed_sync");
                send_cmd("AirbusFBW/PullSPDSel");
            } else {
                send_cmd("laminar/A333/autopilot/speed_knob_push");
                send_cmd("AirbusFBW/PushSPDSel");
            }
            state.spd_managed = !state.spd_managed;
            true
        }
        "trk" | "hdg_trk" | "trkfpa" | "bird" => {
            send_cmd("sim/autopilot/trkfpa");
            send_cmd("AirbusFBW/HDGTRKbutton");
            state.trk_fpa_mode = !state.trk_fpa_mode;
            true
        }
        "vs" | "vs_zero" | "lvl_ch" => {
            send_cmd("laminar/A333/autopilot/vertical_knob_push");
            send_cmd("sim/autopilot/vertical_speed");
            send_cmd("AirbusFBW/PushVSSel");
            state.vs_fpm = 0.0;
            state.exped_on = false;
            true
        }
        "master_warn" | "warn" => {
            send_cmd("sim/annunciator/clear_master_warning");
            send_cmd("AirbusFBW/CaptWarnPush");
            state.master_warn = false;
            true
        }
        "master_caut" | "caut" | "caution" => {
            send_cmd("sim/annunciator/clear_master_caution");
            send_cmd("AirbusFBW/CaptCautPush");
            state.master_caut = false;
            true
        }
        _ => false,
    }
}
