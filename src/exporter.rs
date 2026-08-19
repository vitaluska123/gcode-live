use crate::frame::{BoardBounds, FrameGeometry};
use crate::settings::Settings;

/// Generate G-code for the frame cutting operation
pub fn generate_frame_gcode(
    bounds: &BoardBounds,
    frame: &FrameGeometry,
    settings: &Settings,
) -> String {
    let mut gcode = String::new();

    // Header
    gcode.push_str("G21 G17 G90\n");
    gcode.push_str(&format!("M3 S{}\n", settings.spindle_speed as i64));
    gcode.push_str("\n");

    // Rapid to start position (bottom-left corner, safe height)
    gcode.push_str("G0 Z5\n");
    gcode.push_str(&format!(
        "G0 X{:.3} Y{:.3}\n",
        frame.left, frame.bottom
    ));
    gcode.push_str("\n");

    // Calculate depth passes
    let num_passes = (settings.cut_depth / settings.step_depth).ceil() as i64;

    for pass in 1..=num_passes {
        let depth = (pass as f64 * settings.step_depth).min(settings.cut_depth);

        // Plunge to cut depth
        gcode.push_str(&format!(
            "G1 Z-{:.3} F{}\n",
            depth, settings.feed_rate as i64
        ));

        // Cut the rectangle (counter-clockwise)
        // From bottom-left to bottom-right
        gcode.push_str(&format!(
            "G1 X{:.3} Y{:.3}\n",
            frame.right, frame.bottom
        ));

        // From bottom-right to top-right
        gcode.push_str(&format!(
            "G1 X{:.3} Y{:.3}\n",
            frame.right, frame.top
        ));

        // From top-right to top-left
        gcode.push_str(&format!(
            "G1 X{:.3} Y{:.3}\n",
            frame.left, frame.top
        ));

        // From top-left to bottom-left (close the loop)
        gcode.push_str(&format!(
            "G1 X{:.3} Y{:.3}\n",
            frame.left, frame.bottom
        ));

        gcode.push_str("\n");
    }

    // Retract and end
    gcode.push_str("G0 Z5\n");
    gcode.push_str("M5\n");
    gcode.push_str("M30\n");

    gcode
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
