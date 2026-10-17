use super::state::A320FcuState;

/// Dispatches an Airbus control press across Panels 1-4 (`fcu`, `apu`, `flt`, `sys`),
/// invoking `send_cmd` for each X-Plane command and updating `state` optimistically.
pub fn dispatch_control<F>(
    state: &mut A320FcuState,
    control_name: &str,
    custom_cmd: Option<&str>,
    mut send_cmd: F,
) where
    F: FnMut(&str),
{
    if let Some(cmd) = custom_cmd {
        if !cmd.trim().is_empty() {
            for single in cmd.split(';') {
                let trimmed = single.trim();
                if !trimmed.is_empty() {
                    send_cmd(trimmed);
                }
            }
        }
    }

    let norm = control_name.trim().to_ascii_lowercase();
    if trigger_fcu_control(state, &norm, &mut send_cmd)
        || trigger_apu_control(state, &norm, &mut send_cmd)
        || trigger_flt_control(state, &norm, &mut send_cmd)
        || trigger_sys_control(state, &norm, &mut send_cmd)
    {
        return;
    }

    if custom_cmd.is_none() && !norm.is_empty() {
        send_cmd(&norm);
    }
}

/// Panel 1 (`fcu`): Airbus Glare-Shield & Flight Control Unit controls.
fn trigger_fcu_control<F>(state: &mut A320FcuState, norm: &str, send_cmd: &mut F) -> bool
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

/// Panel 2 (`apu`): Airbus Overhead Electrical, APU & Engine Start Prep controls.
fn trigger_apu_control<F>(state: &mut A320FcuState, norm: &str, send_cmd: &mut F) -> bool
where
    F: FnMut(&str),
{
    match norm {
        "bat1" | "battery1" => {
            send_cmd("sim/electrical/battery_1_toggle");
            send_cmd("AirbusFBW/Bat1Toggle");
            state.bat1_on = !state.bat1_on;
            true
        }
        "bat2" | "battery2" => {
            send_cmd("sim/electrical/battery_2_toggle");
            send_cmd("laminar/A333/buttons/APU_batt_toggle");
            send_cmd("AirbusFBW/Bat2Toggle");
            state.bat2_on = !state.bat2_on;
            true
        }
        "ext_pwr" | "external_power" | "gpu" => {
            send_cmd("laminar/A333/buttons/external_powerA_toggle");
            send_cmd("sim/electrical/gpu_toggle");
            send_cmd("AirbusFBW/ExtPwrToggle");
            state.ext_pwr_on = !state.ext_pwr_on;
            true
        }
        "apu_master" | "apu_mst" => {
            send_cmd("laminar/A333/buttons/APU_master");
            if state.apu_master_on {
                send_cmd("sim/electrical/APU_off");
            } else {
                send_cmd("sim/electrical/APU_on");
            }
            send_cmd("AirbusFBW/APUMaster");
            state.apu_master_on = !state.apu_master_on;
            if !state.apu_master_on {
                state.apu_start_on = false;
                state.apu_avail = false;
            }
            true
        }
        "apu_start" | "apu_str" => {
            send_cmd("laminar/A333/buttons/APU_start");
            send_cmd("sim/electrical/APU_start");
            send_cmd("AirbusFBW/APUStarter");
            if !state.apu_avail {
                state.apu_master_on = true;
                state.apu_start_on = true;
            }
            true
        }
        "apu_bleed" | "apu_bld" => {
            send_cmd("sim/bleed_air/apu_toggle");
            send_cmd("AirbusFBW/APUBleedSwitch");
            state.apu_bleed_on = !state.apu_bleed_on;
            true
        }
        "apu_gen" => {
            send_cmd("laminar/A333/buttons/APU_gen_toggle");
            send_cmd("sim/electrical/APU_generator_toggle");
            state.apu_gen_on = !state.apu_gen_on;
            true
        }
        "fuel_pumps" | "fuel" | "pumps" => {
            for pump_cmd in [
                "laminar/A333/switches/left1_pump_toggle",
                "laminar/A333/switches/left2_pump_toggle",
                "laminar/A333/switches/right1_pump_toggle",
                "laminar/A333/switches/right2_pump_toggle",
                "laminar/A333/switches/center_left_pump_toggle",
                "laminar/A333/switches/center_right_pump_toggle",
            ] {
                send_cmd(pump_cmd);
            }
            state.fuel_pumps_on = !state.fuel_pumps_on;
            true
        }
        "eng_mode" | "ign_start" => {
            // Cycle: NORM (0) -> IGN/START (1) -> CRANK (-1) -> NORM (0)
            match state.eng_mode {
                0 => {
                    send_cmd("laminar/A333/switch/eng_mode_right");
                    state.eng_mode = 1;
                }
                1 => {
                    send_cmd("laminar/A333/switch/eng_mode_left");
                    send_cmd("laminar/A333/switch/eng_mode_left");
                    state.eng_mode = -1;
                }
                _ => {
                    send_cmd("laminar/A333/switch/eng_mode_right");
                    state.eng_mode = 0;
                }
            }
            true
        }
        "packs" | "pack" => {
            send_cmd("sim/bleed_air/pack_left_toggle");
            send_cmd("sim/bleed_air/pack_right_toggle");
            state.packs_on = !state.packs_on;
            true
        }
        "beacon" => {
            send_cmd("sim/lights/beacon_lights_toggle");
            state.beacon_on = !state.beacon_on;
            true
        }
        "ecam_apu" => {
            send_cmd("laminar/A333/button/ecam/apu_mode");
            state.ecam_page = "APU";
            true
        }
        "ecam_eng" => {
            send_cmd("laminar/A333/button/ecam/eng_mode");
            state.ecam_page = "ENG";
            true
        }
        _ => false,
    }
}

/// Panel 3 (`flt`): Landing Gear, Autobrake, Flaps, Spoilers & Flight Control surfaces.
fn trigger_flt_control<F>(state: &mut A320FcuState, norm: &str, send_cmd: &mut F) -> bool
where
    F: FnMut(&str),
{
    match norm {
        "gear" | "ldg_gear" => {
            send_cmd("sim/flight_controls/landing_gear_toggle");
            state.gear_down = !state.gear_down;
            true
        }
        "park_brake" | "parking_brake" => {
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

/// Panel 4 (`sys`): Exterior Lights, Anti-Ice, ADIRS, Seatbelts, Baro STD & TCAS.
fn trigger_sys_control<F>(state: &mut A320FcuState, norm: &str, send_cmd: &mut F) -> bool
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fcu_panel_controls() {
        let mut state = A320FcuState::default();
        let mut sent: Vec<String> = Vec::new();

        // Engage AP 1, then disengage AP 1 -> triggers Master Warning
        dispatch_control(&mut state, "ap1", None, |c| sent.push(c.to_string()));
        assert!(state.ap1_on);
        assert!(sent.contains(&"sim/autopilot/servos_toggle".to_string()));

        dispatch_control(&mut state, "ap1", None, |_| {});
        assert!(!state.ap1_on);
        assert!(state.master_warn);

        // Clear Master Warning
        dispatch_control(&mut state, "master_warn", None, |_| {});
        assert!(!state.master_warn);
    }

    #[test]
    fn test_apu_panel_controls() {
        let mut state = A320FcuState::default();
        let mut sent: Vec<String> = Vec::new();

        dispatch_control(&mut state, "apu_master", None, |c| sent.push(c.to_string()));
        assert!(state.apu_master_on);
        assert!(sent.contains(&"laminar/A333/buttons/APU_master".to_string()));

        // Cycle Engine Mode: NORM (0) -> IGN/START (1) -> CRANK (-1) -> NORM (0)
        assert_eq!(state.eng_mode, 0);
        dispatch_control(&mut state, "eng_mode", None, |_| {});
        assert_eq!(state.eng_mode, 1);
        dispatch_control(&mut state, "eng_mode", None, |_| {});
        assert_eq!(state.eng_mode, -1);
        dispatch_control(&mut state, "eng_mode", None, |_| {});
        assert_eq!(state.eng_mode, 0);
    }

    #[test]
    fn test_flt_panel_controls() {
        let mut state = A320FcuState::default();
        let mut sent: Vec<String> = Vec::new();

        assert!(state.gear_down);
        dispatch_control(&mut state, "gear", None, |c| sent.push(c.to_string()));
        assert!(!state.gear_down);
        assert!(sent.contains(&"sim/flight_controls/landing_gear_toggle".to_string()));

        // Flaps down 2 notches -> 0.50, then up 1 notch -> 0.25
        dispatch_control(&mut state, "flaps_dn", None, |_| {});
        dispatch_control(&mut state, "flaps_dn", None, |_| {});
        assert!((state.flap_ratio - 0.50).abs() < 0.001);
        dispatch_control(&mut state, "flaps_up", None, |_| {});
        assert!((state.flap_ratio - 0.25).abs() < 0.001);

        // Autobrake radio-toggle behavior
        dispatch_control(&mut state, "abrk_med", None, |_| {});
        assert_eq!(state.autobrake_mode, 2);
        dispatch_control(&mut state, "abrk_med", None, |_| {});
        assert_eq!(state.autobrake_mode, 0);
    }

    #[test]
    fn test_sys_panel_controls() {
        let mut state = A320FcuState::default();
        // Nose light cycles OFF (0) -> TAXI (1) -> T.O (2) -> OFF (0)
        assert_eq!(state.nose_light_mode, 0);
        dispatch_control(&mut state, "nose_light", None, |_| {});
        assert_eq!(state.nose_light_mode, 1);
        dispatch_control(&mut state, "nose_light", None, |_| {});
        assert_eq!(state.nose_light_mode, 2);
        dispatch_control(&mut state, "nose_light", None, |_| {});
        assert_eq!(state.nose_light_mode, 0);

        // Baro STD toggle
        dispatch_control(&mut state, "baro_std", None, |_| {});
        assert!(state.baro_std);
    }
}
