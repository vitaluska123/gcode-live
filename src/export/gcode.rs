use crate::domain::frame::{top_tab_intervals, BoardBounds, FrameGeometry};
use crate::domain::settings::Settings;

/// Generate G-code for the frame cutting operation
pub fn generate_frame_gcode(
    _bounds: &BoardBounds,
    frame: &FrameGeometry,
    settings: &Settings,
    source_home: Option<(f64, f64)>,
) -> String {
    let mut gcode = String::new();
    let (shift_x, shift_y) = settings.local_offset();

    // Header
    gcode.push_str("G21 G17 G90\n");
    gcode.push_str(&format!("M3 S{}\n", settings.spindle_speed as i64));
    gcode.push('\n');

    let corner_radius = (settings.tool_diameter / 2.0)
        .min(1.0)
        .min(frame.width() / 4.0)
        .min(frame.height() / 4.0)
        .max(0.0);
    let path = rounded_rectangle_path(frame, corner_radius, 6);
    let top_tabs = top_tab_intervals(frame, corner_radius, settings);
    let Some(&(start_x, start_y)) = path.first() else {
        return gcode;
    };

    // Rapid to the first point of the rounded frame at a safe height.
    gcode.push_str("G0 Z5\n");
    gcode.push_str(&format!(
        "G0 X{:.3} Y{:.3}\n",
        start_x + shift_x,
        start_y + shift_y
    ));
    gcode.push('\n');

    // Calculate depth passes
    let cut_depth = settings.cut_depth.abs();
    let step_depth = settings.step_depth.abs();
    if cut_depth == 0.0 || step_depth == 0.0 {
        gcode.push_str("G0 Z5\nM5\nM30\n");
        return gcode;
    }
    // A tiny floating-point residue (for example 1.8 / 0.36) must not create
    // an extra, identical final pass.
    let num_passes = ((cut_depth / step_depth) - 1e-9).ceil() as usize;

    for pass in 1..=num_passes {
        let depth = (pass as f64 * step_depth).min(cut_depth);

        // Plunge to cut depth
        if pass == 1 {
            gcode.push_str(&format!("G1 Z-{depth:.3} F{}\n", settings.feed_rate as i64));
        } else {
            gcode.push_str(&format!("G1 Z-{depth:.3}\n"));
        }

        // Follow every short segment so the corners remain rounded on controllers
        // that accept only G0/G1 moves.
        for segment in path.windows(2) {
            let (from_x, from_y) = segment[0];
            let (to_x, to_y) = segment[1];
            if (from_y - frame.top).abs() < f64::EPSILON && (to_y - frame.top).abs() < f64::EPSILON
            {
                if settings.score_tabs && depth <= cut_depth / 2.0 + f64::EPSILON {
                    // Score tab locations during the shallow passes, then leave
                    // their lower half intact to keep the board attached.
                    gcode.push_str(&format!("X{:.3} Y{:.3}\n", to_x + shift_x, to_y + shift_y));
                } else {
                    append_top_edge_with_tabs(
                        &mut gcode, from_x, to_x, frame.top, depth, &top_tabs, shift_x, shift_y,
                    );
                }
            } else {
                // G1 stays modal after the plunge: ordinary moves need only
                // coordinates, just like CAM-generated TAP programs.
                gcode.push_str(&format!("X{:.3} Y{:.3}\n", to_x + shift_x, to_y + shift_y));
            }
        }

        gcode.push('\n');
    }

    // Retract and end
    gcode.push_str("G0 Z5\n");
    if let Some((home_x, home_y)) = source_home {
        gcode.push_str(&format!(
            "G0 X{:.3} Y{:.3}\n",
            home_x + shift_x,
            home_y + shift_y
        ));
    }
    gcode.push_str("M5\n");
    gcode.push_str("M30\n");

    gcode
}

#[allow(clippy::too_many_arguments)] // G-code segment coordinates are intentionally explicit.
fn append_top_edge_with_tabs(
    gcode: &mut String,
    from_x: f64,
    to_x: f64,
    y: f64,
    depth: f64,
    tabs: &[(f64, f64)],
    shift_x: f64,
    shift_y: f64,
) {
    let mut cursor_x = from_x;
    let reverse = to_x < from_x;
    let mut ordered_tabs = tabs.to_vec();
    if reverse {
        ordered_tabs.reverse();
    }

    for &(left, right) in &ordered_tabs {
        let (entry, exit) = if reverse {
            (right, left)
        } else {
            (left, right)
        };
        if (entry - cursor_x).abs() > f64::EPSILON {
            gcode.push_str(&format!("X{:.3} Y{:.3}\n", entry + shift_x, y + shift_y));
        }
        // Match the source TAP pattern: lift above the board, cross the tab,
        // return to the board surface, then resume the current cutting depth.
        gcode.push_str("G0 Z2\n");
        gcode.push_str(&format!("X{:.3} Y{:.3}\n", exit + shift_x, y + shift_y));
        gcode.push_str("Z0\n");
        gcode.push_str(&format!("G1 Z-{depth:.3}\n"));
        cursor_x = exit;
    }
    if (to_x - cursor_x).abs() > f64::EPSILON {
        gcode.push_str(&format!("X{:.3} Y{:.3}\n", to_x + shift_x, y + shift_y));
    }
}

fn rounded_rectangle_path(
    frame: &FrameGeometry,
    radius: f64,
    corner_segments: usize,
) -> Vec<(f64, f64)> {
    if radius == 0.0 || corner_segments == 0 {
        return vec![
            (frame.left, frame.bottom),
            (frame.right, frame.bottom),
            (frame.right, frame.top),
            (frame.left, frame.top),
            (frame.left, frame.bottom),
        ];
    }

    let mut path = vec![
        (frame.left + radius, frame.bottom),
        (frame.right - radius, frame.bottom),
    ];
    append_arc(
        &mut path,
        frame.right - radius,
        frame.bottom + radius,
        -90.0,
        0.0,
        radius,
        corner_segments,
    );
    path.push((frame.right, frame.top - radius));
    append_arc(
        &mut path,
        frame.right - radius,
        frame.top - radius,
        0.0,
        90.0,
        radius,
        corner_segments,
    );
    path.push((frame.left + radius, frame.top));
    append_arc(
        &mut path,
        frame.left + radius,
        frame.top - radius,
        90.0,
        180.0,
        radius,
        corner_segments,
    );
    path.push((frame.left, frame.bottom + radius));
    append_arc(
        &mut path,
        frame.left + radius,
        frame.bottom + radius,
        180.0,
        270.0,
        radius,
        corner_segments,
    );
    path
}

fn append_arc(
    path: &mut Vec<(f64, f64)>,
    center_x: f64,
    center_y: f64,
    start_degrees: f64,
    end_degrees: f64,
    radius: f64,
    segments: usize,
) {
    for segment in 1..=segments {
        let angle = (start_degrees
            + (end_degrees - start_degrees) * segment as f64 / segments as f64)
            .to_radians();
        path.push((
            center_x + radius * angle.cos(),
            center_y + radius * angle.sin(),
        ));
    }
}

/// Save G-code content to file
pub fn save_gcode(content: &str, path: &std::path::Path) -> Result<(), ExportError> {
    std::fs::write(path, content).map_err(|e| ExportError::WriteError(e.to_string()))
}

#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("Failed to write G-code file: {0}")]
    WriteError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_contains_all_depth_passes_and_rounded_corner_points() {
        let frame = FrameGeometry {
            left: 0.0,
            right: 20.0,
            bottom: 0.0,
            top: 10.0,
        };
        let settings = Settings {
            cut_depth: 1.0,
            step_depth: 0.4,
            tool_diameter: 2.0,
            tab_width: 3.0,
            minimum_tabs: 3,
            maximum_tab_gap: 20.0,
            ..Settings::default()
        };
        let gcode = generate_frame_gcode(
            &BoardBounds::default(),
            &frame,
            &settings,
            Some((-5.0, 5.0)),
        );

        assert!(gcode.contains("G1 Z-0.400 F120"));
        assert!(gcode.contains("G1 Z-0.800\n"));
        assert!(gcode.contains("G1 Z-1.000\n"));
        assert!(gcode.contains("X19.966 Y0.741"));
        assert!(gcode.contains("G0 X-5.000 Y5.000\nM5"));
        assert_eq!(gcode.matches("G0 Z2\n").count(), 9);
        assert_eq!(gcode.matches("Z0\n").count(), 9);
        assert!(!gcode.contains("G1 X"));
    }

    #[test]
    fn export_translates_local_program_when_local_zero_is_enabled() {
        let frame = FrameGeometry {
            left: 0.0,
            right: 10.0,
            bottom: 0.0,
            top: 5.0,
        };
        let settings = Settings {
            local_offset_enabled: true,
            local_offset_x: 25.0,
            local_offset_y: 30.0,
            tool_diameter: 0.0,
            cut_depth: 1.0,
            step_depth: 1.0,
            ..Settings::default()
        };
        let gcode =
            generate_frame_gcode(&BoardBounds::default(), &frame, &settings, Some((1.0, 2.0)));
        assert!(gcode.contains("G0 X25.000 Y30.000"));
        assert!(gcode.contains("X35.000 Y30.000"));
        assert!(gcode.contains("G0 X26.000 Y32.000"));
    }

    #[test]
    fn scored_tabs_are_cut_only_during_the_first_half_of_depth() {
        let frame = FrameGeometry {
            left: 0.0,
            right: 20.0,
            bottom: 0.0,
            top: 10.0,
        };
        let settings = Settings {
            score_tabs: true,
            tool_diameter: 0.0,
            cut_depth: 1.0,
            step_depth: 0.5,
            tab_width: 3.0,
            minimum_tabs: 3,
            ..Settings::default()
        };
        let gcode = generate_frame_gcode(&BoardBounds::default(), &frame, &settings, None);
        // Tabs are skipped only on the full-depth pass: 3 holding tabs, one lift each.
        assert_eq!(gcode.matches("G0 Z2\n").count(), 3);
    }
}
