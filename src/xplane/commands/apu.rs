use crate::xplane::state::A320FcuState;

/// Panel 2 (`apu`): Airbus Overhead Electrical, APU & Engine Start Prep controls.
pub(super) fn trigger_apu_control<F>(state: &mut A320FcuState, norm: &str, send_cmd: &mut F) -> bool
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
