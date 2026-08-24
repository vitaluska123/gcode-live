/// Preview data for rendering board and frame
#[derive(Debug, Clone)]
pub struct PreviewData {
    /// Board bounds (original PCB outline)
    pub board_x_min: f64,
    pub board_x_max: f64,
    pub board_y_min: f64,
    pub board_y_max: f64,
    /// Frame bounds
    pub frame_left: f64,
    pub frame_right: f64,
    pub frame_bottom: f64,
    pub frame_top: f64,
    pub material_width: f64,
    pub material_height: f64,
    pub material_offset_x: f64,
    pub material_offset_y: f64,
    /// Whether we have valid data to display
    pub has_data: bool,
}

impl Default for PreviewData {
    fn default() -> Self {
        Self {
            board_x_min: 0.0,
            board_x_max: 0.0,
            board_y_min: 0.0,
            board_y_max: 0.0,
            frame_left: 0.0,
            frame_right: 0.0,
            frame_bottom: 0.0,
            frame_top: 0.0,
            material_width: 0.0,
            material_height: 0.0,
            material_offset_x: 0.0,
            material_offset_y: 0.0,
            has_data: false,
        }
    }
}

impl PreviewData {
    #[allow(clippy::too_many_arguments)] // Mirrors the independently supplied preview bounds.
    pub fn from_bounds_with_material(
        board_x_min: f64,
        board_x_max: f64,
        board_y_min: f64,
        board_y_max: f64,
        frame_left: f64,
        frame_right: f64,
        frame_bottom: f64,
        frame_top: f64,
        material_width: f64,
        material_height: f64,
        material_offset_x: f64,
        material_offset_y: f64,
    ) -> Self {
        Self {
            board_x_min,
            board_x_max,
            board_y_min,
            board_y_max,
            frame_left,
            frame_right,
            frame_bottom,
            frame_top,
            material_width: material_width.max(0.0),
            material_height: material_height.max(0.0),
            material_offset_x,
            material_offset_y,
            has_data: true,
        }
    }

    pub fn world_bounds(&self) -> (f64, f64, f64, f64) {
        (
            self.board_x_min
                .min(self.frame_left)
                .min(self.material_offset_x - self.material_width),
            self.board_x_max
                .max(self.frame_right)
                .max(self.material_offset_x),
            self.board_y_min
                .min(self.frame_bottom)
                .min(self.material_offset_y),
            self.board_y_max
                .max(self.frame_top)
                .max(self.material_offset_y + self.material_height),
        )
    }

    /// Calculate scaling factor to fit content in widget
    pub fn calculate_scale(&self, widget_width: f32, widget_height: f32) -> f64 {
        if !self.has_data {
            return 1.0;
        }

        // Find overall bounds including both board and frame
        let (all_x_min, all_x_max, all_y_min, all_y_max) = self.world_bounds();

        let content_width = (all_x_max - all_x_min).abs();
        let content_height = all_y_max - all_y_min;

        if content_width <= 0.0 || content_height <= 0.0 {
            return 1.0;
        }

        let padding = 20.0; // pixels
        let avail_width = widget_width as f64 - padding * 2.0;
        let avail_height = widget_height as f64 - padding * 2.0;

        let scale_x = avail_width / content_width;
        let scale_y = avail_height / content_height;

        scale_x.min(scale_y)
    }

    /// Transform world coordinates to screen coordinates
    pub fn world_to_screen(
        &self,
        x: f64,
        y: f64,
        scale: f64,
        widget_width: f32,
        widget_height: f32,
    ) -> (f32, f32) {
        // Find overall bounds for centering
        let (all_x_min, all_x_max, all_y_min, all_y_max) = self.world_bounds();

        let content_width = all_x_max - all_x_min;
        let content_height = all_y_max - all_y_min;
        let content_screen_width = content_width * scale;
        let content_screen_height = content_height * scale;

        // Keep both axes centered, then flip Y for screen coordinates.
        let sx =
            ((widget_width as f64 - content_screen_width) / 2.0 + (x - all_x_min) * scale) as f32;
        // Flip Y axis for screen coordinates (screen Y goes down)
        let sy =
            ((widget_height as f64 + content_screen_height) / 2.0 - (y - all_y_min) * scale) as f32;

        (sx, sy)
    }
}
