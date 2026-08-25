use serde::{Deserialize, Serialize};

const CURRENT_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub schema_version: u32,
    #[serde(default)]
    pub calculation: CalculationSettings,
    #[serde(default)]
    pub coordinates: CoordinateSettings,
    #[serde(default)]
    pub material: MaterialSettings,
    #[serde(default)]
    pub display: DisplaySettings,
    #[serde(default = "default_use_opengl_renderer")]
    pub use_opengl_renderer: bool,
    #[serde(skip_serializing, default)]
    pub machining: MachiningSettings,
    #[serde(skip)]
    pub offset_x: f64,
    #[serde(skip)]
    pub offset_y: f64,
    #[serde(skip)]
    pub local_offset_enabled: bool,
    #[serde(skip)]
    pub local_offset_x: f64,
    #[serde(skip)]
    pub local_offset_y: f64,
    #[serde(skip)]
    pub snap_to_geometry: bool,
    #[serde(skip)]
    pub material_width: f64,
    #[serde(skip)]
    pub material_height: f64,
    #[serde(skip)]
    pub material_offset_x: f64,
    #[serde(skip)]
    pub material_offset_y: f64,
    #[serde(skip)]
    pub material_edge_margin_x: f64,
    #[serde(skip)]
    pub material_edge_margin_y: f64,
    #[serde(skip)]
    pub show_grid: bool,
    #[serde(skip)]
    pub show_axes: bool,
    #[serde(skip)]
    pub show_local_axes: bool,
    #[serde(skip)]
    pub show_material: bool,
    #[serde(skip)]
    pub show_safe_area: bool,
    #[serde(skip)]
    pub show_margin_hatch: bool,
    #[serde(skip)]
    pub material_color: String,
    #[serde(skip)]
    pub safe_area_color: String,
    #[serde(skip)]
    pub frame_color: String,
    #[serde(skip)]
    pub background_color: String,
    #[serde(skip)]
    pub grid_color: String,
    #[serde(skip)]
    pub grid_label_color: String,
    #[serde(skip)]
    pub axis_x_color: String,
    #[serde(skip)]
    pub axis_y_color: String,
    #[serde(skip)]
    pub local_axis_x_color: String,
    #[serde(skip)]
    pub local_axis_y_color: String,
    #[serde(skip)]
    pub toolpath_color: String,
    #[serde(skip)]
    pub rapid_color: String,
    #[serde(skip)]
    pub expanded_frame_color: String,
    #[serde(skip)]
    pub tab_color: String,
    #[serde(skip)]
    pub margin_hatch_color: String,
    #[serde(skip)]
    pub show_toolpath: bool,
    #[serde(skip)]
    pub show_rapid: bool,
    #[serde(skip)]
    pub show_expanded_frame: bool,
    #[serde(skip)]
    pub show_frame: bool,
    #[serde(skip)]
    pub show_tabs: bool,
    #[serde(skip)]
    pub tab_width: f64,
    #[serde(skip)]
    pub minimum_tabs: usize,
    #[serde(skip)]
    pub maximum_tab_gap: f64,
    #[serde(skip)]
    pub score_tabs: bool,
    #[serde(skip)]
    pub tool_diameter: f64,
    #[serde(skip)]
    pub cut_depth: f64,
    #[serde(skip)]
    pub step_depth: f64,
    /// Distinct progressive cutting depths read from the current source file.
    /// This is runtime state: it must not be saved as an application preference.
    #[serde(skip)]
    pub source_cut_depths: Vec<f64>,
    #[serde(skip)]
    pub feed_rate: f64,
    #[serde(skip)]
    pub spindle_speed: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalculationSettings {
    pub frame_offset_x: f64,
    pub frame_offset_y: f64,
    pub tabs: TabSettings,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabSettings {
    pub width: f64,
    pub minimum_count: usize,
    pub maximum_gap: f64,
    #[serde(default)]
    pub score_to_half_depth: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinateSettings {
    #[serde(default)]
    pub local_origin_enabled: bool,
    #[serde(default)]
    pub local_origin_x: f64,
    #[serde(default)]
    pub local_origin_y: f64,
    #[serde(default = "default_true")]
    pub snap_to_geometry: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialSettings {
    pub width: f64,
    pub height: f64,
    pub offset_x: f64,
    pub offset_y: f64,
    #[serde(default)]
    pub edge_margin_x: f64,
    #[serde(default)]
    pub edge_margin_y: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplaySettings {
    pub background: ColorStyle,
    pub grid: GridStyle,
    pub axes: AxesStyle,
    #[serde(default)]
    pub local_axes: AxesStyle,
    pub material: MaterialDisplaySettings,
    pub source: SourceDisplaySettings,
    pub frame: FrameDisplaySettings,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorStyle {
    pub color: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualStyle {
    #[serde(default = "default_true")]
    pub visible: bool,
    pub color: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridStyle {
    #[serde(flatten)]
    pub style: VisualStyle,
    pub label_color: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxesStyle {
    #[serde(default = "default_true")]
    pub visible: bool,
    pub x_color: String,
    pub y_color: String,
}
impl Default for AxesStyle {
    fn default() -> Self {
        Self {
            visible: true,
            x_color: "#dc4646".into(),
            y_color: "#46d278".into(),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialDisplaySettings {
    pub outline: VisualStyle,
    pub safe_area: VisualStyle,
    pub margin_hatch: VisualStyle,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceDisplaySettings {
    pub toolpath: VisualStyle,
    pub rapid_moves: VisualStyle,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameDisplaySettings {
    pub outline: VisualStyle,
    pub expanded_area: VisualStyle,
    pub tabs: VisualStyle,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachiningSettings {
    pub tool_diameter: f64,
    pub cut_depth: f64,
    pub step_depth: f64,
    pub feed_rate: f64,
    pub spindle_speed: f64,
}

impl Default for Settings {
    fn default() -> Self {
        let mut settings = Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            calculation: CalculationSettings::default(),
            coordinates: CoordinateSettings::default(),
            material: MaterialSettings::default(),
            display: DisplaySettings::default(),
            use_opengl_renderer: true,
            machining: MachiningSettings::default(),
            offset_x: 0.0,
            offset_y: 0.0,
            local_offset_enabled: false,
            local_offset_x: 0.0,
            local_offset_y: 0.0,
            snap_to_geometry: true,
            material_width: 0.0,
            material_height: 0.0,
            material_offset_x: 0.0,
            material_offset_y: 0.0,
            material_edge_margin_x: 0.0,
            material_edge_margin_y: 0.0,
            show_grid: false,
            show_axes: false,
            show_local_axes: false,
            show_material: false,
            show_safe_area: false,
            show_margin_hatch: false,
            material_color: String::new(),
            safe_area_color: String::new(),
            frame_color: String::new(),
            background_color: String::new(),
            grid_color: String::new(),
            grid_label_color: String::new(),
            axis_x_color: String::new(),
            axis_y_color: String::new(),
            local_axis_x_color: String::new(),
            local_axis_y_color: String::new(),
            toolpath_color: String::new(),
            rapid_color: String::new(),
            expanded_frame_color: String::new(),
            tab_color: String::new(),
            margin_hatch_color: String::new(),
            show_toolpath: false,
            show_rapid: false,
            show_expanded_frame: false,
            show_frame: false,
            show_tabs: false,
            tab_width: 0.0,
            minimum_tabs: 0,
            maximum_tab_gap: 0.0,
            score_tabs: false,
            tool_diameter: 0.0,
            cut_depth: 0.0,
            step_depth: 0.0,
            source_cut_depths: Vec::new(),
            feed_rate: 0.0,
            spindle_speed: 0.0,
        };
        settings.populate_runtime();
        settings
    }
}
fn default_use_opengl_renderer() -> bool {
    true
}

impl Default for CalculationSettings {
    fn default() -> Self {
        Self {
            frame_offset_x: 10.0,
            frame_offset_y: 10.0,
            tabs: TabSettings::default(),
        }
    }
}
impl Default for TabSettings {
    fn default() -> Self {
        Self {
            width: 3.0,
            minimum_count: 3,
            maximum_gap: 20.0,
            score_to_half_depth: false,
        }
    }
}
impl Default for CoordinateSettings {
    fn default() -> Self {
        Self {
            local_origin_enabled: false,
            local_origin_x: 0.0,
            local_origin_y: 0.0,
            snap_to_geometry: true,
        }
    }
}
impl Default for MaterialSettings {
    fn default() -> Self {
        Self {
            width: 150.0,
            height: 150.0,
            offset_x: 0.0,
            offset_y: 0.0,
            edge_margin_x: 0.0,
            edge_margin_y: 0.0,
        }
    }
}
impl Default for DisplaySettings {
    fn default() -> Self {
        Self {
            background: ColorStyle::new("#0e1116"),
            grid: GridStyle {
                style: VisualStyle::new("#2a303a"),
                label_color: "#96a0af".into(),
            },
            axes: AxesStyle {
                visible: true,
                x_color: "#dc4646".into(),
                y_color: "#46d278".into(),
            },
            local_axes: AxesStyle {
                visible: true,
                x_color: "#dc4646".into(),
                y_color: "#46d278".into(),
            },
            material: MaterialDisplaySettings {
                outline: VisualStyle::new("#be64ff"),
                safe_area: VisualStyle::new("#ff4646"),
                margin_hatch: VisualStyle::new("#ff4646"),
            },
            source: SourceDisplaySettings {
                toolpath: VisualStyle::new("#00d2ff"),
                rapid_moves: VisualStyle::new("#ffbe00"),
            },
            frame: FrameDisplaySettings {
                outline: VisualStyle::new("#ff4664"),
                expanded_area: VisualStyle::new("#ffd200"),
                tabs: VisualStyle::new("#ffdc46"),
            },
        }
    }
}
impl ColorStyle {
    fn new(color: &str) -> Self {
        Self {
            color: color.into(),
        }
    }
}
impl VisualStyle {
    fn new(color: &str) -> Self {
        Self {
            visible: true,
            color: color.into(),
        }
    }
}
impl Default for MachiningSettings {
    fn default() -> Self {
        Self {
            tool_diameter: 3.175,
            cut_depth: 1.6,
            step_depth: 0.4,
            feed_rate: 120.0,
            spindle_speed: 800.0,
        }
    }
}

impl Settings {
    fn populate_runtime(&mut self) {
        self.offset_x = self.calculation.frame_offset_x;
        self.offset_y = self.calculation.frame_offset_y;
        self.tab_width = self.calculation.tabs.width;
        self.minimum_tabs = self.calculation.tabs.minimum_count;
        self.maximum_tab_gap = self.calculation.tabs.maximum_gap;
        self.score_tabs = self.calculation.tabs.score_to_half_depth;
        self.local_offset_enabled = self.coordinates.local_origin_enabled;
        self.local_offset_x = self.coordinates.local_origin_x;
        self.local_offset_y = self.coordinates.local_origin_y;
        self.snap_to_geometry = self.coordinates.snap_to_geometry;
        self.material_width = self.material.width;
        self.material_height = self.material.height;
        self.material_offset_x = self.material.offset_x;
        self.material_offset_y = self.material.offset_y;
        self.material_edge_margin_x = self.material.edge_margin_x;
        self.material_edge_margin_y = self.material.edge_margin_y;
        self.show_grid = self.display.grid.style.visible;
        self.grid_color = self.display.grid.style.color.clone();
        self.grid_label_color = self.display.grid.label_color.clone();
        self.show_axes = self.display.axes.visible;
        self.axis_x_color = self.display.axes.x_color.clone();
        self.axis_y_color = self.display.axes.y_color.clone();
        self.show_local_axes = self.display.local_axes.visible;
        self.local_axis_x_color = self.display.local_axes.x_color.clone();
        self.local_axis_y_color = self.display.local_axes.y_color.clone();
        self.background_color = self.display.background.color.clone();
        self.show_material = self.display.material.outline.visible;
        self.material_color = self.display.material.outline.color.clone();
        self.show_safe_area = self.display.material.safe_area.visible;
        self.safe_area_color = self.display.material.safe_area.color.clone();
        self.show_margin_hatch = self.display.material.margin_hatch.visible;
        self.margin_hatch_color = self.display.material.margin_hatch.color.clone();
        self.show_toolpath = self.display.source.toolpath.visible;
        self.toolpath_color = self.display.source.toolpath.color.clone();
        self.show_rapid = self.display.source.rapid_moves.visible;
        self.rapid_color = self.display.source.rapid_moves.color.clone();
        self.show_frame = self.display.frame.outline.visible;
        self.frame_color = self.display.frame.outline.color.clone();
        self.show_expanded_frame = self.display.frame.expanded_area.visible;
        self.expanded_frame_color = self.display.frame.expanded_area.color.clone();
        self.show_tabs = self.display.frame.tabs.visible;
        self.tab_color = self.display.frame.tabs.color.clone();
        self.tool_diameter = self.machining.tool_diameter;
        self.cut_depth = self.machining.cut_depth;
        self.step_depth = self.machining.step_depth;
        self.feed_rate = self.machining.feed_rate;
        self.spindle_speed = self.machining.spindle_speed;
    }
    fn sync_persisted(&mut self) {
        self.calculation.frame_offset_x = self.offset_x;
        self.calculation.frame_offset_y = self.offset_y;
        self.calculation.tabs = TabSettings {
            width: self.tab_width,
            minimum_count: self.minimum_tabs,
            maximum_gap: self.maximum_tab_gap,
            score_to_half_depth: self.score_tabs,
        };
        self.coordinates = CoordinateSettings {
            local_origin_enabled: self.local_offset_enabled,
            local_origin_x: self.local_offset_x,
            local_origin_y: self.local_offset_y,
            snap_to_geometry: self.snap_to_geometry,
        };
        self.material = MaterialSettings {
            width: self.material_width,
            height: self.material_height,
            offset_x: self.material_offset_x,
            offset_y: self.material_offset_y,
            edge_margin_x: self.material_edge_margin_x,
            edge_margin_y: self.material_edge_margin_y,
        };
        self.display = DisplaySettings {
            background: ColorStyle {
                color: self.background_color.clone(),
            },
            grid: GridStyle {
                style: VisualStyle {
                    visible: self.show_grid,
                    color: self.grid_color.clone(),
                },
                label_color: self.grid_label_color.clone(),
            },
            axes: AxesStyle {
                visible: self.show_axes,
                x_color: self.axis_x_color.clone(),
                y_color: self.axis_y_color.clone(),
            },
            local_axes: AxesStyle {
                visible: self.show_local_axes,
                x_color: self.local_axis_x_color.clone(),
                y_color: self.local_axis_y_color.clone(),
            },
            material: MaterialDisplaySettings {
                outline: VisualStyle {
                    visible: self.show_material,
                    color: self.material_color.clone(),
                },
                safe_area: VisualStyle {
                    visible: self.show_safe_area,
                    color: self.safe_area_color.clone(),
                },
                margin_hatch: VisualStyle {
                    visible: self.show_margin_hatch,
                    color: self.margin_hatch_color.clone(),
                },
            },
            source: SourceDisplaySettings {
                toolpath: VisualStyle {
                    visible: self.show_toolpath,
                    color: self.toolpath_color.clone(),
                },
                rapid_moves: VisualStyle {
                    visible: self.show_rapid,
                    color: self.rapid_color.clone(),
                },
            },
            frame: FrameDisplaySettings {
                outline: VisualStyle {
                    visible: self.show_frame,
                    color: self.frame_color.clone(),
                },
                expanded_area: VisualStyle {
                    visible: self.show_expanded_frame,
                    color: self.expanded_frame_color.clone(),
                },
                tabs: VisualStyle {
                    visible: self.show_tabs,
                    color: self.tab_color.clone(),
                },
            },
        };
        self.machining = MachiningSettings {
            tool_diameter: self.tool_diameter,
            cut_depth: self.cut_depth,
            step_depth: self.step_depth,
            feed_rate: self.feed_rate,
            spindle_speed: self.spindle_speed,
        };
    }
    pub fn load() -> Result<Self, SettingsError> {
        let path = std::path::PathBuf::from("settings.json");
        if !path.exists() {
            let settings = Self::default();
            settings.save()?;
            return Ok(settings);
        }
        let content =
            std::fs::read_to_string(&path).map_err(|e| SettingsError::Read(e.to_string()))?;
        let value: serde_json::Value =
            serde_json::from_str(&content).map_err(|e| SettingsError::Parse(e.to_string()))?;
        if value.get("schema_version").is_some() {
            let mut settings: Self =
                serde_json::from_value(value).map_err(|e| SettingsError::Parse(e.to_string()))?;
            settings.populate_runtime();
            return Ok(settings);
        }
        let mut settings = LegacySettings::from_value(value)?;
        settings.populate_runtime();
        settings.save()?;
        Ok(settings)
    }
    pub fn save(&self) -> Result<(), SettingsError> {
        let mut saved = self.clone();
        saved.sync_persisted();
        saved.schema_version = CURRENT_SCHEMA_VERSION;
        saved.calculation.frame_offset_x = round_six(saved.calculation.frame_offset_x);
        saved.calculation.frame_offset_y = round_six(saved.calculation.frame_offset_y);
        saved.calculation.tabs.width = round_six(saved.calculation.tabs.width);
        saved.calculation.tabs.maximum_gap = round_six(saved.calculation.tabs.maximum_gap);
        saved.coordinates.local_origin_x = round_six(saved.coordinates.local_origin_x);
        saved.coordinates.local_origin_y = round_six(saved.coordinates.local_origin_y);
        saved.material.width = round_six(saved.material.width);
        saved.material.height = round_six(saved.material.height);
        saved.material.offset_x = round_six(saved.material.offset_x);
        saved.material.offset_y = round_six(saved.material.offset_y);
        saved.material.edge_margin_x = round_six(saved.material.edge_margin_x);
        saved.material.edge_margin_y = round_six(saved.material.edge_margin_y);
        let content = serde_json::to_string_pretty(&saved)
            .map_err(|e| SettingsError::Serialize(e.to_string()))?;
        std::fs::write("settings.json", content).map_err(|e| SettingsError::Write(e.to_string()))
    }
    pub fn local_offset(&self) -> (f64, f64) {
        if self.local_offset_enabled {
            (self.local_offset_x, self.local_offset_y)
        } else {
            (0.0, 0.0)
        }
    }
}

#[derive(Debug, Deserialize)]
struct LegacySettings {
    #[serde(default = "default_offset")]
    offset_x: f64,
    #[serde(default = "default_offset")]
    offset_y: f64,
    #[serde(default)]
    local_offset_enabled: bool,
    #[serde(default)]
    local_offset_x: f64,
    #[serde(default)]
    local_offset_y: f64,
    #[serde(default = "default_material_width")]
    material_width: f64,
    #[serde(default = "default_material_width")]
    material_height: f64,
    #[serde(default)]
    material_offset_x: f64,
    #[serde(default)]
    material_offset_y: f64,
    #[serde(default)]
    material_edge_margin_x: f64,
    #[serde(default)]
    material_edge_margin_y: f64,
    #[serde(default = "default_true")]
    show_grid: bool,
    #[serde(default = "default_true")]
    show_axes: bool,
    #[serde(default = "default_true")]
    show_material: bool,
    #[serde(default = "default_true")]
    show_safe_area: bool,
    #[serde(default = "default_true")]
    show_margin_hatch: bool,
    #[serde(default = "default_material_color")]
    material_color: String,
    #[serde(default = "default_safe_area_color")]
    safe_area_color: String,
    #[serde(default = "default_frame_color")]
    frame_color: String,
    #[serde(default = "default_background_color")]
    background_color: String,
    #[serde(default = "default_grid_color")]
    grid_color: String,
    #[serde(default = "default_grid_label_color")]
    grid_label_color: String,
    #[serde(default = "default_axis_x_color")]
    axis_x_color: String,
    #[serde(default = "default_axis_y_color")]
    axis_y_color: String,
    #[serde(default = "default_toolpath_color")]
    toolpath_color: String,
    #[serde(default = "default_rapid_color")]
    rapid_color: String,
    #[serde(default = "default_expanded_frame_color")]
    expanded_frame_color: String,
    #[serde(default = "default_tab_color")]
    tab_color: String,
    #[serde(default = "default_margin_hatch_color")]
    margin_hatch_color: String,
    #[serde(default = "default_true")]
    show_toolpath: bool,
    #[serde(default = "default_true")]
    show_rapid: bool,
    #[serde(default = "default_true")]
    show_expanded_frame: bool,
    #[serde(default = "default_true")]
    show_frame: bool,
    #[serde(default = "default_true")]
    show_tabs: bool,
    #[serde(default = "default_tab_width")]
    tab_width: f64,
    #[serde(default = "default_minimum_tabs")]
    minimum_tabs: usize,
    #[serde(default = "default_maximum_tab_gap")]
    maximum_tab_gap: f64,
    #[serde(default)]
    score_tabs: bool,
}
impl LegacySettings {
    fn from_value(value: serde_json::Value) -> Result<Settings, SettingsError> {
        let legacy: Self =
            serde_json::from_value(value).map_err(|e| SettingsError::Parse(e.to_string()))?;
        Ok(Settings {
            calculation: CalculationSettings {
                frame_offset_x: legacy.offset_x,
                frame_offset_y: legacy.offset_y,
                tabs: TabSettings {
                    width: legacy.tab_width,
                    minimum_count: legacy.minimum_tabs,
                    maximum_gap: legacy.maximum_tab_gap,
                    score_to_half_depth: legacy.score_tabs,
                },
            },
            coordinates: CoordinateSettings {
                local_origin_enabled: legacy.local_offset_enabled,
                local_origin_x: legacy.local_offset_x,
                local_origin_y: legacy.local_offset_y,
                snap_to_geometry: true,
            },
            material: MaterialSettings {
                width: legacy.material_width,
                height: legacy.material_height,
                offset_x: legacy.material_offset_x,
                offset_y: legacy.material_offset_y,
                edge_margin_x: legacy.material_edge_margin_x,
                edge_margin_y: legacy.material_edge_margin_y,
            },
            display: DisplaySettings {
                background: ColorStyle {
                    color: legacy.background_color,
                },
                grid: GridStyle {
                    style: VisualStyle {
                        visible: legacy.show_grid,
                        color: legacy.grid_color,
                    },
                    label_color: legacy.grid_label_color,
                },
                axes: AxesStyle {
                    visible: legacy.show_axes,
                    x_color: legacy.axis_x_color.clone(),
                    y_color: legacy.axis_y_color.clone(),
                },
                local_axes: AxesStyle {
                    visible: true,
                    x_color: legacy.axis_x_color,
                    y_color: legacy.axis_y_color,
                },
                material: MaterialDisplaySettings {
                    outline: VisualStyle {
                        visible: legacy.show_material,
                        color: legacy.material_color,
                    },
                    safe_area: VisualStyle {
                        visible: legacy.show_safe_area,
                        color: legacy.safe_area_color,
                    },
                    margin_hatch: VisualStyle {
                        visible: legacy.show_margin_hatch,
                        color: legacy.margin_hatch_color,
                    },
                },
                source: SourceDisplaySettings {
                    toolpath: VisualStyle {
                        visible: legacy.show_toolpath,
                        color: legacy.toolpath_color,
                    },
                    rapid_moves: VisualStyle {
                        visible: legacy.show_rapid,
                        color: legacy.rapid_color,
                    },
                },
                frame: FrameDisplaySettings {
                    outline: VisualStyle {
                        visible: legacy.show_frame,
                        color: legacy.frame_color,
                    },
                    expanded_area: VisualStyle {
                        visible: legacy.show_expanded_frame,
                        color: legacy.expanded_frame_color,
                    },
                    tabs: VisualStyle {
                        visible: legacy.show_tabs,
                        color: legacy.tab_color,
                    },
                },
            },
            ..Settings::default()
        })
    }
}

fn round_six(value: f64) -> f64 {
    const DECIMAL_PLACES: f64 = 1_000_000.0;
    (value * DECIMAL_PLACES).round() / DECIMAL_PLACES
}
fn default_true() -> bool {
    true
}
fn default_offset() -> f64 {
    10.0
}
fn default_material_width() -> f64 {
    150.0
}
fn default_tab_width() -> f64 {
    3.0
}
fn default_minimum_tabs() -> usize {
    3
}
fn default_maximum_tab_gap() -> f64 {
    20.0
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_six_decimal_places_when_persisting_values() {
        assert_eq!(round_six(12.123_456_7), 12.123_457);
    }

    #[test]
    fn legacy_settings_are_converted_to_the_nested_schema() {
        let legacy = serde_json::json!({
            "offset_x": 4.5,
            "material_width": 120.0,
            "show_grid": false,
            "toolpath_color": "#123456"
        });
        let mut settings = LegacySettings::from_value(legacy).expect("legacy settings parse");
        settings.populate_runtime();
        settings.sync_persisted();

        let saved = serde_json::to_value(settings).expect("settings serialize");
        assert_eq!(saved["schema_version"], CURRENT_SCHEMA_VERSION);
        assert_eq!(saved["calculation"]["frame_offset_x"], 4.5);
        assert_eq!(saved["material"]["width"], 120.0);
        assert_eq!(saved["display"]["grid"]["visible"], false);
        assert_eq!(saved["display"]["source"]["toolpath"]["color"], "#123456");
        assert!(saved.get("offset_x").is_none());
    }
}
