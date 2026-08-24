use serde::{Deserialize, Serialize};

/// Application settings loaded from settings.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Clearance around the source board in X direction (mm).
    pub offset_x: f64,
    /// Clearance around the source board in Y direction (mm).
    pub offset_y: f64,
    /// Place the local source coordinate system at this global position.
    #[serde(default)]
    pub local_offset_enabled: bool,
    #[serde(default)]
    pub local_offset_x: f64,
    #[serde(default)]
    pub local_offset_y: f64,
    /// Dimensions of the stock sheet in the global coordinate system.
    #[serde(default = "default_material_width")]
    pub material_width: f64,
    #[serde(default = "default_material_height")]
    pub material_height: f64,
    /// Position of the material's right edge relative to machine zero (mm).
    #[serde(default)]
    pub material_offset_x: f64,
    /// Position of the material's bottom edge relative to machine zero (mm).
    #[serde(default)]
    pub material_offset_y: f64,
    /// Minimum clearance between the frame and material edges along X (mm).
    #[serde(default)]
    pub material_edge_margin_x: f64,
    /// Minimum clearance between the frame and material edges along Y (mm).
    #[serde(default)]
    pub material_edge_margin_y: f64,
    #[serde(default = "default_true")]
    pub show_grid: bool,
    #[serde(default = "default_true")]
    pub show_axes: bool,
    #[serde(default = "default_true")]
    pub show_material: bool,
    #[serde(default = "default_true")]
    pub show_safe_area: bool,
    #[serde(default = "default_true")]
    pub show_margin_hatch: bool,
    #[serde(default = "default_material_color")]
    pub material_color: String,
    #[serde(default = "default_safe_area_color")]
    pub safe_area_color: String,
    #[serde(default = "default_frame_color")]
    pub frame_color: String,
    #[serde(default = "default_background_color")]
    pub background_color: String,
    #[serde(default = "default_grid_color")]
    pub grid_color: String,
    #[serde(default = "default_grid_label_color")]
    pub grid_label_color: String,
    #[serde(default = "default_axis_x_color")]
    pub axis_x_color: String,
    #[serde(default = "default_axis_y_color")]
    pub axis_y_color: String,
    #[serde(default = "default_toolpath_color")]
    pub toolpath_color: String,
    #[serde(default = "default_rapid_color")]
    pub rapid_color: String,
    #[serde(default = "default_expanded_frame_color")]
    pub expanded_frame_color: String,
    #[serde(default = "default_tab_color")]
    pub tab_color: String,
    #[serde(default = "default_margin_hatch_color")]
    pub margin_hatch_color: String,
    #[serde(default = "default_true")]
    pub show_toolpath: bool,
    #[serde(default = "default_true")]
    pub show_rapid: bool,
    #[serde(default = "default_true")]
    pub show_expanded_frame: bool,
    #[serde(default = "default_true")]
    pub show_frame: bool,
    #[serde(default = "default_true")]
    pub show_tabs: bool,
    /// Width of an uncut holding tab on the upper frame edge (mm).
    pub tab_width: f64,
    /// Minimum number of holding tabs.
    pub minimum_tabs: usize,
    /// Maximum allowed free distance between tabs and the frame ends (mm).
    pub maximum_tab_gap: f64,
    /// Cut tab locations to half depth before leaving them as holding tabs.
    #[serde(default)]
    pub score_tabs: bool,
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
            local_offset_enabled: false,
            local_offset_x: 0.0,
            local_offset_y: 0.0,
            material_width: 150.0,
            material_height: 150.0,
            material_offset_x: 0.0,
            material_offset_y: 0.0,
            material_edge_margin_x: 0.0,
            material_edge_margin_y: 0.0,
            show_grid: true,
            show_axes: true,
            show_material: true,
            show_safe_area: true,
            show_margin_hatch: true,
            material_color: default_material_color(),
            safe_area_color: default_safe_area_color(),
            frame_color: default_frame_color(),
            background_color: default_background_color(),
            grid_color: default_grid_color(),
            grid_label_color: default_grid_label_color(),
            axis_x_color: default_axis_x_color(),
            axis_y_color: default_axis_y_color(),
            toolpath_color: default_toolpath_color(),
            rapid_color: default_rapid_color(),
            expanded_frame_color: default_expanded_frame_color(),
            tab_color: default_tab_color(),
            margin_hatch_color: default_margin_hatch_color(),
            show_toolpath: true,
            show_rapid: true,
            show_expanded_frame: true,
            show_frame: true,
            show_tabs: true,
            tab_width: 3.0,
            minimum_tabs: 3,
            maximum_tab_gap: 20.0,
            score_tabs: false,
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

        let content =
            std::fs::read_to_string(&path).map_err(|e| SettingsError::Read(e.to_string()))?;

        let settings: Self =
            serde_json::from_str(&content).map_err(|e| SettingsError::Parse(e.to_string()))?;

        Ok(settings)
    }

    /// Save settings to file
    pub fn save(&self) -> Result<(), SettingsError> {
        let path = std::path::PathBuf::from("settings.json");

        let saved = Self {
            offset_x: round_two(self.offset_x),
            offset_y: round_two(self.offset_y),
            local_offset_x: round_two(self.local_offset_x),
            local_offset_y: round_two(self.local_offset_y),
            material_width: round_two(self.material_width),
            material_height: round_two(self.material_height),
            material_offset_x: round_two(self.material_offset_x),
            material_offset_y: round_two(self.material_offset_y),
            material_edge_margin_x: round_two(self.material_edge_margin_x),
            material_edge_margin_y: round_two(self.material_edge_margin_y),
            tab_width: round_two(self.tab_width),
            minimum_tabs: self.minimum_tabs,
            maximum_tab_gap: round_two(self.maximum_tab_gap),
            ..self.clone()
        };
        let content = serde_json::to_string_pretty(&saved)
            .map_err(|e| SettingsError::Serialize(e.to_string()))?;

        std::fs::write(&path, content).map_err(|e| SettingsError::Write(e.to_string()))?;

        Ok(())
    }
}

fn round_two(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}
fn default_tool_diameter() -> f64 {
    3.175
}
fn default_cut_depth() -> f64 {
    1.6
}
fn default_step_depth() -> f64 {
    0.4
}
fn default_feed_rate() -> f64 {
    120.0
}
fn default_spindle_speed() -> f64 {
    800.0
}
fn default_material_width() -> f64 {
    150.0
}
fn default_material_height() -> f64 {
    150.0
}
fn default_true() -> bool {
    true
}
fn default_material_color() -> String {
    "#be64ff".into()
}
fn default_safe_area_color() -> String {
    "#ff4646".into()
}
fn default_frame_color() -> String {
    "#ff4664".into()
}
fn default_background_color() -> String {
    "#0e1116".into()
}
fn default_grid_color() -> String {
    "#2a303a".into()
}
fn default_grid_label_color() -> String {
    "#96a0af".into()
}
fn default_axis_x_color() -> String {
    "#dc4646".into()
}
fn default_axis_y_color() -> String {
    "#46d278".into()
}
fn default_toolpath_color() -> String {
    "#00d2ff".into()
}
fn default_rapid_color() -> String {
    "#ffbe00".into()
}
fn default_expanded_frame_color() -> String {
    "#ffd200".into()
}
fn default_tab_color() -> String {
    "#ffdc46".into()
}
fn default_margin_hatch_color() -> String {
    "#ff4646".into()
}

impl Settings {
    pub fn local_offset(&self) -> (f64, f64) {
        if self.local_offset_enabled {
            (self.local_offset_x, self.local_offset_y)
        } else {
            (0.0, 0.0)
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("Failed to read settings file: {0}")]
    Read(String),
    #[error("Failed to parse settings file: {0}")]
    Parse(String),
    #[error("Failed to serialize settings: {0}")]
    Serialize(String),
    #[error("Failed to write settings file: {0}")]
    Write(String),
}
