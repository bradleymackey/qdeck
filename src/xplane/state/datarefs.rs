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
pub const DREF_GEAR_DEPLOY: i32 = 65;
pub const DREF_GEAR_LEFT_DEPLOY: i32 = 66;
pub const DREF_GEAR_RIGHT_DEPLOY: i32 = 67;

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
    (
        DREF_SIM_SPD,
        "sim/cockpit2/autopilot/airspeed_dial_kts_mach",
    ),
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
    (DREF_GEAR_DEPLOY, "sim/flightmodel2/gear/deploy_ratio[0]"),
    (
        DREF_GEAR_LEFT_DEPLOY,
        "sim/flightmodel2/gear/deploy_ratio[1]",
    ),
    (
        DREF_GEAR_RIGHT_DEPLOY,
        "sim/flightmodel2/gear/deploy_ratio[2]",
    ),
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
    (
        DREF_SIM_SPD,
        "sim/cockpit2/autopilot/airspeed_dial_kts_mach",
    ),
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
    (DREF_GEAR_DEPLOY, "sim/flightmodel2/gear/deploy_ratio"),
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
