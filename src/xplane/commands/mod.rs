mod apu;
mod fcu;
mod flt;
mod sys;

use super::state::A320FcuState;
use apu::trigger_apu_control;
use fcu::trigger_fcu_control;
use flt::trigger_flt_control;
use sys::trigger_sys_control;

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
    if let Some(cmd) = custom_cmd
        && !cmd.trim().is_empty()
    {
        for single in cmd.split(';') {
            let trimmed = single.trim();
            if !trimmed.is_empty() {
                send_cmd(trimmed);
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
