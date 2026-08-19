use crate::settings::Settings;

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
        self.x_min.is_finite() && self.x_max.is_finite()
            && self.y_min.is_finite() && self.y_max.is_finite()
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
    /// Calculate frame geometry based on board bounds and settings
    pub fn calculate(bounds: &BoardBounds, settings: &Settings) -> Option<Self> {
        if !bounds.is_valid() {
            return None;
        }

        let margin = settings.total_margin();

        // Machine coordinate system:
        // Origin at bottom-right
        // X: negative to the left
        // Y: positive upward

        // Right boundary (closer to origin, less negative)
        let right = -(settings.offset_x + margin);

        // Bottom boundary
        let bottom = settings.offset_y + margin;

        // Left boundary (more negative)
        let left = right - bounds.width();

        // Top boundary
        let top = bottom + bounds.height();

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

    /// Get all corner points in order (for drawing)
    pub fn corners(&self) -> Vec<(f64, f64)> {
        vec![
            (self.left, self.bottom),
            (self.right, self.bottom),
            (self.right, self.top),
            (self.left, self.top),
            (self.left, self.bottom), // Close the loop
        ]
    }
}

/// Extract board bounds from G-code content
pub fn parse_gcode_bounds(content: &str) -> BoardBounds {
    let mut bounds = BoardBounds::new();

    for line in content.lines() {
        let line = line.trim().to_uppercase();

        // Skip comments and empty lines
        if line.is_empty() || line.starts_with(';') || line.starts_with('(') {
            continue;
        }

        // Parse G0/G1 commands with X/Y coordinates
        if line.starts_with("G0") || line.starts_with("G1") {
            let x = extract_coordinate(&line, 'X');
            let y = extract_coordinate(&line, 'Y');

            if let Some(x_val) = x {
                if let Some(y_val) = y {
                    bounds.update(x_val, y_val);
                }
            }
        }
    }

    bounds
}

/// Extract a coordinate value from a G-code line
fn extract_coordinate(line: &str, axis: char) -> Option<f64> {
    // Find the axis letter
    let pos = line.find(axis)?;

    // Get the substring after the axis letter
    let start = pos + 1;

    // Find the end of this number (next letter or end of string)
    let rest = &line[start..];
    let end = rest
        .find(|c: char| c.is_ascii_alphabetic())
        .unwrap_or(rest.len());

    let num_str = rest[..end].trim();

    num_str.parse::<f64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_gcode() {
        let gcode = "G0 X0 Y0\nG1 X10 Y5\nG1 X20 Y10";
        let bounds = parse_gcode_bounds(gcode);

        assert!((bounds.x_min - 0.0).abs() < 0.001);
        assert!((bounds.x_max - 20.0).abs() < 0.001);
        assert!((bounds.y_min - 0.0).abs() < 0.001);
        assert!((bounds.y_max - 10.0).abs() < 0.001);
    }

    #[test]
    fn test_extract_coordinate() {
        assert_eq!(extract_coordinate("G1 X10.5 Y20", 'X'), Some(10.5));
        assert_eq!(extract_coordinate("G1 X10.5 Y20", 'Y'), Some(20.0));
        assert_eq!(extract_coordinate("G0 X-5", 'X'), Some(-5.0));
    }
}
