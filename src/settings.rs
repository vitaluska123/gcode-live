use serde::{Deserialize, Serialize};

/// Application settings loaded from settings.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Offset from machine origin in X direction (mm)
    pub offset_x: f64,
    /// Offset from machine origin in Y direction (mm)
    pub offset_y: f64,
    /// Width of an uncut holding tab on the upper frame edge (mm).
    pub tab_width: f64,
    /// Minimum number of holding tabs.
    pub minimum_tabs: usize,
    /// Maximum allowed free distance between tabs and the frame ends (mm).
    pub maximum_tab_gap: f64,
    /// Tool diameter for calculating toolpath offset (mm)
    #[serde(skip_serializing, default = "default_tool_diameter")]
    pub tool_diameter: f64,
    /// Total cut depth (mm)
    #[serde(skip_serializing, default = "default_cut_depth")]
    pub cut_depth: f64,
    /// Depth per pass (mm)
    #[serde(skip_serializing, default = "default_step_depth")]
    pub step_depth: f64,
    /// Feed rate for cutting moves (mm/min)
    #[serde(skip_serializing, default = "default_feed_rate")]
    pub feed_rate: f64,
    /// Spindle speed (RPM)
    #[serde(skip_serializing, default = "default_spindle_speed")]
    pub spindle_speed: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            offset_x: 10.0,
            offset_y: 10.0,
            tab_width: 3.0,
            minimum_tabs: 3,
            maximum_tab_gap: 20.0,
            tool_diameter: 3.175,
            cut_depth: 1.6,
            step_depth: 0.4,
            feed_rate: 120.0,
            spindle_speed: 800.0,
        }
    }
}

impl Settings {
    /// Load settings from file, or create default if not exists
    pub fn load() -> Result<Self, SettingsError> {
        let path = std::path::PathBuf::from("settings.json");

        if !path.exists() {
            let default = Self::default();
            default.save()?;
            return Ok(default);
        }

        let content = std::fs::read_to_string(&path)
            .map_err(|e| SettingsError::ReadError(e.to_string()))?;

        let settings: Self = serde_json::from_str(&content)
            .map_err(|e| SettingsError::ParseError(e.to_string()))?;

        Ok(settings)
    }

    /// Save settings to file
    pub fn save(&self) -> Result<(), SettingsError> {
        let path = std::path::PathBuf::from("settings.json");

        let saved = Self {
            offset_x: round_two(self.offset_x),
            offset_y: round_two(self.offset_y),
            tab_width: round_two(self.tab_width),
            minimum_tabs: self.minimum_tabs,
            maximum_tab_gap: round_two(self.maximum_tab_gap),
            ..self.clone()
        };
        let content = serde_json::to_string_pretty(&saved)
            .map_err(|e| SettingsError::SerializeError(e.to_string()))?;

        std::fs::write(&path, content)
            .map_err(|e| SettingsError::WriteError(e.to_string()))?;

        Ok(())
    }

}

fn round_two(value: f64) -> f64 { (value * 100.0).round() / 100.0 }
fn default_tool_diameter() -> f64 { 3.175 }
fn default_cut_depth() -> f64 { 1.6 }
fn default_step_depth() -> f64 { 0.4 }
fn default_feed_rate() -> f64 { 120.0 }
fn default_spindle_speed() -> f64 { 800.0 }

#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("Failed to read settings file: {0}")]
    ReadError(String),
    #[error("Failed to parse settings file: {0}")]
    ParseError(String),
    #[error("Failed to serialize settings: {0}")]
    SerializeError(String),
    #[error("Failed to write settings file: {0}")]
    WriteError(String),
}
