//! G-code parsing used by file actions and preview preparation.

use crate::domain::frame::BoardBounds;
use crate::domain::settings::Settings;

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

    let source_cut_depths = progressive_cut_depths(depths);
    settings.source_cut_depths = source_cut_depths.clone();
    if let Some(&deepest) = source_cut_depths.last() {
        settings.cut_depth = deepest;
    }
    if source_cut_depths.len() >= 2 {
        let step = source_cut_depths
            .windows(2)
            .map(|pair| pair[1] - pair[0])
            .filter(|value| *value > 0.000_001)
            .min_by(|left, right| left.total_cmp(right));
        if let Some(step) = step {
            settings.step_depth = step;
        }
    } else if let Some(&depth) = source_cut_depths.first() {
        settings.step_depth = depth;
    }
}

/// Keep each newly reached cutting depth in source order.
///
/// Returning to a shallower depth is used for holding tabs and must not become
/// an additional frame-cutting pass.
fn progressive_cut_depths(depths: Vec<f64>) -> Vec<f64> {
    const DEPTH_COMPARISON_TOLERANCE: f64 = 0.000_001;

    let mut progressive_depths = Vec::new();
    let mut deepest = 0.0;
    for depth in depths {
        if depth > deepest + DEPTH_COMPARISON_TOLERANCE {
            progressive_depths.push(depth);
            deepest = depth;
        }
    }
    progressive_depths
}

/// Extract board bounds from G-code content.
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
    fn parses_simple_gcode() {
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
    fn finds_the_source_parking_position() {
        assert_eq!(
            parse_gcode_home_position("G0 X-10 Y2\nG0 Z5\nX-5Y5\nM5\nM30"),
            Some((-5.0, 5.0))
        );
    }

    #[test]
    fn reads_cutting_values_from_source_program() {
        let mut settings = Settings::default();
        apply_source_cutting_parameters("M3 S12000\nG1 Z-0.4 F150\nZ-0.8\nZ-1.2", &mut settings);
        assert_eq!(settings.spindle_speed, 12000.0);
        assert_eq!(settings.feed_rate, 150.0);
        assert_eq!(settings.cut_depth, 1.2);
        assert!((settings.step_depth - 0.4).abs() < 0.000_001);
        assert_eq!(settings.source_cut_depths, vec![0.4, 0.8, 1.2]);
    }

    #[test]
    fn ignores_tab_lifts_when_reading_source_cut_depths() {
        let mut settings = Settings::default();
        apply_source_cutting_parameters(
            "G1 Z-0.3\nZ-0.6\nZ-0.3\nZ-0.9\nZ-0.6\nZ-1.0",
            &mut settings,
        );

        assert_eq!(settings.source_cut_depths, vec![0.3, 0.6, 0.9, 1.0]);
    }
}
