use crate::frame::{BoardBounds, FrameGeometry};
use crate::settings::Settings;

/// Generate G-code for the frame cutting operation
pub fn generate_frame_gcode(
    _bounds: &BoardBounds,
    frame: &FrameGeometry,
    settings: &Settings,
    source_home: Option<(f64, f64)>,
) -> String {
    let mut gcode = String::new();

    // Header
    gcode.push_str("G21 G17 G90\n");
    gcode.push_str(&format!("M3 S{}\n", settings.spindle_speed as i64));
    gcode.push_str("\n");

    let corner_radius = (settings.tool_diameter / 2.0)
        .min(1.0)
        .min(frame.width() / 4.0)
        .min(frame.height() / 4.0)
        .max(0.0);
    let path = rounded_rectangle_path(frame, corner_radius, 6);
    let Some(&(start_x, start_y)) = path.first() else {
        return gcode;
    };

    // Rapid to the first point of the rounded frame at a safe height.
    gcode.push_str("G0 Z5\n");
    gcode.push_str(&format!("G0 X{start_x:.3} Y{start_y:.3}\n"));
    gcode.push_str("\n");

    // Calculate depth passes
    let cut_depth = settings.cut_depth.abs();
    let step_depth = settings.step_depth.abs();
    if cut_depth == 0.0 || step_depth == 0.0 {
        gcode.push_str("G0 Z5\nM5\nM30\n");
        return gcode;
    }
    let num_passes = (cut_depth / step_depth).ceil() as usize;

    for pass in 1..=num_passes {
        let depth = (pass as f64 * step_depth).min(cut_depth);

        // Plunge to cut depth
        gcode.push_str(&format!(
            "G1 Z-{:.3} F{}\n",
            depth, settings.feed_rate as i64
        ));

        // Follow every short segment so the corners remain rounded on controllers
        // that accept only G0/G1 moves.
        for &(x, y) in path.iter().skip(1) {
            gcode.push_str(&format!("G1 X{x:.3} Y{y:.3}\n"));
        }

        gcode.push_str("\n");
    }

    // Retract and end
    gcode.push_str("G0 Z5\n");
    if let Some((home_x, home_y)) = source_home {
        gcode.push_str(&format!("G0 X{home_x:.3} Y{home_y:.3}\n"));
    }
    gcode.push_str("M5\n");
    gcode.push_str("M30\n");

    gcode
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

    let mut path = vec![(frame.left + radius, frame.bottom), (frame.right - radius, frame.bottom)];
    append_arc(&mut path, frame.right - radius, frame.bottom + radius, -90.0, 0.0, radius, corner_segments);
    path.push((frame.right, frame.top - radius));
    append_arc(&mut path, frame.right - radius, frame.top - radius, 0.0, 90.0, radius, corner_segments);
    path.push((frame.left + radius, frame.top));
    append_arc(&mut path, frame.left + radius, frame.top - radius, 90.0, 180.0, radius, corner_segments);
    path.push((frame.left, frame.bottom + radius));
    append_arc(&mut path, frame.left + radius, frame.bottom + radius, 180.0, 270.0, radius, corner_segments);
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
        let angle = (start_degrees + (end_degrees - start_degrees) * segment as f64 / segments as f64)
            .to_radians();
        path.push((center_x + radius * angle.cos(), center_y + radius * angle.sin()));
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
        let frame = FrameGeometry { left: 0.0, right: 20.0, bottom: 0.0, top: 10.0 };
        let settings = Settings { cut_depth: 1.0, step_depth: 0.4, tool_diameter: 2.0, ..Settings::default() };
        let gcode = generate_frame_gcode(
            &BoardBounds::default(),
            &frame,
            &settings,
            Some((-5.0, 5.0)),
        );

        assert!(gcode.contains("G1 Z-0.400 F120"));
        assert!(gcode.contains("G1 Z-0.800 F120"));
        assert!(gcode.contains("G1 Z-1.000 F120"));
        assert!(gcode.contains("X19.966 Y0.741"));
        assert!(gcode.contains("G0 X-5.000 Y5.000\nM5"));
    }
}
