pub mod defaults;
pub mod theme;
pub mod tile;

pub use defaults::DEFAULT_CONFIG_TOML;
pub use theme::{ResolvedTheme, ThemeConfig, parse_hex_color};
pub use tile::{ClickAction, TileConfig};

use anyhow::{Context, Result};
use serde::Deserialize;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

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

    /// Returns the ordered list of enabled modes. `modes` is authoritative: a mode left out of
    /// it is disabled, even if tiles still reference it. Falls back to `default_mode` (or
    /// `"dashboard"`) when `modes` is empty.
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
        out
    }

    /// Returns the ordered list of sub-panels for a given `mode`
    /// (e.g. `["fcu", "apu", "flt", "sys"]` for `xplane`).
    pub fn effective_panels(&self, active_mode: &str) -> Vec<String> {
        let active_norm = active_mode.trim().to_ascii_lowercase();
        let mut out: Vec<String> = Vec::new();
        for tile in &self.tiles {
            if tile.matches_mode(&active_norm, false)
                && let Some(p) = &tile.panel
            {
                let clean = p.trim().to_ascii_lowercase();
                if !clean.is_empty() && clean != "*" && clean != "all" && !out.contains(&clean) {
                    out.push(clean);
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

    /// Loads configuration from `~/.config/qdeck/config.toml` (or an explicit `--config` path).
    /// If `~/.config/qdeck/config.toml` does not exist yet, it is initialized once with
    /// `DEFAULT_CONFIG_TOML`.
    pub fn load(custom_path: Option<&Path>) -> Result<(Self, PathBuf)> {
        if let Some(path) = custom_path {
            let cfg = Self::load_from_file(path)?;
            return Ok((cfg, path.to_path_buf()));
        }

        let config_toml =
            user_config_path().context("Could not determine ~/.config/qdeck/config.toml path")?;
        if !config_toml.exists() {
            if let Some(parent) = config_toml.parent() {
                fs::create_dir_all(parent).with_context(|| {
                    format!("Failed to create config directory {}", parent.display())
                })?;
            }
            fs::write(&config_toml, DEFAULT_CONFIG_TOML).with_context(|| {
                format!(
                    "Failed to initialize default config at {}",
                    config_toml.display()
                )
            })?;
        }

        let cfg = Self::load_from_file(&config_toml)?;
        Ok((cfg, config_toml))
    }

    pub fn load_from_file(path: &Path) -> Result<Self> {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        let cfg: QDeckConfig = toml::from_str(&raw)
            .with_context(|| format!("Failed to parse QDeckConfig from {}", path.display()))?;
        Ok(cfg)
    }
}

pub fn user_config_dir() -> Option<PathBuf> {
    if let Ok(xdg) = env::var("XDG_CONFIG_HOME")
        && !xdg.trim().is_empty()
    {
        return Some(PathBuf::from(xdg).join("qdeck"));
    }
    env::var("HOME")
        .ok()
        .map(|home| PathBuf::from(home).join(".config").join("qdeck"))
}

pub fn user_config_path() -> Option<PathBuf> {
    user_config_dir().map(|dir| dir.join("config.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_default_config_parses_cleanly() {
        let cfg = QDeckConfig::builtin_default();
        assert_eq!(cfg.refresh_ms, 1000);
        assert_eq!(cfg.effective_modes(), vec!["dashboard", "xplane"]);
        assert_eq!(
            cfg.effective_panels("xplane"),
            vec!["fcu", "apu", "flt", "sys"]
        );
    }

    #[test]
    fn test_modes_list_disables_unlisted_modes() {
        let mut cfg = QDeckConfig::builtin_default();
        cfg.modes = vec!["dashboard".to_string()];
        assert_eq!(cfg.effective_modes(), vec!["dashboard"]);

        cfg.modes = Vec::new();
        cfg.default_mode = "xplane".to_string();
        assert_eq!(cfg.effective_modes(), vec!["xplane"]);
    }

    #[test]
    fn test_tile_resolution_across_modes_and_panels() {
        let cfg = QDeckConfig::builtin_default();
        // Key 0 (top-left) is always the global mode selector
        for mode in ["dashboard", "xplane"] {
            for panel in ["fcu", "apu", "flt", "sys"] {
                let t0 = cfg.tile_for_key(0, 5, 15, mode, panel).unwrap();
                assert!(t0.is_mode_selector());
            }
        }

        // In dashboard mode: Key 3 is GPU, Key 4 is CPU
        assert_eq!(
            cfg.tile_for_key(3, 5, 15, "dashboard", "fcu")
                .unwrap()
                .tile_type,
            "gpu"
        );
        assert_eq!(
            cfg.tile_for_key(4, 5, 15, "dashboard", "fcu")
                .unwrap()
                .tile_type,
            "cpu"
        );

        // In xplane mode: Key 1 (0,1) is always the PANEL selector across all 4 panels
        for panel in ["fcu", "apu", "flt", "sys"] {
            let t1 = cfg.tile_for_key(1, 5, 15, "xplane", panel).unwrap();
            assert!(t1.is_panel_selector());
        }

        // Key 2 (0,2) changes per panel:
        // fcu -> ap1, apu -> bat1, flt -> gear, sys -> strobe
        assert_eq!(
            cfg.tile_for_key(2, 5, 15, "xplane", "fcu")
                .unwrap()
                .control
                .as_deref(),
            Some("ap1")
        );
        assert_eq!(
            cfg.tile_for_key(2, 5, 15, "xplane", "apu")
                .unwrap()
                .control
                .as_deref(),
            Some("bat1")
        );
        assert_eq!(
            cfg.tile_for_key(2, 5, 15, "xplane", "flt")
                .unwrap()
                .control
                .as_deref(),
            Some("gear")
        );
        assert_eq!(
            cfg.tile_for_key(2, 5, 15, "xplane", "sys")
                .unwrap()
                .control
                .as_deref(),
            Some("strobe")
        );
    }
}
