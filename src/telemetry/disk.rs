use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static DISK_VOLUME_INDEX: AtomicUsize = AtomicUsize::new(0);

/// Advances the active volume index displayed on the Disk tile and returns the new raw index.
pub fn toggle_disk_volume() -> usize {
    DISK_VOLUME_INDEX
        .fetch_add(1, Ordering::Relaxed)
        .wrapping_add(1)
}

/// Returns the current raw volume index counter.
pub fn current_disk_volume_index() -> usize {
    DISK_VOLUME_INDEX.load(Ordering::Relaxed)
}

/// Telemetry for a single mounted user volume (e.g. `Macintosh HD` or `/Volumes/BANK-1`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DiskVolume {
    /// Full volume name (e.g. `"Macintosh HD"`, `"BANK-1"`).
    pub raw_name: String,
    /// Compact uppercase name (`<= 7` chars) for the 72x72 tile row (e.g. `"MAC HD"`, `"BANK-1"`).
    pub short_name: String,
    /// Mount point path (`"/"` or `"/Volumes/..."`).
    pub mount_point: String,
    /// Total capacity in bytes.
    pub total_bytes: u64,
    /// Used space in bytes (`total_bytes - avail_bytes`).
    pub used_bytes: u64,
    /// Available space in bytes.
    pub avail_bytes: u64,
    /// Used capacity percentage (`0.0..=100.0`).
    pub used_pct: f32,
}

/// Snapshot of all mounted user volumes and the currently selected volume index.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DisksState {
    pub volumes: Vec<DiskVolume>,
    pub active_index: usize,
    pub available: bool,
}

impl DisksState {
    /// Returns `(active_volume, 0-based_index, total_volumes)` taking into account the live toggle counter.
    pub fn active_volume(&self) -> Option<(&DiskVolume, usize, usize)> {
        if self.volumes.is_empty() {
            return None;
        }
        let total = self.volumes.len();
        let idx = current_disk_volume_index() % total;
        Some((&self.volumes[idx], idx, total))
    }
}

/// Queries mounted user volumes via `df -k` and `/Volumes`.
pub fn query_macos_disks() -> DisksState {
    let boot_name = detect_boot_volume_name().unwrap_or_else(|| "Macintosh HD".to_string());
    let Ok(output) = Command::new("df").arg("-k").output() else {
        return DisksState::default();
    };
    if !output.status.success() {
        return DisksState::default();
    }
    let text = String::from_utf8_lossy(&output.stdout);
    parse_df_k_output(&text, &boot_name)
}

fn detect_boot_volume_name() -> Option<String> {
    let entries = fs::read_dir("/Volumes").ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if let Ok(target) = fs::read_link(&path)
            && target == Path::new("/")
            && let Some(name) = path.file_name().and_then(|n| n.to_str())
            && !name.is_empty()
        {
            return Some(name.to_string());
        }
    }
    None
}

/// Parses `df -k` output into user-facing `DisksState` (boot volume + `/Volumes/<External>`).
pub fn parse_df_k_output(df_text: &str, boot_volume_name: &str) -> DisksState {
    let mut boot_vol: Option<DiskVolume> = None;
    let mut external_vols: Vec<DiskVolume> = Vec::new();

    for line in df_text.lines().skip(1) {
        let trimmed = line.trim();
        if !trimmed.starts_with("/dev/disk") {
            continue;
        }

        // Mount point is after the last `%` column (the `%iused` column, or `Capacity` if no inodes)
        let Some((before_mount, mount_raw)) = split_df_line_mount(trimmed) else {
            continue;
        };
        let mount_point = mount_raw.trim();

        let cols: Vec<&str> = before_mount.split_whitespace().collect();
        if cols.len() < 4 {
            continue;
        }

        let Ok(blocks_1k) = cols[1].parse::<u64>() else {
            continue;
        };
        let Ok(avail_1k) = cols[3].parse::<u64>() else {
            continue;
        };
        if blocks_1k == 0 {
            continue;
        }

        let total_bytes = blocks_1k.saturating_mul(1024);
        let avail_bytes = avail_1k.saturating_mul(1024).min(total_bytes);
        let used_bytes = total_bytes.saturating_sub(avail_bytes);
        let used_pct = ((used_bytes as f64 / total_bytes as f64) * 100.0) as f32;

        if mount_point == "/" || mount_point == "/System/Volumes/Data" {
            // Prefer /System/Volumes/Data if both appear,though both share container blocks/avail
            if boot_vol.is_none() || mount_point == "/System/Volumes/Data" {
                boot_vol = Some(DiskVolume {
                    raw_name: boot_volume_name.to_string(),
                    short_name: format_volume_short_name(boot_volume_name),
                    mount_point: "/".to_string(),
                    total_bytes,
                    used_bytes,
                    avail_bytes,
                    used_pct: used_pct.clamp(0.0, 100.0),
                });
            }
        } else if let Some(vol_name) = mount_point.strip_prefix("/Volumes/") {
            // Only top-level volumes under /Volumes/<Name>, excluding Recovery and hidden dot-mounts
            if !vol_name.is_empty()
                && !vol_name.contains('/')
                && !vol_name.starts_with('.')
                && !vol_name.eq_ignore_ascii_case("Recovery")
                && !external_vols.iter().any(|v| v.raw_name == vol_name)
            {
                external_vols.push(DiskVolume {
                    raw_name: vol_name.to_string(),
                    short_name: format_volume_short_name(vol_name),
                    mount_point: mount_point.to_string(),
                    total_bytes,
                    used_bytes,
                    avail_bytes,
                    used_pct: used_pct.clamp(0.0, 100.0),
                });
            }
        }
    }

    let mut volumes = Vec::new();
    if let Some(b) = boot_vol {
        volumes.push(b);
    }
    volumes.extend(external_vols);

    let total = volumes.len();
    let active_index = if total > 0 {
        current_disk_volume_index() % total
    } else {
        0
    };

    DisksState {
        available: !volumes.is_empty(),
        volumes,
        active_index,
    }
}

fn split_df_line_mount(line: &str) -> Option<(&str, &str)> {
    // In macOS `df -k`, the mount point starts after `%iused` (the last `%` token before `/` or `/Volumes`)
    let slash_pos = line.find(" /")?;
    Some((&line[..slash_pos], &line[slash_pos + 1..]))
}

/// Formats a volume name into a compact uppercase string (`<= 7` chars) so it fits next to `100%` on a 72x72 tile.
pub fn format_volume_short_name(raw_name: &str) -> String {
    let clean = raw_name.trim().to_ascii_uppercase();
    if clean.is_empty() {
        return "DISK".to_string();
    }
    if clean == "MACINTOSH HD" || clean == "MACINTOSH" {
        return "MAC HD".to_string();
    }
    if clean.len() <= 7 {
        return clean;
    }

    let abbreviated = clean
        .replace("MACINTOSH ", "MAC ")
        .replace("EXTERNAL ", "EXT ")
        .replace("VOLUME ", "VOL ")
        .replace("BACKUP", "BKUP");

    let compact = abbreviated.split_whitespace().collect::<Vec<_>>().join(" ");
    compact
        .chars()
        .take(7)
        .collect::<String>()
        .trim_end()
        .to_string()
}

/// Formats a byte count into a compact 2..=4 character decimal SI string (e.g. `"867G"`, `"1.1T"`, `"512M"`).
pub fn format_compact_bytes(bytes: u64) -> String {
    const MB: f64 = 1_000_000.0;
    const GB: f64 = 1_000_000_000.0;
    const TB: f64 = 1_000_000_000_000.0;

    let b = bytes as f64;
    if b >= TB {
        let tb = b / TB;
        if tb >= 9.95 {
            format!("{:.0}T", tb.min(999.0))
        } else {
            format!("{:.1}T", tb)
        }
    } else if b >= GB {
        let gb = (b / GB).round();
        if gb >= 1000.0 {
            "1.0T".to_string()
        } else {
            format!("{:.0}G", gb.max(1.0))
        }
    } else if b >= MB {
        let mb = (b / MB).round();
        format!("{:.0}M", mb.clamp(1.0, 999.0))
    } else {
        "0G".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_volume_short_name() {
        assert_eq!(format_volume_short_name("Macintosh HD"), "MAC HD");
        assert_eq!(format_volume_short_name("BANK-1"), "BANK-1");
        assert_eq!(format_volume_short_name("TimeMachine"), "TIMEMAC");
    }

    #[test]
    fn test_format_compact_bytes() {
        assert_eq!(format_compact_bytes(867_116_941_312), "867G");
        assert_eq!(format_compact_bytes(938_950_377_472), "939G");
        assert_eq!(format_compact_bytes(1_128_048_795_648), "1.1T");
        assert_eq!(format_compact_bytes(2_000_189_177_856), "2.0T");
        assert_eq!(format_compact_bytes(512_000_000), "512M");
    }

    #[test]
    fn test_parse_df_k_output() {
        let sample = r#"Filesystem 1024-blocks Used Available Capacity iused ifree %iused Mounted on
/dev/disk3s1s1 1948404040 13334320 1101610152 2% 484019 4294062940 0% /
devfs 257 257 0 100% 892 0 100% /dev
/dev/disk3s6 1948404040 3145748 1101610152 1% 3 11016101520 0% /System/Volumes/VM
/dev/disk3s5 1948404040 816960428 1101610152 43% 7489758 11016101520 0% /System/Volumes/Data
/dev/disk7s1 1953309744 916650904 1036366016 47% 3325450 10363660160 0% /Volumes/BANK-1
/dev/disk3s3 1948404040 1940412 1101610152 1% 74 11016101520 0% /Volumes/Recovery
/dev/disk5s1 17809408 17286252 477900 98% 669862 4779000 12% /Library/Developer/CoreSimulator/Volumes/iOS_24A94401
"#;
        let state = parse_df_k_output(sample, "Macintosh HD");
        assert!(state.available);
        assert_eq!(state.volumes.len(), 2);
        assert_eq!(state.volumes[0].short_name, "MAC HD");
        assert_eq!(format_compact_bytes(state.volumes[0].used_bytes), "867G");
        assert_eq!(format_compact_bytes(state.volumes[0].avail_bytes), "1.1T");
        assert!((state.volumes[0].used_pct - 43.46).abs() < 0.5);

        assert_eq!(state.volumes[1].short_name, "BANK-1");
        assert_eq!(format_compact_bytes(state.volumes[1].used_bytes), "939G");
        assert_eq!(format_compact_bytes(state.volumes[1].avail_bytes), "1.1T");
        assert!((state.volumes[1].used_pct - 46.94).abs() < 0.5);
    }
}
