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

        // The source board's right-bottom corner is the reference point.
        // Offset X grows the generated contour to the left, and offset Y grows
        // it upward, without moving the reference edges.
        let left = bounds.x_min - settings.offset_x.max(0.0);
        let right = bounds.x_max;
        let bottom = bounds.y_min;
        let top = bounds.y_max + settings.offset_y.max(0.0);

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

/// Extract board bounds from G-code content
pub fn parse_gcode_bounds(content: &str) -> BoardBounds {
    let mut bounds = BoardBounds::new();
    let mut current_x = None;
    let mut current_y = None;
    let mut cutting_motion = false;

    for raw_line in content.lines() {
        let line = strip_comments(raw_line);
        let mut has_coordinate = false;

        for (letter, value) in gcode_words(&line) {

            match letter {
                'G' if value == 1.0 => cutting_motion = true,
                'G' => cutting_motion = false,
                'X' => {
                    current_x = Some(value);
                    has_coordinate = true;
                }
                'Y' => {
                    current_y = Some(value);
                    has_coordinate = true;
                }
                _ => {}
            }
        }

        if cutting_motion && has_coordinate {
            if let (Some(x), Some(y)) = (current_x, current_y) {
                bounds.update(x, y);
            }
        }
    }

    bounds
}

/// Find the final XY position before the source program stops the spindle or ends.
/// This is the source file's parking/home position and is preserved on export.
pub fn parse_gcode_home_position(content: &str) -> Option<(f64, f64)> {
    let mut current_x = None;
    let mut current_y = None;

    for raw_line in content.lines() {
        let words = gcode_words(&strip_comments(raw_line));
        if words.iter().any(|&(letter, value)| {
            letter == 'M' && (value == 5.0 || value == 30.0)
        }) {
            break;
        }

        for (letter, value) in words {
            match letter {
                'X' => current_x = Some(value),
                'Y' => current_y = Some(value),
                _ => {}
            }
        }
    }

    Some((current_x?, current_y?))
}

/// Extract every cutting position from G1/modal G1 moves for the preview.
pub fn parse_gcode_toolpath(content: &str) -> Vec<(f64, f64)> {
    let mut points = Vec::new();
    let mut x = None;
    let mut y = None;
    let mut cutting = false;
    for raw_line in content.lines() {
        let mut changed = false;
        for (letter, value) in gcode_words(&strip_comments(raw_line)) {
            match letter {
                'G' if value == 1.0 => cutting = true,
                'G' => cutting = false,
                'X' => { x = Some(value); changed = true; }
                'Y' => { y = Some(value); changed = true; }
                _ => {}
            }
        }
        if cutting && changed {
            if let (Some(x), Some(y)) = (x, y) { points.push((x, y)); }
        }
    }
    points
}

/// Extract rapid G0 positioning moves, including the source parking move.
pub fn parse_gcode_rapid_path(content: &str) -> Vec<(f64, f64)> {
    let mut points = Vec::new();
    let mut x = None;
    let mut y = None;
    let mut rapid = false;
    for raw_line in content.lines() {
        let mut changed = false;
        for (letter, value) in gcode_words(&strip_comments(raw_line)) {
            match letter {
                'G' if value == 0.0 => rapid = true,
                'G' => rapid = false,
                'X' => { x = Some(value); changed = true; }
                'Y' => { y = Some(value); changed = true; }
                _ => {}
            }
        }
        if rapid && changed {
            if let (Some(x), Some(y)) = (x, y) { points.push((x, y)); }
        }
    }
    points
}

/// Parse contiguous or whitespace-separated G-code words, such as `G1X10Y-2`.
fn gcode_words(line: &str) -> Vec<(char, f64)> {
    let chars: Vec<char> = line.chars().collect();
    let mut words = Vec::new();
    let mut index = 0;

    while index < chars.len() {
        if !chars[index].is_ascii_alphabetic() {
            index += 1;
            continue;
        }

        let letter = chars[index].to_ascii_uppercase();
        index += 1;
        let start = index;
        while index < chars.len() && !chars[index].is_ascii_alphabetic() {
            index += 1;
        }
        let value: String = chars[start..index].iter().collect();
        if let Ok(value) = value.trim().parse::<f64>() {
            words.push((letter, value));
        }
    }

    words
}

/// Remove semicolon and parenthesized comments while preserving command words.
fn strip_comments(line: &str) -> String {
    let mut cleaned = String::new();
    let mut in_comment = false;
    for ch in line.chars() {
        match ch {
            ';' if !in_comment => break,
            '(' => in_comment = true,
            ')' => in_comment = false,
            _ if !in_comment => cleaned.push(ch),
            _ => {}
        }
    }
    cleaned
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_gcode() {
        let gcode = "G0 X0 Y0\nG1 X10 Y5\nG1 X20 Y10";
        let bounds = parse_gcode_bounds(gcode);

        assert!((bounds.x_min - 10.0).abs() < 0.001);
        assert!((bounds.x_max - 20.0).abs() < 0.001);
        assert!((bounds.y_min - 5.0).abs() < 0.001);
        assert!((bounds.y_max - 10.0).abs() < 0.001);
    }

    #[test]
    fn parses_contiguous_gcode_words() {
        let bounds = parse_gcode_bounds("G0X-5Y0\nG1X10.5Y20");
        assert_eq!(bounds.x_min, 10.5);
        assert_eq!(bounds.x_max, 10.5);
        assert_eq!(bounds.y_max, 20.0);
    }

    #[test]
    fn parses_modal_coordinates_and_inline_comments() {
        let gcode = "g0 x0 y0 ; start\nG1 X10\nY5 (move only y)\nG1 X-2";
        let bounds = parse_gcode_bounds(gcode);

        assert_eq!(bounds.x_min, -2.0);
        assert_eq!(bounds.x_max, 10.0);
        assert_eq!(bounds.y_min, 0.0);
        assert_eq!(bounds.y_max, 5.0);
    }

    #[test]
    fn frame_expands_board_bounds_by_offsets_and_margin() {
        let bounds = BoardBounds { x_min: -5.0, x_max: 15.0, y_min: 2.0, y_max: 12.0 };
        let settings = Settings { offset_x: 1.0, offset_y: 2.0, clamp_zone: 3.0, safe_zone: 4.0, tool_diameter: 2.0, ..Settings::default() };
        let frame = FrameGeometry::calculate(&bounds, &settings).expect("valid bounds");

        assert_eq!(frame.left, -6.0);
        assert_eq!(frame.right, 15.0);
        assert_eq!(frame.bottom, 2.0);
        assert_eq!(frame.top, 14.0);
    }

    #[test]
    fn finds_the_source_parking_position() {
        assert_eq!(
            parse_gcode_home_position("G0 X-10 Y2\nG0 Z5\nX-5Y5\nM5\nM30"),
            Some((-5.0, 5.0))
        );
    }
}
