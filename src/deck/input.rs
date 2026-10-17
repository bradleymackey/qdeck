use crate::config::{ClickAction, QDeckConfig};
use crate::telemetry::{request_ai_limits_refresh, toggle_gemini_view};
use crate::xplane::XPlaneClient;
use std::process::Command;
use std::time::{Duration, Instant};

pub(super) const TAP_HIGHLIGHT_DURATION: Duration = Duration::from_millis(120);

pub(super) struct ButtonEventOutcome {
    pub newly_highlighted_keys: Vec<u8>,
    pub mode_or_panel_switched: bool,
    pub fcu_changed: bool,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn process_button_edges(
    prev_buttons: &[u8],
    buttons: &[u8],
    highlight_until: &mut [Option<Instant>],
    cfg: &QDeckConfig,
    cols: u8,
    total_keys: u8,
    modes: &[String],
    mode_idx: &mut usize,
    panel_idx: &mut usize,
    xplane: &mut XPlaneClient,
    demo_until: &mut Option<Instant>,
) -> ButtonEventOutcome {
    let mut newly_highlighted_keys = Vec::new();
    let mut mode_or_panel_switched = false;
    let mut fcu_changed = false;
    let press_time = Instant::now();

    for (k_idx, (&prev, &curr)) in prev_buttons.iter().zip(buttons.iter()).enumerate() {
        let key_u8 = k_idx as u8;
        if prev == 0 && curr != 0 {
            highlight_until[k_idx] = Some(press_time + TAP_HIGHLIGHT_DURATION);
            newly_highlighted_keys.push(key_u8);
            let active_mode = &modes[*mode_idx];
            let panels = cfg.effective_panels(active_mode);
            let total_panels = panels.len().max(1);
            *panel_idx %= total_panels;
            let active_panel = &panels[*panel_idx];

            if let Some(tile) =
                cfg.tile_for_key(key_u8, cols, total_keys, active_mode, active_panel)
            {
                let norm_type = tile.tile_type.trim().to_ascii_lowercase();
                if matches!(norm_type.as_str(), "gemini" | "gemini_limits") {
                    let view = toggle_gemini_view();
                    println!(
                        "[QDECK] Gemini limits view toggled -> {}/2 ({})",
                        view + 1,
                        if view == 0 { "ARG + FLS" } else { "CLD + GPT" }
                    );
                    request_ai_limits_refresh();
                } else if matches!(norm_type.as_str(), "claude" | "claude_limits") {
                    request_ai_limits_refresh();
                }

                match tile.click_action() {
                    ClickAction::CycleMode => {
                        *mode_idx = (*mode_idx + 1) % modes.len().max(1);
                        println!("[QDECK] Mode toggled -> '{}'", modes[*mode_idx]);
                        mode_or_panel_switched = true;
                    }
                    ClickAction::CyclePanel => {
                        *panel_idx = (*panel_idx + 1) % total_panels;
                        println!(
                            "[QDECK] Panel toggled -> '{}/{}'",
                            active_mode, panels[*panel_idx]
                        );
                        mode_or_panel_switched = true;
                    }
                    ClickAction::XPlaneControl {
                        control,
                        custom_cmd,
                    } => {
                        println!(
                            "[QDECK] [X-Plane] Triggered control '{}' (panel='{}')",
                            control, active_panel
                        );
                        xplane.trigger_a320_control(&control, custom_cmd.as_deref());
                        fcu_changed = true;
                    }
                    ClickAction::OpenApp(app_name) => {
                        println!("[QDECK] Click -> Opening macOS application: '{}'", app_name);
                        if let Err(e) = Command::new("open").arg("-a").arg(&app_name).spawn() {
                            eprintln!("[QDECK] Failed to open app '{}': {}", app_name, e);
                        }
                    }
                    ClickAction::ShellCommand(cmd) => {
                        println!("[QDECK] Click -> Executing command: '{}'", cmd);
                        if let Err(e) = Command::new("/bin/sh").arg("-c").arg(&cmd).spawn() {
                            eprintln!("[QDECK] Failed to run command '{}': {}", cmd, e);
                        }
                    }
                    ClickAction::TriggerDemoSpike => {
                        println!(
                            "[QDECK] Click -> Triggering 4s simulated Amber/Red load spike demo!"
                        );
                        *demo_until = Some(Instant::now() + Duration::from_secs(4));
                    }
                    ClickAction::None => {}
                }
            }
        }
    }

    ButtonEventOutcome {
        newly_highlighted_keys,
        mode_or_panel_switched,
        fcu_changed,
    }
}

pub(super) fn collect_expired_highlights(
    highlight_until: &mut [Option<Instant>],
    now: Instant,
) -> Vec<u8> {
    let mut expired = Vec::new();
    for (k_idx, slot) in highlight_until.iter_mut().enumerate() {
        if let Some(deadline) = *slot
            && now >= deadline
        {
            *slot = None;
            expired.push(k_idx as u8);
        }
    }
    expired
}
