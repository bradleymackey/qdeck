pub mod datarefs;

pub use datarefs::*;

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
    /// True when all 3 landing gear struts are down and locked (`>= 0.98`).
    pub gear_down: bool,
    /// Cockpit landing gear lever position (`true` = DOWN, `false` = UP).
    pub gear_handle_down: bool,
    /// Actual physical gear deployment ratio `[nose, left_main, right_main]` in `0.0..=1.0`.
    pub gear_deploy: [f32; 3],
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

impl A320FcuState {
    /// Returns true if any landing gear strut is currently unlocked / in transit (`0.02 < r < 0.98`).
    pub fn is_gear_in_transit(&self) -> bool {
        self.gear_deploy.iter().any(|&r| r > 0.02 && r < 0.98)
    }

    /// Recomputes `gear_down` from the 3 physical gear strut deployment ratios.
    pub fn sync_gear_down_from_deploy(&mut self) {
        self.gear_down = self.gear_deploy.iter().all(|&r| r >= 0.98);
    }

    /// Sets all 3 physical gear struts to either fully down (`1.0`) or fully retracted (`0.0`).
    pub fn set_all_gear_deployed(&mut self, down: bool) {
        let v = if down { 1.0 } else { 0.0 };
        self.gear_deploy = [v, v, v];
        self.gear_down = down;
        self.gear_handle_down = down;
    }
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
            gear_handle_down: true,
            gear_deploy: [1.0, 1.0, 1.0],
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

/// Applies an incoming array dataref value update (e.g., `sim/flightmodel2/gear/deploy_ratio`)
/// to `state`.
pub fn apply_dref_array_update(state: &mut A320FcuState, id: i32, vals: &[f32]) {
    if id == DREF_GEAR_DEPLOY && !vals.is_empty() {
        let nose = vals[0].clamp(0.0, 1.0);
        let left = vals.get(1).copied().unwrap_or(nose).clamp(0.0, 1.0);
        let right = vals.get(2).copied().unwrap_or(left).clamp(0.0, 1.0);
        state.gear_deploy = [nose, left, right];
        state.sync_gear_down_from_deploy();
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
        DREF_GEAR_HANDLE => state.gear_handle_down = val > 0.5,
        DREF_GEAR_DEPLOY => {
            state.gear_deploy[0] = val.clamp(0.0, 1.0);
            state.sync_gear_down_from_deploy();
        }
        DREF_GEAR_LEFT_DEPLOY => {
            state.gear_deploy[1] = val.clamp(0.0, 1.0);
            state.sync_gear_down_from_deploy();
        }
        DREF_GEAR_RIGHT_DEPLOY => {
            state.gear_deploy[2] = val.clamp(0.0, 1.0);
            state.sync_gear_down_from_deploy();
        }
        DREF_PARK_BRAKE if changed => state.park_brake_on = val > 0.5,
        DREF_FLAP_RATIO if changed => state.flap_ratio = val.clamp(0.0, 1.0),
        DREF_SPDBRK_RATIO if changed => state.spdbrk_ratio = val.clamp(-0.5, 1.0),
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

        // Actual landing gear deployment array: in transit (UNLK), retracted (UP), and locked (DOWN)
        apply_dref_array_update(&mut state, DREF_GEAR_DEPLOY, &[0.45, 0.50, 0.50]);
        assert!(state.is_gear_in_transit());
        assert!(!state.gear_down);

        apply_dref_array_update(&mut state, DREF_GEAR_DEPLOY, &[0.0, 0.0, 0.0]);
        assert!(!state.is_gear_in_transit());
        assert!(!state.gear_down);

        apply_dref_array_update(&mut state, DREF_GEAR_DEPLOY, &[1.0, 1.0, 1.0]);
        assert!(!state.is_gear_in_transit());
        assert!(state.gear_down);
    }
}
