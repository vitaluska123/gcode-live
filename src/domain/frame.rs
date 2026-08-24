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

/// Read cutting values that are available in a standard TAP program.
/// Offsets and holding-tab settings remain application settings; all cutting
/// values are refreshed from the currently opened source program.
pub fn apply_source_cutting_parameters(content: &str, settings: &mut Settings) {
    let mut cutting_motion = false;
    let mut depths: Vec<f64> = Vec::new();

    for raw_line in content.lines() {
        let words = gcode_words(&strip_comments(raw_line));
        let line_is_g1 = words
            .iter()
            .any(|&(letter, value)| letter == 'G' && value == 1.0);
        for &(letter, value) in &words {
            match letter {
                'G' if value == 1.0 => cutting_motion = true,
                'G' => cutting_motion = false,
                'S' if value > 0.0 => settings.spindle_speed = value,
                'F' if cutting_motion || line_is_g1 => settings.feed_rate = value,
                'Z' if cutting_motion && value < 0.0 => depths.push(value.abs()),
                _ => {}
            }
        }
    }

    depths.sort_by(|left, right| left.total_cmp(right));
    depths.dedup_by(|left, right| (*left - *right).abs() < 0.000_001);
    if let Some(&deepest) = depths.last() {
        settings.cut_depth = deepest;
    }
    if depths.len() >= 2 {
        let step = depths
            .windows(2)
            .map(|pair| pair[1] - pair[0])
            .filter(|value| *value > 0.000_001)
            .min_by(|left, right| left.total_cmp(right));
        if let Some(step) = step {
            settings.step_depth = step;
        }
    } else if let Some(&depth) = depths.first() {
        settings.step_depth = depth;
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
        if words
            .iter()
            .any(|&(letter, value)| letter == 'M' && (value == 5.0 || value == 30.0))
        {
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
                'X' => {
                    x = Some(value);
                    changed = true;
                }
                'Y' => {
                    y = Some(value);
                    changed = true;
                }
                _ => {}
            }
        }
        if cutting && changed {
            if let (Some(x), Some(y)) = (x, y) {
                points.push((x, y));
            }
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
                'X' => {
                    x = Some(value);
                    changed = true;
                }
                'Y' => {
                    y = Some(value);
                    changed = true;
                }
                _ => {}
            }
        }
        if rapid && changed {
            if let (Some(x), Some(y)) = (x, y) {
                points.push((x, y));
            }
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
    fn finds_the_source_parking_position() {
        assert_eq!(
            parse_gcode_home_position("G0 X-10 Y2\nG0 Z5\nX-5Y5\nM5\nM30"),
            Some((-5.0, 5.0))
        );
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

    #[test]
    fn reads_cutting_values_from_source_program() {
        let mut settings = Settings::default();
        apply_source_cutting_parameters("M3 S12000\nG1 Z-0.4 F150\nZ-0.8\nZ-1.2", &mut settings);
        assert_eq!(settings.spindle_speed, 12000.0);
        assert_eq!(settings.feed_rate, 150.0);
        assert_eq!(settings.cut_depth, 1.2);
        assert!((settings.step_depth - 0.4).abs() < 0.000_001);
    }
}
