use anyhow::{Context, Result};
use image::Rgb;
use serde::Deserialize;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub const DEFAULT_CONFIG_TOML: &str = r##"# =============================================================================
# QDeck // Tactical Ops-Center Configuration (~/.config/qdeck/config.toml)
# Changes to this file are hot-reloaded automatically while qdeck is running.
# =============================================================================

brightness = 85
refresh_ms = 1000
vid = 0x0fd9
# pid = 0x0080          # Stream Deck MK.2 (auto-detected if omitted)
background_mode = "grid" # "grid" (tactical standby reticle) or "blank" (pure black)
default_mode = "dashboard"
modes = ["dashboard", "xplane"]
xplane_addr = "127.0.0.1:49000"

[theme]
background = "#000000"
grid_dim = "#09260e"
bar_track = "#061809"
phosphor_green = "#00ff41"
phosphor_dim = "#008f24"
amber = "#ffb000"
red = "#ff2a2a"
warn_threshold = 60.0
crit_threshold = 85.0

# -----------------------------------------------------------------------------
# Global Mode Selector (Top-Left across all modes)
# -----------------------------------------------------------------------------
[[tiles]]
type = "mode"
position = "top-left"
mode = "*"

# -----------------------------------------------------------------------------
# Mode: "dashboard" (System Telemetry)
# -----------------------------------------------------------------------------
[[tiles]]
type = "gpu"
position = "0,3"
mode = "dashboard"
label = "GPU"
app = "Activity Monitor"
warn_threshold = 60.0
crit_threshold = 85.0

[[tiles]]
type = "cpu"
position = "top-right"
mode = "dashboard"
label = "CPU"
app = "Activity Monitor"
warn_threshold = 60.0
crit_threshold = 85.0

# -----------------------------------------------------------------------------
# Mode: "xplane" — Secondary Panel Selector (Key 0,1)
# Toggles between Panel 1 ("fcu" Autopilot) and Panel 2 ("apu" APU / Eng Start)
# -----------------------------------------------------------------------------
[[tiles]]
type = "panel"
position = "0,1"
mode = "xplane"
label = "PANEL"

# =============================================================================
# Mode: "xplane" // PANEL 1: "fcu" (Airbus A320/A330 FCU & Glare-Shield)
# =============================================================================
# Row 0: MODE (0,0) | PANEL (0,1) | AP 1 (0,2) | AP 2 (0,3) | A/THR (0,4)
[[tiles]]
type = "a320_fcu"
control = "ap1"
position = "0,2"
mode = "xplane"
panel = "fcu"
label = "AP 1"

[[tiles]]
type = "a320_fcu"
control = "ap2"
position = "0,3"
mode = "xplane"
panel = "fcu"
label = "AP 2"

[[tiles]]
type = "a320_fcu"
control = "athr"
position = "0,4"
mode = "xplane"
panel = "fcu"
label = "A/THR"

# Row 1: FD 1 (1,0) | SPD (1,1) | HDG (1,2) | ALT (1,3) | APPR (1,4)
[[tiles]]
type = "a320_fcu"
control = "fd"
position = "1,0"
mode = "xplane"
panel = "fcu"
label = "FD 1"

[[tiles]]
type = "a320_fcu"
control = "spd"
position = "1,1"
mode = "xplane"
panel = "fcu"
label = "SPD"

[[tiles]]
type = "a320_fcu"
control = "hdg"
position = "1,2"
mode = "xplane"
panel = "fcu"
label = "HDG"

[[tiles]]
type = "a320_fcu"
control = "alt"
position = "1,3"
mode = "xplane"
panel = "fcu"
label = "ALT"

[[tiles]]
type = "a320_fcu"
control = "appr"
position = "1,4"
mode = "xplane"
panel = "fcu"
label = "APPR"

# Row 2: M WARN (2,0) | M CAUT (2,1) | LOC (2,2) | FPA (2,3) | V/S (2,4)
[[tiles]]
type = "a320_fcu"
control = "master_warn"
position = "2,0"
mode = "xplane"
panel = "fcu"

[[tiles]]
type = "a320_fcu"
control = "master_caut"
position = "2,1"
mode = "xplane"
panel = "fcu"

[[tiles]]
type = "a320_fcu"
control = "loc"
position = "2,2"
mode = "xplane"
panel = "fcu"
label = "LOC"

[[tiles]]
type = "a320_fcu"
control = "trk"
position = "2,3"
mode = "xplane"
panel = "fcu"
label = "FPA"

[[tiles]]
type = "a320_fcu"
control = "vs"
position = "2,4"
mode = "xplane"
panel = "fcu"
label = "V/S"

# =============================================================================
# Mode: "xplane" // PANEL 2: "apu" (Airbus Overhead Electrical, APU & Eng Prep)
# =============================================================================
# Row 0: MODE (0,0) | PANEL (0,1) | BAT 1 (0,2) | BAT 2 (0,3) | EXT PWR (0,4)
[[tiles]]
type = "a320_fcu"
control = "bat1"
position = "0,2"
mode = "xplane"
panel = "apu"
label = "BAT 1"

[[tiles]]
type = "a320_fcu"
control = "bat2"
position = "0,3"
mode = "xplane"
panel = "apu"
label = "BAT 2"

[[tiles]]
type = "a320_fcu"
control = "ext_pwr"
position = "0,4"
mode = "xplane"
panel = "apu"
label = "EXT PWR"

# Row 1: APU MST (1,0) | APU STR (1,1) | APU BLD (1,2) | APU GEN (1,3) | FUEL PMP (1,4)
[[tiles]]
type = "a320_fcu"
control = "apu_master"
position = "1,0"
mode = "xplane"
panel = "apu"
label = "APU MST"

[[tiles]]
type = "a320_fcu"
control = "apu_start"
position = "1,1"
mode = "xplane"
panel = "apu"
label = "APU STR"

[[tiles]]
type = "a320_fcu"
control = "apu_bleed"
position = "1,2"
mode = "xplane"
panel = "apu"
label = "APU BLD"

[[tiles]]
type = "a320_fcu"
control = "apu_gen"
position = "1,3"
mode = "xplane"
panel = "apu"
label = "APU GEN"

[[tiles]]
type = "a320_fcu"
control = "fuel_pumps"
position = "1,4"
mode = "xplane"
panel = "apu"
label = "FUEL PMP"

# Row 2: ENG MOD (2,0) | PACK 1/2 (2,1) | BEACON (2,2) | ECAM APU (2,3) | ECAM ENG (2,4)
[[tiles]]
type = "a320_fcu"
control = "eng_mode"
position = "2,0"
mode = "xplane"
panel = "apu"
label = "ENG MOD"

[[tiles]]
type = "a320_fcu"
control = "packs"
position = "2,1"
mode = "xplane"
panel = "apu"
label = "PACK 1/2"

[[tiles]]
type = "a320_fcu"
control = "beacon"
position = "2,2"
mode = "xplane"
panel = "apu"
label = "BEACON"

[[tiles]]
type = "a320_fcu"
control = "ecam_apu"
position = "2,3"
mode = "xplane"
panel = "apu"
label = "ECAM APU"

[[tiles]]
type = "a320_fcu"
control = "ecam_eng"
position = "2,4"
mode = "xplane"
panel = "apu"
label = "ECAM ENG"
"##;

#[derive(Debug, Clone, Deserialize)]
pub struct CargoManifest {
    pub package: Option<CargoPackage>,
    pub qdeck: Option<QDeckConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CargoPackage {
    pub metadata: Option<CargoMetadata>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CargoMetadata {
    pub qdeck: Option<QDeckConfig>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct QDeckConfig {
    /// Display brightness (0-100%)
    pub brightness: u8,
    /// Telemetry refresh rate in milliseconds
    pub refresh_ms: u64,
    /// USB Vendor ID (Elgato = 0x0fd9)
    pub vid: u16,
    /// Optional USB Product ID (auto-detected if None)
    pub pid: Option<u16>,
    /// Optional USB Serial number
    pub serial: Option<String>,
    /// How unconfigured keys should be rendered ("grid" or "blank")
    pub background_mode: String,
    /// Initial mode when qdeck starts
    pub default_mode: String,
    /// List of available modes to toggle through when clicking the mode tile
    pub modes: Vec<String>,
    /// X-Plane UDP address (default "127.0.0.1:49000")
    pub xplane_addr: String,
    /// Color and threshold theme
    pub theme: ThemeConfig,
    /// Configured tiles on the deck
    pub tiles: Vec<TileConfig>,
}

impl Default for QDeckConfig {
    fn default() -> Self {
        Self {
            brightness: 90,
            refresh_ms: 1000,
            vid: 0x0fd9,
            pid: None,
            serial: None,
            background_mode: "grid".to_string(),
            default_mode: "dashboard".to_string(),
            modes: vec!["dashboard".to_string(), "xplane".to_string()],
            xplane_addr: "127.0.0.1:49000".to_string(),
            theme: ThemeConfig::default(),
            tiles: Vec::new(),
        }
    }
}

impl QDeckConfig {
    pub fn builtin_default() -> Self {
        toml::from_str(DEFAULT_CONFIG_TOML).unwrap_or_default()
    }

    /// Returns the ordered list of modes (always starting with `default_mode` or `"dashboard"`),
    /// including any additional modes referenced by tiles.
    pub fn effective_modes(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for m in &self.modes {
            let clean = m.trim().to_ascii_lowercase();
            if !clean.is_empty() && !out.contains(&clean) {
                out.push(clean);
            }
        }
        if out.is_empty() {
            let def = self.default_mode.trim().to_ascii_lowercase();
            out.push(if def.is_empty() {
                "dashboard".to_string()
            } else {
                def
            });
        }
        for tile in &self.tiles {
            if let Some(m) = &tile.mode {
                let clean = m.trim().to_ascii_lowercase();
                if !clean.is_empty() && clean != "*" && clean != "all" && !out.contains(&clean) {
                    out.push(clean);
                }
            }
        }
        out
    }

    /// Returns the ordered list of sub-panels for a given `mode` (e.g. `["fcu", "apu"]` for `xplane`).
    pub fn effective_panels(&self, active_mode: &str) -> Vec<String> {
        let active_norm = active_mode.trim().to_ascii_lowercase();
        let mut out: Vec<String> = Vec::new();
        for tile in &self.tiles {
            if tile.matches_mode(&active_norm, false) {
                if let Some(p) = &tile.panel {
                    let clean = p.trim().to_ascii_lowercase();
                    if !clean.is_empty() && clean != "*" && clean != "all" && !out.contains(&clean)
                    {
                        out.push(clean);
                    }
                }
            }
        }
        if out.is_empty() {
            out.push("fcu".to_string());
        }
        out
    }

    /// Returns the active tile for `key_idx` in `(active_mode, active_panel)`.
    pub fn tile_for_key(
        &self,
        key_idx: u8,
        cols: u8,
        total_keys: u8,
        active_mode: &str,
        active_panel: &str,
    ) -> Option<&TileConfig> {
        let active_norm = active_mode.trim().to_ascii_lowercase();
        let panel_norm = active_panel.trim().to_ascii_lowercase();

        // 1. Exact mode match + exact panel match
        if let Some(t) = self.tiles.iter().find(|t| {
            t.resolve_key(cols, total_keys) == Some(key_idx)
                && t.matches_mode(&active_norm, false)
                && t.matches_panel(&panel_norm, false)
        }) {
            return Some(t);
        }

        // 2. Exact mode match + wildcard/shared panel (e.g. the panel toggle tile at 0,1)
        if let Some(t) = self.tiles.iter().find(|t| {
            t.resolve_key(cols, total_keys) == Some(key_idx)
                && t.matches_mode(&active_norm, false)
                && t.matches_panel(&panel_norm, true)
        }) {
            return Some(t);
        }

        // 3. Global wildcard mode (e.g. the top-left mode selector at 0,0)
        self.tiles.iter().find(|t| {
            t.resolve_key(cols, total_keys) == Some(key_idx) && t.matches_mode(&active_norm, true)
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ThemeConfig {
    /// Background color (hex)
    pub background: String,
    /// Dim tactical grid / frame color
    pub grid_dim: String,
    /// Unlit bar segment color
    pub bar_track: String,
    /// Primary phosphor-green color
    pub phosphor_green: String,
    /// Dim phosphor-green for secondary labels
    pub phosphor_dim: String,
    /// Amber color for elevated / warning states needing attention
    pub amber: String,
    /// Red color for critical states needing immediate attention
    pub red: String,
    /// Default warning threshold percentage (0.0 - 100.0)
    pub warn_threshold: f32,
    /// Default critical threshold percentage (0.0 - 100.0)
    pub crit_threshold: f32,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            background: "#000000".to_string(),
            grid_dim: "#09260e".to_string(),
            bar_track: "#061809".to_string(),
            phosphor_green: "#00ff41".to_string(),
            phosphor_dim: "#008f24".to_string(),
            amber: "#ffb000".to_string(),
            red: "#ff2a2a".to_string(),
            warn_threshold: 60.0,
            crit_threshold: 85.0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ResolvedTheme {
    pub background: Rgb<u8>,
    pub grid_dim: Rgb<u8>,
    pub bar_track: Rgb<u8>,
    pub phosphor_green: Rgb<u8>,
    pub phosphor_dim: Rgb<u8>,
    pub amber: Rgb<u8>,
    pub red: Rgb<u8>,
    pub warn_threshold: f32,
    pub crit_threshold: f32,
}

impl ThemeConfig {
    pub fn resolve(&self) -> ResolvedTheme {
        ResolvedTheme {
            background: parse_hex_color(&self.background).unwrap_or(Rgb([0, 0, 0])),
            grid_dim: parse_hex_color(&self.grid_dim).unwrap_or(Rgb([9, 38, 14])),
            bar_track: parse_hex_color(&self.bar_track).unwrap_or(Rgb([6, 24, 9])),
            phosphor_green: parse_hex_color(&self.phosphor_green).unwrap_or(Rgb([0, 255, 65])),
            phosphor_dim: parse_hex_color(&self.phosphor_dim).unwrap_or(Rgb([0, 143, 36])),
            amber: parse_hex_color(&self.amber).unwrap_or(Rgb([255, 176, 0])),
            red: parse_hex_color(&self.red).unwrap_or(Rgb([255, 42, 42])),
            warn_threshold: self.warn_threshold,
            crit_threshold: self.crit_threshold,
        }
    }
}

impl ResolvedTheme {
    pub fn color_for_pct(
        &self,
        pct: f32,
        warn_override: Option<f32>,
        crit_override: Option<f32>,
    ) -> Rgb<u8> {
        let warn = warn_override.unwrap_or(self.warn_threshold);
        let crit = crit_override.unwrap_or(self.crit_threshold);
        if pct >= crit {
            self.red
        } else if pct >= warn {
            self.amber
        } else {
            self.phosphor_green
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct TileConfig {
    /// Named position: "top-right", "top-left", "bottom-left", "bottom-right", "center", or "row,col"
    pub position: Option<String>,
    /// Explicit 0-based key index
    pub key: Option<u8>,
    /// Explicit 0-based row index
    pub row: Option<u8>,
    /// Explicit 0-based column index
    pub col: Option<u8>,
    /// Tile type: "mode", "panel", "cpu", "gpu", "memory", "a320_fcu", "ops_status", "launcher"
    #[serde(rename = "type", alias = "widget", alias = "kind")]
    pub tile_type: String,
    /// Optional mode this tile belongs to (e.g. "dashboard", "xplane", or "*" for all modes)
    pub mode: Option<String>,
    /// Optional sub-panel within a mode (e.g. "fcu" vs "apu" in "xplane" mode)
    pub panel: Option<String>,
    /// Optional A320 / X-Plane control identifier
    pub control: Option<String>,
    /// Optional explicit X-Plane UDP command string (e.g. "sim/autopilot/servos_toggle")
    pub xplane_cmd: Option<String>,
    /// Optional custom label on the tile
    pub label: Option<String>,
    /// Optional macOS application name to open when clicked (e.g. "Activity Monitor")
    pub app: Option<String>,
    /// Optional shell command to execute when clicked
    #[serde(alias = "command", alias = "action")]
    pub on_click: Option<String>,
    /// Optional per-tile warning threshold override
    pub warn_threshold: Option<f32>,
    /// Optional per-tile critical threshold override
    pub crit_threshold: Option<f32>,
}

#[derive(Debug, Clone)]
pub enum ClickAction {
    CycleMode,
    CyclePanel,
    XPlaneControl {
        control: String,
        custom_cmd: Option<String>,
    },
    OpenApp(String),
    ShellCommand(String),
    TriggerDemoSpike,
    None,
}

impl TileConfig {
    pub fn is_mode_selector(&self) -> bool {
        matches!(
            self.tile_type.trim().to_ascii_lowercase().as_str(),
            "mode" | "mode_selector"
        )
    }

    pub fn is_panel_selector(&self) -> bool {
        matches!(
            self.tile_type.trim().to_ascii_lowercase().as_str(),
            "panel" | "panel_selector" | "submode"
        )
    }

    pub fn is_a320_fcu(&self) -> bool {
        matches!(
            self.tile_type.trim().to_ascii_lowercase().as_str(),
            "a320_fcu" | "xplane" | "fcu" | "a320"
        )
    }

    pub fn matches_mode(&self, active_mode: &str, wildcard_pass: bool) -> bool {
        if self.is_mode_selector() {
            return wildcard_pass;
        }
        match &self.mode {
            Some(m) => {
                let norm = m.trim().to_ascii_lowercase();
                if norm == "*" || norm == "all" {
                    wildcard_pass
                } else {
                    !wildcard_pass && norm == active_mode
                }
            }
            None => !wildcard_pass && active_mode == "dashboard",
        }
    }

    pub fn matches_panel(&self, active_panel: &str, wildcard_pass: bool) -> bool {
        if self.is_panel_selector() {
            return wildcard_pass;
        }
        match &self.panel {
            Some(p) => {
                let norm = p.trim().to_ascii_lowercase();
                if norm == "*" || norm == "all" {
                    wildcard_pass
                } else {
                    !wildcard_pass && norm == active_panel
                }
            }
            None => wildcard_pass,
        }
    }

    /// Determines what action should happen when this tile is clicked on the Stream Deck.
    pub fn click_action(&self) -> ClickAction {
        if self.is_mode_selector() {
            return ClickAction::CycleMode;
        }
        if self.is_panel_selector() {
            return ClickAction::CyclePanel;
        }
        if self.is_a320_fcu() || self.control.is_some() || self.xplane_cmd.is_some() {
            let ctrl = self
                .control
                .clone()
                .unwrap_or_else(|| self.tile_type.clone());
            return ClickAction::XPlaneControl {
                control: ctrl,
                custom_cmd: self.xplane_cmd.clone(),
            };
        }
        if let Some(cmd) = &self.on_click {
            if !cmd.trim().is_empty() {
                return ClickAction::ShellCommand(cmd.clone());
            }
        }
        if let Some(app) = &self.app {
            if !app.trim().is_empty() {
                return ClickAction::OpenApp(app.clone());
            }
        }
        match self.tile_type.trim().to_ascii_lowercase().as_str() {
            "cpu" | "gpu" | "memory" | "ram" => ClickAction::OpenApp("Activity Monitor".to_string()),
            "ops_status" | "status" => ClickAction::TriggerDemoSpike,
            _ => ClickAction::None,
        }
    }

    /// Resolves this tile's configuration to a 0-based key index given the deck layout (columns, total keys)
    pub fn resolve_key(&self, cols: u8, total_keys: u8) -> Option<u8> {
        let rows = if cols > 0 { total_keys / cols } else { 3 };
        if let Some(k) = self.key {
            return (k < total_keys).then_some(k);
        }
        if let (Some(r), Some(c)) = (self.row, self.col) {
            if r < rows && c < cols {
                let k = r.saturating_mul(cols).saturating_add(c);
                return (k < total_keys).then_some(k);
            }
            return None;
        }
        if let Some(pos) = &self.position {
            let normalized = pos.trim().to_ascii_lowercase().replace('_', "-");
            if let Some((r_str, c_str)) = normalized.split_once(',') {
                let r_clean = r_str.trim().trim_start_matches("row:").trim_start_matches('r');
                let c_clean = c_str.trim().trim_start_matches("col:").trim_start_matches('c');
                if let (Ok(r), Ok(c)) = (r_clean.parse::<u8>(), c_clean.parse::<u8>()) {
                    if r < rows && c < cols {
                        return Some(r * cols + c);
                    }
                }
            }
            let k = match normalized.as_str() {
                "top-left" => 0,
                "top-center" | "top-middle" => cols / 2,
                "top-right" => cols.saturating_sub(1),
                "middle-left" | "center-left" => (rows / 2).saturating_mul(cols),
                "center" | "middle" => (rows / 2).saturating_mul(cols).saturating_add(cols / 2),
                "middle-right" | "center-right" => (rows / 2)
                    .saturating_mul(cols)
                    .saturating_add(cols.saturating_sub(1)),
                "bottom-left" => rows.saturating_sub(1).saturating_mul(cols),
                "bottom-center" | "bottom-middle" => rows
                    .saturating_sub(1)
                    .saturating_mul(cols)
                    .saturating_add(cols / 2),
                "bottom-right" => total_keys.saturating_sub(1),
                other => {
                    if let Ok(idx) = other.parse::<u8>() {
                        idx
                    } else {
                        return None;
                    }
                }
            };
            return (k < total_keys).then_some(k);
        }
        None
    }
}

pub fn parse_hex_color(s: &str) -> Option<Rgb<u8>> {
    let hex = s.trim().trim_start_matches('#').trim_start_matches("0x");
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Rgb([r, g, b]))
}

pub fn user_config_dir() -> Option<PathBuf> {
    if let Ok(xdg) = env::var("XDG_CONFIG_HOME") {
        if !xdg.trim().is_empty() {
            return Some(PathBuf::from(xdg).join("qdeck"));
        }
    }
    env::var("HOME")
        .ok()
        .map(|home| PathBuf::from(home).join(".config").join("qdeck"))
}

impl QDeckConfig {
    pub fn load(custom_path: Option<&Path>) -> Result<(Self, PathBuf)> {
        if let Some(path) = custom_path {
            let cfg = Self::load_from_file(path)?;
            return Ok((cfg, path.to_path_buf()));
        }

        if let Some(dir) = user_config_dir() {
            let config_toml = dir.join("config.toml");
            let qdeck_toml = dir.join("qdeck.toml");

            if qdeck_toml.exists() && !config_toml.exists() {
                let cfg = Self::load_from_file(&qdeck_toml)?;
                return Ok((cfg, qdeck_toml));
            }

            if !config_toml.exists() {
                if let Err(e) = fs::create_dir_all(&dir)
                    .and_then(|_| fs::write(&config_toml, DEFAULT_CONFIG_TOML))
                {
                    eprintln!(
                        "[QDECK] Note: Could not write default config to {}: {}",
                        config_toml.display(),
                        e
                    );
                }
            }

            if config_toml.exists() {
                let cfg = Self::load_from_file(&config_toml)?;
                return Ok((cfg, config_toml));
            }
        }

        let local_qdeck = PathBuf::from("qdeck.toml");
        if local_qdeck.exists() {
            let cfg = Self::load_from_file(&local_qdeck)?;
            return Ok((cfg, local_qdeck));
        }

        let cargo_toml = PathBuf::from("Cargo.toml");
        if cargo_toml.exists() {
            if let Ok(Some(cfg)) = Self::try_load_manifest(&cargo_toml) {
                return Ok((cfg, cargo_toml));
            }
        }

        Ok((Self::builtin_default(), PathBuf::from("built-in defaults")))
    }

    fn try_load_manifest(path: &Path) -> Result<Option<Self>> {
        let raw =
            fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;
        let manifest: CargoManifest = toml::from_str(&raw)
            .with_context(|| format!("Failed to parse TOML in {}", path.display()))?;

        if let Some(pkg) = manifest.package {
            if let Some(meta) = pkg.metadata {
                if let Some(qdeck) = meta.qdeck {
                    return Ok(Some(qdeck));
                }
            }
        }
        if let Some(qdeck) = manifest.qdeck {
            return Ok(Some(qdeck));
        }
        Ok(None)
    }

    pub fn load_from_file(path: &Path) -> Result<Self> {
        let raw =
            fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;
        if let Ok(cfg) = toml::from_str::<QDeckConfig>(&raw) {
            if !cfg.tiles.is_empty() || raw.contains("tiles") || raw.contains("[theme]") {
                return Ok(cfg);
            }
        }
        if let Ok(Some(cfg)) = Self::try_load_manifest(path) {
            return Ok(cfg);
        }
        let cfg: QDeckConfig = toml::from_str(&raw)
            .with_context(|| format!("Failed to parse QDeckConfig from {}", path.display()))?;
        Ok(cfg)
    }
}
