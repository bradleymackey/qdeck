use image::Rgb;
use serde::Deserialize;

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

#[derive(Debug, Clone, Copy, PartialEq)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex_color() {
        assert_eq!(parse_hex_color("#00ff41"), Some(Rgb([0, 255, 65])));
        assert_eq!(parse_hex_color("0xffb000"), Some(Rgb([255, 176, 0])));
        assert_eq!(parse_hex_color("  #FF2A2A  "), Some(Rgb([255, 42, 42])));
        assert_eq!(parse_hex_color("#123"), None);
        assert_eq!(parse_hex_color("not-a-color"), None);
    }

    #[test]
    fn test_theme_threshold_colors() {
        let theme = ThemeConfig::default().resolve();
        assert_eq!(theme.color_for_pct(25.0, None, None), theme.phosphor_green);
        assert_eq!(theme.color_for_pct(60.0, None, None), theme.amber);
        assert_eq!(theme.color_for_pct(85.0, None, None), theme.red);
        assert_eq!(
            theme.color_for_pct(75.0, Some(80.0), Some(95.0)),
            theme.phosphor_green
        );
    }
}
