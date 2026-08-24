use std::rc::Rc;

use slint::ComponentHandle;

use crate::app::state::AppState;
use crate::domain::{frame, gcode};
use crate::export::gcode as export_gcode;
use crate::MainWindow;

// Slint's text editor eagerly lays out its entire value. Keep the editor
// responsive and avoid renderer crashes when an otherwise valid TAP is huge.
const MAX_SOURCE_EDITOR_BYTES: usize = 10 * 1024;

/// Produce editor-safe text while preserving the fact that a source was cut.
pub(crate) fn source_gcode_for_editor(content: &str) -> (String, bool) {
    if content.len() <= MAX_SOURCE_EDITOR_BYTES {
        return (content.to_owned(), false);
    }

    let max_end = content.floor_char_boundary(MAX_SOURCE_EDITOR_BYTES);
    let end = content[..max_end].rfind('\n').unwrap_or(max_end);
    (
        format!(
            "{}\n\n; --- Display limited to the first {} KB of a large source file ---\n",
            &content[..end],
            MAX_SOURCE_EDITOR_BYTES / 1024
        ),
        true,
    )
}

pub(crate) fn install_callbacks(main_window: &MainWindow, state: &AppState) {
    let board = state.preview_scene.board_bounds.clone();
    let frame_geometry = state.preview_scene.frame_geometry.clone();
    let settings = state.settings.clone();
    let home = state.source_home.clone();
    let toolpath = state.preview_scene.toolpath.clone();
    let rapid_path = state.preview_scene.rapid_path.clone();
    let stem = state.source_file_stem.clone();
    let window = main_window.as_weak();
    main_window.on_open_tap_file(move || {
        let Some(window) = window.upgrade() else {
            return;
        };
        let Some(path) = rfd::FileDialog::new()
            .add_filter("TAP Files", &["tap", "nc", "gcode", "ngc"])
            .pick_file()
        else {
            return;
        };
        let content = match std::fs::read_to_string(&path) {
            Ok(content) => content,
            Err(error) => {
                window.invoke_show_error(format!("Failed to read file: {error}").into());
                return;
            }
        };
        *stem.borrow_mut() = path
            .file_stem()
            .map(|value| value.to_string_lossy().into_owned())
            .filter(|value| !value.is_empty());
        apply_source(
            &window,
            &content,
            &board,
            &frame_geometry,
            &settings,
            &home,
            &toolpath,
            &rapid_path,
            true,
        );
    });

    let board = state.preview_scene.board_bounds.clone();
    let frame_geometry = state.preview_scene.frame_geometry.clone();
    let settings = state.settings.clone();
    let home = state.source_home.clone();
    let toolpath = state.preview_scene.toolpath.clone();
    let rapid_path = state.preview_scene.rapid_path.clone();
    let window = main_window.as_weak();
    main_window.on_apply_source_gcode(move || {
        let Some(window) = window.upgrade() else {
            return;
        };
        let content = window.get_source_gcode().to_string();
        apply_source(
            &window,
            &content,
            &board,
            &frame_geometry,
            &settings,
            &home,
            &toolpath,
            &rapid_path,
            false,
        );
    });

    let toolpath = state.preview_scene.toolpath.clone();
    let rapid_path = state.preview_scene.rapid_path.clone();
    let window = main_window.as_weak();
    main_window.on_apply_final_gcode(move || {
        let Some(window) = window.upgrade() else {
            return;
        };
        let content = window.get_final_gcode().to_string();
        let path = gcode::parse_gcode_toolpath(&content);
        if path.is_empty() {
            window.invoke_show_error("В финальном G-code не найдена траектория G1.".into());
            return;
        }
        *toolpath.borrow_mut() = path;
        *rapid_path.borrow_mut() = gcode::parse_gcode_rapid_path(&content);
        window.invoke_update_preview();
    });
}

#[allow(clippy::too_many_arguments)]
fn apply_source(
    window: &MainWindow,
    content: &str,
    board: &Rc<std::cell::RefCell<Option<frame::BoardBounds>>>,
    frame_geometry: &Rc<std::cell::RefCell<Option<frame::FrameGeometry>>>,
    settings: &Rc<std::cell::RefCell<crate::domain::settings::Settings>>,
    home: &Rc<std::cell::RefCell<Option<(f64, f64)>>>,
    toolpath: &Rc<std::cell::RefCell<Vec<(f64, f64)>>>,
    rapid_path: &Rc<std::cell::RefCell<Vec<(f64, f64)>>>,
    update_editor: bool,
) {
    let bounds = gcode::parse_gcode_bounds(content);
    if !bounds.is_valid() {
        window.invoke_show_error("В исходном G-code не найдена корректная траектория G1.".into());
        return;
    }
    let source_home = gcode::parse_gcode_home_position(content);
    let path = gcode::parse_gcode_toolpath(content);
    let rapid = gcode::parse_gcode_rapid_path(content);
    let current_settings = {
        let mut current = settings.borrow_mut();
        gcode::apply_source_cutting_parameters(content, &mut current);
        current.clone()
    };
    let Some(frame) = frame::FrameGeometry::calculate(&bounds, &current_settings) else {
        window.invoke_show_error("Не удалось вычислить рамку.".into());
        return;
    };
    window.set_board_width(format!("{:.3} mm", bounds.width()).into());
    window.set_board_height(format!("{:.3} mm", bounds.height()).into());
    window.set_x_min(format!("{:.3}", bounds.x_min).into());
    window.set_x_max(format!("{:.3}", bounds.x_max).into());
    window.set_y_min(format!("{:.3}", bounds.y_min).into());
    window.set_y_max(format!("{:.3}", bounds.y_max).into());
    window.set_frame_width(format!("{:.3} mm", frame.width()).into());
    window.set_frame_height(format!("{:.3} mm", frame.height()).into());
    window.set_file_loaded(true);
    *board.borrow_mut() = Some(bounds.clone());
    *frame_geometry.borrow_mut() = Some(frame.clone());
    *home.borrow_mut() = source_home;
    *toolpath.borrow_mut() = path;
    *rapid_path.borrow_mut() = rapid;
    if update_editor {
        let (text, truncated) = source_gcode_for_editor(content);
        window.set_source_gcode(text.into());
        window.set_source_gcode_truncated(truncated);
    }
    window.set_final_gcode(
        export_gcode::generate_frame_gcode(&bounds, &frame, &current_settings, source_home).into(),
    );
    window.invoke_update_preview();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn large_source_is_truncated_at_a_character_boundary() {
        let content = format!("{}\nG1 X1", "Ж".repeat(MAX_SOURCE_EDITOR_BYTES));
        let (editor_text, truncated) = source_gcode_for_editor(&content);

        assert!(truncated);
        assert!(editor_text.is_char_boundary(editor_text.len()));
    }
}
