use serde::{Deserialize, Serialize};

/// Application settings loaded from settings.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Offset from machine origin in X direction (mm)
    pub offset_x: f64,
    /// Offset from machine origin in Y direction (mm)
    pub offset_y: f64,
    /// Clamp zone - space reserved for clamps (mm)
    pub clamp_zone: f64,
    /// Safe zone - additional safety margin (mm)
    pub safe_zone: f64,
    /// Tool diameter for calculating toolpath offset (mm)
    pub tool_diameter: f64,
    /// Total cut depth (mm)
    pub cut_depth: f64,
    /// Depth per pass (mm)
    pub step_depth: f64,
    /// Feed rate for cutting moves (mm/min)
    pub feed_rate: f64,
    /// Spindle speed (RPM)
    pub spindle_speed: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            offset_x: 10.0,
            offset_y: 10.0,
            clamp_zone: 5.0,
            safe_zone: 3.0,
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

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| SettingsError::SerializeError(e.to_string()))?;

        std::fs::write(&path, content)
            .map_err(|e| SettingsError::WriteError(e.to_string()))?;

        Ok(())
    }

}

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
