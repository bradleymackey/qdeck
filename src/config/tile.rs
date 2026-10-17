use serde::Deserialize;

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
    /// Optional sub-panel within a mode (e.g. "fcu", "apu", "flt", "sys" in "xplane" mode)
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

#[derive(Debug, Clone, PartialEq, Eq)]
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
        if let Some(cmd) = &self.on_click
            && !cmd.trim().is_empty()
        {
            return ClickAction::ShellCommand(cmd.clone());
        }
        if let Some(app) = &self.app
            && !app.trim().is_empty()
        {
            return ClickAction::OpenApp(app.clone());
        }
        match self.tile_type.trim().to_ascii_lowercase().as_str() {
            "cpu" | "gpu" | "memory" | "ram" => {
                ClickAction::OpenApp("Activity Monitor".to_string())
            }
            "ops_status" | "status" => ClickAction::TriggerDemoSpike,
            _ => ClickAction::None,
        }
    }

    /// Resolves this tile's configuration to a 0-based key index given the deck layout (columns, total keys)
    pub fn resolve_key(&self, cols: u8, total_keys: u8) -> Option<u8> {
        let rows = total_keys.checked_div(cols).unwrap_or(3);
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
                let r_clean = r_str
                    .trim()
                    .trim_start_matches("row:")
                    .trim_start_matches('r');
                let c_clean = c_str
                    .trim()
                    .trim_start_matches("col:")
                    .trim_start_matches('c');
                if let (Ok(r), Ok(c)) = (r_clean.parse::<u8>(), c_clean.parse::<u8>())
                    && r < rows
                    && c < cols
                {
                    return Some(r * cols + c);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_tile(pos: Option<&str>, tile_type: &str) -> TileConfig {
        TileConfig {
            position: pos.map(str::to_string),
            key: None,
            row: None,
            col: None,
            tile_type: tile_type.to_string(),
            mode: None,
            panel: None,
            control: None,
            xplane_cmd: None,
            label: None,
            app: None,
            on_click: None,
            warn_threshold: None,
            crit_threshold: None,
        }
    }

    #[test]
    fn test_resolve_key_mk2_grid() {
        // Stream Deck MK.2 has 5 columns x 3 rows = 15 keys (indices 0..14)
        assert_eq!(
            dummy_tile(Some("top-left"), "mode").resolve_key(5, 15),
            Some(0)
        );
        assert_eq!(
            dummy_tile(Some("top-right"), "cpu").resolve_key(5, 15),
            Some(4)
        );
        assert_eq!(dummy_tile(Some("0,3"), "gpu").resolve_key(5, 15), Some(3));
        assert_eq!(
            dummy_tile(Some("1,2"), "a320_fcu").resolve_key(5, 15),
            Some(7)
        );
        assert_eq!(
            dummy_tile(Some("bottom-right"), "a320_fcu").resolve_key(5, 15),
            Some(14)
        );
        assert_eq!(dummy_tile(Some("5,5"), "a320_fcu").resolve_key(5, 15), None);
    }

    #[test]
    fn test_click_action_resolution() {
        let mode_tile = dummy_tile(Some("0,0"), "mode");
        assert_eq!(mode_tile.click_action(), ClickAction::CycleMode);

        let panel_tile = dummy_tile(Some("0,1"), "panel");
        assert_eq!(panel_tile.click_action(), ClickAction::CyclePanel);

        let cpu_tile = dummy_tile(Some("0,4"), "cpu");
        assert_eq!(
            cpu_tile.click_action(),
            ClickAction::OpenApp("Activity Monitor".to_string())
        );

        let mut fcu_tile = dummy_tile(Some("0,2"), "a320_fcu");
        fcu_tile.control = Some("ap1".to_string());
        assert_eq!(
            fcu_tile.click_action(),
            ClickAction::XPlaneControl {
                control: "ap1".to_string(),
                custom_cmd: None,
            }
        );
    }
}
