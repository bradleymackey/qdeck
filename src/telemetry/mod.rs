pub mod ai_limits;
pub mod airport;
pub mod audio;
pub mod disk;

pub use ai_limits::{
    AiLimitsState, ClaudeLimits, GeminiLimits, QuotaWindowPair, current_ai_limits,
    fetch_claude_limits, fetch_gemini_limits, parse_claude_usage_json,
    parse_gemini_quota_summary_json, request_ai_limits_refresh, toggle_gemini_view,
};
pub use airport::{
    AirportFlight, AirportStatus, DEFAULT_AIRPORT_ICAO, RunwayInUse, RunwaySource, Wind,
    current_airport_status, note_airport_tile_active, toggle_airport_page,
};
pub use audio::{
    AudioDevices, AudioEndpoint, format_audio_device_short_name, parse_spaudio_json,
    query_macos_audio_devices,
};
pub use disk::{
    DiskVolume, DisksState, current_disk_volume_index, format_compact_bytes,
    format_volume_short_name, parse_df_k_output, query_macos_disks, toggle_disk_volume,
};

use crate::widgets::TelemetrySnapshot;
use crate::xplane::A320FcuState;
use std::process::Command;
use sysinfo::System;

/// Live Apple GPU telemetry parsed from IOKit `IOAccelerator`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct GpuStats {
    /// Number of GPU cores (`gpu-core-count`).
    pub cores: usize,
    /// Peak of Device / Renderer / Tiler utilization (`0.0..=100.0`).
    pub util_pct: f32,
    /// GPU memory currently in use (`In use system memory`), in bytes.
    pub mem_used_bytes: u64,
    /// GPU memory currently allocated (`Alloc system memory`), in bytes.
    pub mem_alloc_bytes: u64,
}

/// Parses `ioreg -r -d 1 -w 0 -c IOAccelerator` output into [`GpuStats`].
pub fn parse_ioreg_gpu_output(text: &str, fallback_cores: usize) -> GpuStats {
    let mut stats = GpuStats {
        cores: fallback_cores,
        ..GpuStats::default()
    };

    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("\"gpu-core-count\" =")
            && let Ok(c) = rest.trim().parse::<usize>()
            && c > 0
        {
            stats.cores = c;
        }
        if trimmed.starts_with("\"PerformanceStatistics\"") {
            for key in [
                "\"Device Utilization %\"=",
                "\"Renderer Utilization %\"=",
                "\"Tiler Utilization %\"=",
            ] {
                if let Some(val) = extract_ioreg_number(trimmed, key) {
                    stats.util_pct = stats.util_pct.max(val as f32);
                }
            }
            if let Some(v) = extract_ioreg_number(trimmed, "\"In use system memory\"=") {
                stats.mem_used_bytes = v as u64;
            }
            if let Some(v) = extract_ioreg_number(trimmed, "\"Alloc system memory\"=") {
                stats.mem_alloc_bytes = v as u64;
            }
        }
    }

    stats.util_pct = stats.util_pct.clamp(0.0, 100.0);
    stats
}

fn extract_ioreg_number(text: &str, key: &str) -> Option<f64> {
    let pos = text.find(key)?;
    let digits: String = text[pos + key.len()..]
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    digits.parse::<f64>().ok()
}

/// Queries macOS IOKit `IOAccelerator` for GPU core count, live utilization and GPU memory.
pub fn query_macos_gpu_stats(fallback_cores: usize) -> GpuStats {
    let Ok(output) = Command::new("ioreg")
        .args(["-r", "-d", "1", "-w", "0", "-c", "IOAccelerator"])
        .output()
    else {
        return GpuStats {
            cores: fallback_cores,
            ..GpuStats::default()
        };
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
            | "disk"
            | "storage"
            | "volume"
            | "claude"
            | "claude_limits"
            | "gemini"
            | "gemini_limits"
            | "airport"
            | "audio"
            | "audio_io"
            | "sound"
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
    gpu: GpuStats,
    claude_limits: QuotaWindowPair,
    gemini_limits: GeminiLimits,
    airport: AirportStatus,
    audio_devices: &'a AudioDevices,
    disks: &'a DisksState,
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
        (sys.global_cpu_usage(), gpu.util_pct)
    };
    let load_avg_1m = System::load_average().one as f32;

    let total_mem = sys.total_memory().max(1) as f32;
    let used_mem = sys.used_memory() as f32;
    let mem_used_pct = (used_mem / total_mem) * 100.0;
    let mem_used_gb = used_mem / 1_073_741_824.0;
    let mem_total_gb = total_mem / 1_073_741_824.0;
    let swap_total_bytes = sys.total_swap();
    let swap_used_bytes = sys.used_swap().min(swap_total_bytes);
    let total_swap = swap_total_bytes as f32;
    let swap_used_pct = if total_swap > 0.0 {
        (swap_used_bytes as f32 / total_swap) * 100.0
    } else {
        0.0
    };

    TelemetrySnapshot {
        total_cpu_pct,
        load_avg_1m,
        core_usages: core_buf.as_slice(),
        gpu_pct,
        gpu_cores: gpu.cores,
        gpu_mem_used_bytes: gpu.mem_used_bytes,
        gpu_mem_alloc_bytes: gpu.mem_alloc_bytes,
        mem_used_pct,
        mem_used_gb,
        mem_total_gb,
        swap_used_pct,
        swap_used_bytes,
        swap_total_bytes,
        claude_limits,
        gemini_limits,
        airport,
        audio_devices,
        disks,
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
            "PerformanceStatistics" = {"Device Utilization %"=34,"Renderer Utilization %"=29,"Tiler Utilization %"=12,"In use system memory"=1618493440,"Alloc system memory"=16684777472}
          }
        "#;
        let stats = parse_ioreg_gpu_output(sample, 16);
        assert_eq!(stats.cores, 40);
        assert_eq!(stats.util_pct, 34.0);
        assert_eq!(stats.mem_used_bytes, 1_618_493_440);
        assert_eq!(stats.mem_alloc_bytes, 16_684_777_472);
    }

    #[test]
    fn test_is_dynamic_telemetry_tile() {
        assert!(is_dynamic_telemetry_tile("cpu"));
        assert!(is_dynamic_telemetry_tile("gpu"));
        assert!(is_dynamic_telemetry_tile("disk"));
        assert!(is_dynamic_telemetry_tile("claude"));
        assert!(is_dynamic_telemetry_tile("gemini"));
        assert!(is_dynamic_telemetry_tile("airport"));
        assert!(is_dynamic_telemetry_tile("audio"));
        assert!(is_dynamic_telemetry_tile("a320_fcu"));
        assert!(!is_dynamic_telemetry_tile("mode"));
        assert!(!is_dynamic_telemetry_tile("panel"));
    }
}
