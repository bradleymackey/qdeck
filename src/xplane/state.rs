use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct A320FcuState {
    // Panel 1: FCU / Autopilot
    pub fd_on: bool,
    pub ap1_on: bool,
    pub ap2_on: bool,
    pub athr_on: bool,
    pub loc_on: bool,
    pub exped_on: bool,
    pub appr_on: bool,
    pub trk_fpa_mode: bool,
    pub master_warn: bool,
    pub master_caut: bool,
    pub hdg_managed: bool,
    pub alt_managed: bool,
    pub spd_managed: bool,
    pub hdg_deg: f32,
    pub alt_ft: f32,
    pub spd_kts: f32,
    pub vs_fpm: f32,

    // Panel 2: Overhead Electrical, APU & Engine Start Prep
    pub bat1_on: bool,
    pub bat2_on: bool,
    pub ext_pwr_on: bool,
    pub apu_master_on: bool,
    pub apu_start_on: bool,
    pub apu_avail: bool,
    pub apu_n1_pct: f32,
    pub apu_bleed_on: bool,
    pub apu_gen_on: bool,
    pub fuel_pumps_on: bool,
    pub packs_on: bool,
    pub beacon_on: bool,
    /// Engine Mode Selector: -1 = CRANK, 0 = NORM, 1 = IGN/START
    pub eng_mode: i8,
    pub ecam_page: &'static str,

    // Panel 3: Gear, Autobrake, Flaps, Spoilers & Flight Controls
    pub gear_down: bool,
    pub park_brake_on: bool,
    pub brake_fan_on: bool,
    /// Autobrake: 0 = OFF, 1 = LO, 2 = MED, 3 = MAX
    pub autobrake_mode: u8,
    pub flap_ratio: f32,
    pub spdbrk_ratio: f32,
    pub to_config_ok: bool,

    // Panel 4: Exterior Lights, Anti-Ice, ADIRS, Seatbelts, Baro STD & TCAS
    pub strobe_on: bool,
    pub land_lights_on: bool,
    /// Nose Light: 0 = OFF, 1 = TAXI, 2 = T.O
    pub nose_light_mode: u8,
    pub wing_ai_on: bool,
    pub eng1_ai_on: bool,
    pub eng2_ai_on: bool,
    pub probe_heat_on: bool,
    pub nav_light_on: bool,
    pub adirs_aligned: bool,
    pub seatbelt_on: bool,
    pub baro_std: bool,
    pub tcas_tara: bool,

    pub last_rx: Option<Instant>,
}

impl Default for A320FcuState {
    fn default() -> Self {
        Self {
            fd_on: true,
            ap1_on: false,
            ap2_on: false,
            athr_on: false,
            loc_on: false,
            exped_on: false,
            appr_on: false,
            trk_fpa_mode: false,
            master_warn: false,
            master_caut: false,
            hdg_managed: true,
            alt_managed: true,
            spd_managed: true,
            hdg_deg: 254.0,
            alt_ft: 15000.0,
            spd_kts: 250.0,
            vs_fpm: 0.0,
            bat1_on: false,
            bat2_on: false,
            ext_pwr_on: false,
            apu_master_on: false,
            apu_start_on: false,
            apu_avail: false,
            apu_n1_pct: 0.0,
            apu_bleed_on: false,
            apu_gen_on: true,
            fuel_pumps_on: false,
            packs_on: true,
            beacon_on: false,
            eng_mode: 0,
            ecam_page: "APU",
            gear_down: true,
            park_brake_on: true,
            brake_fan_on: false,
            autobrake_mode: 0,
            flap_ratio: 0.0,
            spdbrk_ratio: 0.0,
            to_config_ok: false,
            strobe_on: false,
            land_lights_on: false,
            nose_light_mode: 0,
            wing_ai_on: false,
            eng1_ai_on: false,
            eng2_ai_on: false,
            probe_heat_on: false,
            nav_light_on: true,
            adirs_aligned: true,
            seatbelt_on: true,
            baro_std: false,
            tcas_tara: false,
            last_rx: None,
        }
    }
}

// Panel 1 Dataref IDs
pub const DREF_SIM_FD: i32 = 1;
pub const DREF_SIM_AP1: i32 = 2;
pub const DREF_SIM_AP2: i32 = 3;
pub const DREF_SIM_ATHR: i32 = 4;
pub const DREF_SIM_LOC: i32 = 5;
pub const DREF_SIM_APPR: i32 = 6;
pub const DREF_SIM_HDG: i32 = 7;
pub const DREF_SIM_ALT: i32 = 8;
pub const DREF_SIM_SPD: i32 = 9;
pub const DREF_SIM_VS: i32 = 10;
pub const DREF_SIM_WARN: i32 = 11;
pub const DREF_SIM_CAUT: i32 = 12;
pub const DREF_SIM_TRK: i32 = 13;
pub const DREF_TOLISS_AP1: i32 = 20;
pub const DREF_TOLISS_AP2: i32 = 21;
pub const DREF_TOLISS_ATHR: i32 = 22;
pub const DREF_TOLISS_LOC: i32 = 23;
pub const DREF_TOLISS_APPR: i32 = 24;
pub const DREF_TOLISS_EXPED: i32 = 25;
pub const DREF_A333_HDG_OPEN: i32 = 30;

// Panel 2 Dataref IDs
pub const DREF_APU_N1: i32 = 40;
pub const DREF_APU_MASTER: i32 = 41;
pub const DREF_APU_START: i32 = 42;
pub const DREF_APU_AVAIL: i32 = 43;
pub const DREF_APU_BLEED: i32 = 44;
pub const DREF_BAT1: i32 = 45;
pub const DREF_BAT2: i32 = 46;
pub const DREF_FUEL_PUMP: i32 = 47;
pub const DREF_ENG_MODE: i32 = 48;
pub const DREF_BEACON: i32 = 49;

// Panel 3 Dataref IDs
pub const DREF_GEAR_HANDLE: i32 = 60;
pub const DREF_PARK_BRAKE: i32 = 61;
pub const DREF_FLAP_RATIO: i32 = 62;
pub const DREF_SPDBRK_RATIO: i32 = 63;
pub const DREF_BRAKE_FAN: i32 = 64;

// Panel 4 Dataref IDs
pub const DREF_STROBE: i32 = 70;
pub const DREF_LAND_LT: i32 = 71;
pub const DREF_TAXI_LT: i32 = 72;
pub const DREF_NAV_LT: i32 = 73;
pub const DREF_WING_AI: i32 = 74;
pub const DREF_SEATBELT: i32 = 75;
pub const DREF_BARO_STD: i32 = 76;

pub const UDP_RREF_SUBSCRIPTIONS: &[(i32, &str)] = &[
    (DREF_SIM_FD, "sim/cockpit2/autopilot/flight_director_mode"),
    (DREF_SIM_AP1, "sim/cockpit2/autopilot/servos_on"),
    (DREF_SIM_AP2, "sim/cockpit2/autopilot/servos2_on"),
    (DREF_SIM_ATHR, "sim/cockpit2/autopilot/autothrottle_enabled"),
    (DREF_SIM_LOC, "sim/cockpit2/autopilot/nav_status"),
    (DREF_SIM_APPR, "sim/cockpit2/autopilot/approach_status"),
    (
        DREF_SIM_HDG,
        "sim/cockpit2/autopilot/heading_dial_deg_mag_pilot",
    ),
    (DREF_SIM_ALT, "sim/cockpit2/autopilot/altitude_dial_ft"),
    (DREF_SIM_SPD, "sim/cockpit2/autopilot/airspeed_dial_kts_mach"),
    (DREF_SIM_VS, "sim/cockpit2/autopilot/vvi_dial_fpm"),
    (DREF_SIM_WARN, "sim/cockpit2/annunciators/master_warning"),
    (DREF_SIM_CAUT, "sim/cockpit2/annunciators/master_caution"),
    (DREF_TOLISS_AP1, "AirbusFBW/AP1Engage"),
    (DREF_TOLISS_AP2, "AirbusFBW/AP2Engage"),
    (DREF_TOLISS_ATHR, "AirbusFBW/ATHRmode"),
    (DREF_TOLISS_LOC, "AirbusFBW/LOCilluminated"),
    (DREF_TOLISS_APPR, "AirbusFBW/APPRilluminated"),
    (DREF_TOLISS_EXPED, "AirbusFBW/EXPEDilluminated"),
    (DREF_APU_N1, "sim/cockpit2/electrical/APU_N1_percent"),
    (DREF_GEAR_HANDLE, "sim/cockpit2/controls/gear_handle_down"),
    (DREF_PARK_BRAKE, "sim/cockpit2/controls/parking_brake_ratio"),
    (DREF_FLAP_RATIO, "sim/cockpit2/controls/flap_ratio"),
    (DREF_SPDBRK_RATIO, "sim/cockpit2/controls/speedbrake_ratio"),
];

pub const FCU_PANEL_DATAREFS: &[(i32, &str)] = &[
    (DREF_SIM_FD, "sim/cockpit2/autopilot/flight_director_mode"),
    (DREF_SIM_AP1, "sim/cockpit2/autopilot/servos_on"),
    (DREF_SIM_AP2, "sim/cockpit2/autopilot/servos2_on"),
    (DREF_SIM_ATHR, "sim/cockpit2/autopilot/autothrottle_on"),
    (DREF_SIM_LOC, "sim/cockpit2/autopilot/nav_status"),
    (DREF_SIM_APPR, "sim/cockpit2/autopilot/approach_status"),
    (
        DREF_SIM_HDG,
        "sim/cockpit2/autopilot/heading_dial_deg_mag_pilot",
    ),
    (DREF_SIM_ALT, "sim/cockpit2/autopilot/altitude_dial_ft"),
    (DREF_SIM_SPD, "sim/cockpit2/autopilot/airspeed_dial_kts_mach"),
    (DREF_SIM_VS, "sim/cockpit2/autopilot/vvi_dial_fpm"),
    (DREF_SIM_TRK, "sim/cockpit2/autopilot/trk_fpa"),
    (DREF_SIM_WARN, "sim/cockpit2/annunciators/master_warning"),
    (DREF_SIM_CAUT, "sim/cockpit2/annunciators/master_caution"),
    (DREF_A333_HDG_OPEN, "laminar/A333/autopilot/hdg_window_open"),
];

pub const APU_PANEL_DATAREFS: &[(i32, &str)] = &[
    (DREF_APU_N1, "sim/cockpit2/electrical/APU_N1_percent"),
    (DREF_APU_MASTER, "laminar/A333/annun/apu_master_on"),
    (DREF_APU_START, "laminar/A333/annun/apu_start_on"),
    (DREF_APU_AVAIL, "laminar/A333/annun/apu_avail"),
    (DREF_APU_BLEED, "laminar/A333/annun/apu_bleed_on"),
    (DREF_BAT1, "laminar/A333/buttons/batt1_ctct_on_off"),
    (DREF_BAT2, "laminar/A333/buttons/batt2_ctct_on_off"),
    (DREF_FUEL_PUMP, "laminar/A333/fuel/buttons/left1_pump_pos"),
    (
        DREF_ENG_MODE,
        "sim/cockpit2/engine/actuators/eng_mode_selector",
    ),
    (DREF_BEACON, "sim/cockpit2/switches/beacon_on"),
];

pub const FLT_PANEL_DATAREFS: &[(i32, &str)] = &[
    (DREF_GEAR_HANDLE, "sim/cockpit2/controls/gear_handle_down"),
    (DREF_PARK_BRAKE, "sim/cockpit2/controls/parking_brake_ratio"),
    (DREF_FLAP_RATIO, "sim/cockpit2/controls/flap_ratio"),
    (DREF_SPDBRK_RATIO, "sim/cockpit2/controls/speedbrake_ratio"),
    (
        DREF_BRAKE_FAN,
        "laminar/A333/annun/landing_gear/brake_fan_on",
    ),
];

pub const SYS_PANEL_DATAREFS: &[(i32, &str)] = &[
    (DREF_STROBE, "sim/cockpit2/switches/strobe_lights_on"),
    (DREF_LAND_LT, "sim/cockpit2/switches/landing_lights_on"),
    (DREF_TAXI_LT, "sim/cockpit2/switches/taxi_light_on"),
    (DREF_NAV_LT, "sim/cockpit2/switches/navigation_lights_on"),
    (
        DREF_WING_AI,
        "sim/cockpit2/ice/ice_surface_hot_bleed_air_on",
    ),
    (DREF_SEATBELT, "sim/cockpit2/annunciators/fasten_seatbelt"),
    (DREF_BARO_STD, "laminar/A333/baro/capt_std_pos"),
];

/// Returns the list of WebAPI datarefs to poll for the given active sub-panel.
pub fn datarefs_for_panel(active_panel: &str) -> &'static [(i32, &'static str)] {
    match active_panel.trim().to_ascii_lowercase().as_str() {
        "apu" => APU_PANEL_DATAREFS,
        "flt" => FLT_PANEL_DATAREFS,
        "sys" => SYS_PANEL_DATAREFS,
        _ => FCU_PANEL_DATAREFS,
    }
}

/// Applies an incoming dataref value update to `state`, using `prev_vals` to edge-trigger
/// boolean indicators so local optimistic toggles aren't overwritten by stale values.
pub fn apply_dref_update(
    state: &mut A320FcuState,
    prev_vals: &mut HashMap<i32, f32>,
    id: i32,
    val: f32,
) {
    let changed = prev_vals
        .insert(id, val)
        .is_none_or(|old| (old - val).abs() > 0.001);

    match id {
        DREF_SIM_FD if changed => state.fd_on = val >= 1.0,
        DREF_SIM_AP1 | DREF_TOLISS_AP1 if changed => state.ap1_on = val > 0.5,
        DREF_SIM_AP2 | DREF_TOLISS_AP2 if changed => state.ap2_on = val > 0.5,
        DREF_SIM_ATHR | DREF_TOLISS_ATHR if changed => state.athr_on = val > 0.5,
        DREF_SIM_LOC | DREF_TOLISS_LOC if changed => state.loc_on = val >= 1.0,
        DREF_SIM_APPR | DREF_TOLISS_APPR if changed => state.appr_on = val >= 1.0,
        DREF_TOLISS_EXPED if changed => state.exped_on = val > 0.5,
        DREF_SIM_TRK if changed => state.trk_fpa_mode = val > 0.5,
        DREF_A333_HDG_OPEN if changed => state.hdg_managed = val < 0.5,
        DREF_SIM_HDG => state.hdg_deg = val.rem_euclid(360.0),
        DREF_SIM_ALT => state.alt_ft = val,
        DREF_SIM_SPD => state.spd_kts = val,
        DREF_SIM_VS => state.vs_fpm = val,
        DREF_SIM_WARN if changed => state.master_warn = val > 0.5,
        DREF_SIM_CAUT if changed => state.master_caut = val > 0.5,
        // Panel 2 (APU / Overhead / Eng Mode)
        DREF_APU_N1 => {
            state.apu_n1_pct = val.clamp(0.0, 100.0);
            if state.apu_n1_pct >= 95.0 {
                state.apu_avail = true;
                state.apu_start_on = false;
            }
        }
        DREF_APU_MASTER if changed => state.apu_master_on = val > 0.1,
        DREF_APU_START if changed => state.apu_start_on = val > 0.1,
        DREF_APU_AVAIL if changed => state.apu_avail = val > 0.1,
        DREF_APU_BLEED if changed => state.apu_bleed_on = val > 0.1,
        DREF_BAT1 if changed => state.bat1_on = val > 0.5,
        DREF_BAT2 if changed => state.bat2_on = val > 0.5,
        DREF_FUEL_PUMP if changed => state.fuel_pumps_on = val > 0.5,
        DREF_ENG_MODE if changed => state.eng_mode = val.round() as i8,
        DREF_BEACON if changed => state.beacon_on = val > 0.5,
        // Panel 3 (Gear, Brakes, Flaps, Speedbrakes)
        DREF_GEAR_HANDLE if changed => state.gear_down = val > 0.5,
        DREF_PARK_BRAKE if changed => state.park_brake_on = val > 0.5,
        DREF_FLAP_RATIO => state.flap_ratio = val.clamp(0.0, 1.0),
        DREF_SPDBRK_RATIO => state.spdbrk_ratio = val.clamp(-0.5, 1.0),
        DREF_BRAKE_FAN if changed => state.brake_fan_on = val > 0.1,
        // Panel 4 (Lights, Anti-Ice, ADIRS, Seatbelts, Baro STD)
        DREF_STROBE if changed => state.strobe_on = val > 0.5,
        DREF_LAND_LT if changed => state.land_lights_on = val > 0.5,
        DREF_TAXI_LT if changed => {
            state.nose_light_mode = if val > 0.5 { 1 } else { 0 };
        }
        DREF_NAV_LT if changed => state.nav_light_on = val > 0.5,
        DREF_WING_AI if changed => state.wing_ai_on = val > 0.5,
        DREF_SEATBELT if changed => state.seatbelt_on = val > 0.5,
        DREF_BARO_STD if changed => state.baro_std = val > 0.5,
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_dref_updates_and_edge_triggering() {
        let mut state = A320FcuState::default();
        let mut prev = HashMap::new();

        // Initial update sets AP1 off
        apply_dref_update(&mut state, &mut prev, DREF_SIM_AP1, 0.0);
        assert!(!state.ap1_on);

        // Optimistic toggle turns AP1 on; unchanged 0.0 poll does not overwrite it
        state.ap1_on = true;
        apply_dref_update(&mut state, &mut prev, DREF_SIM_AP1, 0.0);
        assert!(state.ap1_on);

        // Actual transition to 1.0 then 0.0 updates state
        apply_dref_update(&mut state, &mut prev, DREF_SIM_AP1, 1.0);
        assert!(state.ap1_on);
        apply_dref_update(&mut state, &mut prev, DREF_SIM_AP1, 0.0);
        assert!(!state.ap1_on);

        // APU N1 reaching 98% automatically marks APU AVAIL
        apply_dref_update(&mut state, &mut prev, DREF_APU_N1, 98.5);
        assert!(state.apu_avail);
        assert!(!state.apu_start_on);
    }
}
