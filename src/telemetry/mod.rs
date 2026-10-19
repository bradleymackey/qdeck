pub mod ai_limits;
pub mod airport;

pub use ai_limits::{
    AiLimitsState, ClaudeLimits, GeminiLimits, QuotaWindowPair, current_ai_limits,
    fetch_claude_limits, fetch_gemini_limits, parse_claude_usage_json,
    parse_gemini_quota_summary_json, request_ai_limits_refresh, toggle_gemini_view,
};
pub use airport::{
    AirportFlight, AirportStatus, DEFAULT_AIRPORT_ICAO, RunwayInUse, RunwaySource, Wind,
    current_airport_status, note_airport_tile_active, toggle_airport_page,
};

use crate::widgets::TelemetrySnapshot;
use crate::xplane::A320FcuState;
use std::process::Command;
use sysinfo::System;

/// Parses `ioreg -r -d 1 -w 0 -c IOAccelerator` output for `(gpu_cores, utilization_pct)`.
pub fn parse_ioreg_gpu_output(text: &str, fallback_cores: usize) -> (usize, f32) {
    let mut gpu_cores = fallback_cores;
    let mut max_util: f32 = 0.0;

    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("\"gpu-core-count\" =")
            && let Ok(c) = rest.trim().parse::<usize>()
            && c > 0
        {
            gpu_cores = c;
        }
        if trimmed.starts_with("\"PerformanceStatistics\"") {
            for key in [
                "\"Device Utilization %\"=",
                "\"Renderer Utilization %\"=",
                "\"Tiler Utilization %\"=",
            ] {
                if let Some(pos) = trimmed.find(key) {
                    let after = &trimmed[pos + key.len()..];
                    let digits: String = after
                        .chars()
                        .take_while(|c| c.is_ascii_digit() || *c == '.')
                        .collect();
                    if let Ok(val) = digits.parse::<f32>() {
                        max_util = max_util.max(val);
                    }
                }
            }
        }
    }

    (gpu_cores, max_util.clamp(0.0, 100.0))
}

/// Queries macOS IOKit `IOAccelerator` for GPU core count and live utilization percentage.
pub fn query_macos_gpu_stats(fallback_cores: usize) -> (usize, f32) {
    let Ok(output) = Command::new("ioreg")
        .args(["-r", "-d", "1", "-w", "0", "-c", "IOAccelerator"])
        .output()
    else {
        return (fallback_cores, 0.0);
    };

    let text = String::from_utf8_lossy(&output.stdout);
    parse_ioreg_gpu_output(&text, fallback_cores)
}

/// Returns `true` if the given tile type displays dynamic telemetry that should be redrawn on each tick.
pub fn is_dynamic_telemetry_tile(tile_type: &str) -> bool {
    matches!(
        tile_type.trim().to_ascii_lowercase().as_str(),
        "cpu"
            | "gpu"
            | "memory"
            | "ram"
            | "claude"
            | "claude_limits"
            | "gemini"
            | "gemini_limits"
            | "airport"
            | "ops_status"
            | "status"
            | "a320_fcu"
            | "xplane"
            | "fcu"
            | "a320"
    )
}

#[allow(clippy::too_many_arguments)]
pub fn build_snapshot<'a>(
    sys: &System,
    core_buf: &'a mut Vec<f32>,
    gpu_cores: usize,
    live_gpu_pct: f32,
    claude_limits: QuotaWindowPair,
    gemini_limits: GeminiLimits,
    airport: AirportStatus,
    active_mode: &'a str,
    next_mode: &'a str,
    mode_index: usize,
    total_modes: usize,
    active_panel: &'a str,
    next_panel: &'a str,
    panel_index: usize,
    total_panels: usize,
    fcu: &'a A320FcuState,
    tick: u64,
    in_demo: bool,
) -> TelemetrySnapshot<'a> {
    let num_cores = sys.cpus().len().max(1);
    core_buf.clear();
    let (total_cpu_pct, gpu_pct) = if in_demo {
        let phase = (tick as f32) * 0.65;
        for c in 0..num_cores {
            let c_ratio = c as f32 / (num_cores.max(1) as f32);
            let wave = ((phase + c_ratio * 4.2).sin() * 0.5 + 0.5).powf(0.85);
            let pct = (18.0 + wave * 80.0).clamp(5.0, 98.0);
            core_buf.push(pct);
        }
        let avg_cpu = core_buf.iter().sum::<f32>() / (core_buf.len() as f32);
        let sim_gpu = (15.0 + ((phase * 0.85).sin() * 0.5 + 0.5) * 82.0).clamp(10.0, 98.0);
        (avg_cpu, sim_gpu)
    } else {
        for cpu in sys.cpus() {
            core_buf.push(cpu.cpu_usage());
        }
        (sys.global_cpu_usage(), live_gpu_pct)
    };

    let total_mem = sys.total_memory().max(1) as f32;
    let used_mem = sys.used_memory() as f32;
    let mem_used_pct = (used_mem / total_mem) * 100.0;
    let mem_used_gb = used_mem / 1_073_741_824.0;
    let mem_total_gb = total_mem / 1_073_741_824.0;
    let total_swap = sys.total_swap() as f32;
    let swap_used_pct = if total_swap > 0.0 {
        (sys.used_swap() as f32 / total_swap) * 100.0
    } else {
        0.0
    };

    TelemetrySnapshot {
        total_cpu_pct,
        core_usages: core_buf.as_slice(),
        gpu_pct,
        gpu_cores,
        mem_used_pct,
        mem_used_gb,
        mem_total_gb,
        swap_used_pct,
        claude_limits,
        gemini_limits,
        airport,
        active_mode,
        next_mode,
        mode_index,
        total_modes,
        active_panel,
        next_panel,
        panel_index,
        total_panels,
        fcu,
        tick,
        demo_mode_active: in_demo,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ioreg_gpu_output() {
        let sample = r#"
        +-o AGXAcceleratorG16X  <class AGXAcceleratorG16X>
          {
            "gpu-core-count" = 40
            "PerformanceStatistics" = {"Device Utilization %"=34,"Renderer Utilization %"=29,"Tiler Utilization %"=12}
          }
        "#;
        let (cores, util) = parse_ioreg_gpu_output(sample, 16);
        assert_eq!(cores, 40);
        assert_eq!(util, 34.0);
    }

    #[test]
    fn test_is_dynamic_telemetry_tile() {
        assert!(is_dynamic_telemetry_tile("cpu"));
        assert!(is_dynamic_telemetry_tile("gpu"));
        assert!(is_dynamic_telemetry_tile("claude"));
        assert!(is_dynamic_telemetry_tile("gemini"));
        assert!(is_dynamic_telemetry_tile("airport"));
        assert!(is_dynamic_telemetry_tile("a320_fcu"));
        assert!(!is_dynamic_telemetry_tile("mode"));
        assert!(!is_dynamic_telemetry_tile("panel"));
    }
}
