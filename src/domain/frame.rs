use crate::domain::settings::Settings;

/// Board dimensions extracted from G-code file
#[derive(Debug, Clone, Default)]
pub struct BoardBounds {
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f64,
    pub y_max: f64,
}

impl BoardBounds {
    pub fn new() -> Self {
        Self {
            x_min: f64::INFINITY,
            x_max: f64::NEG_INFINITY,
            y_min: f64::INFINITY,
            y_max: f64::NEG_INFINITY,
        }
    }

    /// Update bounds with a new point
    pub fn update(&mut self, x: f64, y: f64) {
        if x < self.x_min {
            self.x_min = x;
        }
        if x > self.x_max {
            self.x_max = x;
        }
        if y < self.y_min {
            self.y_min = y;
        }
        if y > self.y_max {
            self.y_max = y;
        }
    }

    /// Check if any points were recorded
    pub fn is_valid(&self) -> bool {
        self.x_min.is_finite()
            && self.x_max.is_finite()
            && self.y_min.is_finite()
            && self.y_max.is_finite()
    }

    /// Calculate board width
    pub fn width(&self) -> f64 {
        (self.x_max - self.x_min).abs()
    }

    /// Calculate board height
    pub fn height(&self) -> f64 {
        self.y_max - self.y_min
    }
}

/// Frame geometry calculated from board bounds and settings
#[derive(Debug, Clone)]
pub struct FrameGeometry {
    pub left: f64,
    pub right: f64,
    pub bottom: f64,
    pub top: f64,
}

impl FrameGeometry {
    /// Symmetric reference frame: offsets are applied around all board sides.
    pub fn expanded(bounds: &BoardBounds, settings: &Settings) -> Option<Self> {
        if !bounds.is_valid() {
            return None;
        }
        let offset_x = settings.offset_x.max(0.0);
        let offset_y = settings.offset_y.max(0.0);
        Some(Self {
            left: bounds.x_min - offset_x,
            right: bounds.x_max + offset_x,
            bottom: bounds.y_min - offset_y,
            top: bounds.y_max + offset_y,
        })
    }

    /// Calculate frame geometry based on board bounds and settings
    pub fn calculate(bounds: &BoardBounds, settings: &Settings) -> Option<Self> {
        if !bounds.is_valid() {
            return None;
        }

        // The source board's right-bottom corner is the reference point.
        // Offset X grows the generated contour to the left, and offset Y grows
        // it upward, without moving the reference edges.
        // Keep the right-bottom corner fixed while matching the dimensions of
        // the symmetric reference frame (which grows by the offset on both sides).
        let left = bounds.x_min - 2.0 * settings.offset_x.max(0.0);
        let right = bounds.x_max;
        let bottom = bounds.y_min;
        let top = bounds.y_max + 2.0 * settings.offset_y.max(0.0);

        Some(Self {
            left,
            right,
            bottom,
            top,
        })
    }

    /// Get frame width
    pub fn width(&self) -> f64 {
        (self.right - self.left).abs()
    }

    /// Get frame height
    pub fn height(&self) -> f64 {
        self.top - self.bottom
    }
}

/// Uncut intervals along the straight part of the generated frame's upper edge.
/// Each pair is ordered from left to right and represents one holding tab.
pub fn top_tab_intervals(
    frame: &FrameGeometry,
    corner_radius: f64,
    settings: &Settings,
) -> Vec<(f64, f64)> {
    let left = frame.left + corner_radius.max(0.0);
    let right = frame.right - corner_radius.max(0.0);
    let usable_width = right - left;
    let tab_width = settings.tab_width.max(0.0).min(usable_width);
    if usable_width <= 0.0 || tab_width <= 0.0 {
        return Vec::new();
    }

    let mut count = settings.minimum_tabs.max(3);
    let maximum_gap = settings.maximum_tab_gap.max(0.0);
    while (usable_width - count as f64 * tab_width) / (count as f64 + 1.0) > maximum_gap {
        count += 1;
    }

    // More tabs than can physically fit would produce overlapping intervals.
    count = count.min((usable_width / tab_width).floor().max(1.0) as usize);
    let gap = ((usable_width - count as f64 * tab_width) / (count as f64 + 1.0)).max(0.0);
    (0..count)
        .map(|index| {
            let start = left + gap * (index as f64 + 1.0) + tab_width * index as f64;
            (start, start + tab_width)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_expands_board_bounds_by_offsets_and_margin() {
        let bounds = BoardBounds {
            x_min: -5.0,
            x_max: 15.0,
            y_min: 2.0,
            y_max: 12.0,
        };
        let settings = Settings {
            offset_x: 1.0,
            offset_y: 2.0,
            tool_diameter: 2.0,
            ..Settings::default()
        };
        let frame = FrameGeometry::calculate(&bounds, &settings).expect("valid bounds");

        assert_eq!(frame.left, -7.0);
        assert_eq!(frame.right, 15.0);
        assert_eq!(frame.bottom, 2.0);
        assert_eq!(frame.top, 16.0);
    }

    #[test]
    fn top_tabs_keep_gaps_within_configured_limit() {
        let frame = FrameGeometry {
            left: 0.0,
            right: 100.0,
            bottom: 0.0,
            top: 40.0,
        };
        let settings = Settings {
            tab_width: 3.0,
            minimum_tabs: 3,
            maximum_tab_gap: 20.0,
            ..Settings::default()
        };
        let tabs = top_tab_intervals(&frame, 1.0, &settings);
        assert_eq!(tabs.len(), 4);
        assert!((tabs[0].0 - 1.0) < 20.0);
        assert!((99.0 - tabs.last().expect("tab").1) < 20.0);
    }
}
