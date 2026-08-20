use slint::{ComponentHandle, SharedPixelBuffer};
use std::cell::RefCell;
use std::rc::Rc;

use crate::viewport::Viewport;
use crate::{exporter, frame, preview, preview_renderer, settings, MainWindow, UiSettings};

// Slint's text editor eagerly lays out its entire value.  Keep the editor
// responsive and avoid renderer crashes when an otherwise valid TAP is huge.
const MAX_SOURCE_EDITOR_BYTES: usize = 10 * 1024;

fn source_gcode_for_editor(content: &str) -> (String, bool) {
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

pub fn run(main_window: MainWindow) -> Result<(), Box<dyn std::error::Error>> {
    // Load settings
    let settings = match settings::Settings::load() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Warning: Could not load settings: {}", e);
            settings::Settings::default()
        }
    };

    // Initialize UI with settings
    let ui_settings = main_window.global::<UiSettings>();
    ui_settings.set_offset_x(settings.offset_x as f32);
    ui_settings.set_offset_y(settings.offset_y as f32);
    ui_settings.set_tab_width(settings.tab_width as f32);
    ui_settings.set_minimum_tabs(settings.minimum_tabs as f32);
    ui_settings.set_maximum_tab_gap(settings.maximum_tab_gap as f32);
    ui_settings.set_score_tabs(settings.score_tabs);
    ui_settings.set_local_offset_enabled(settings.local_offset_enabled);
    ui_settings.set_local_offset_x(settings.local_offset_x as f32);
    ui_settings.set_local_offset_y(settings.local_offset_y as f32);
    ui_settings.set_material_width(settings.material_width as f32);
    ui_settings.set_material_height(settings.material_height as f32);

    // State management
    let board_bounds = Rc::new(RefCell::new(None));
    let frame_geometry = Rc::new(RefCell::new(None));
    let source_home = Rc::new(RefCell::new(None));
    let toolpath = Rc::new(RefCell::new(Vec::new()));
    let rapid_path = Rc::new(RefCell::new(Vec::new()));
    let source_file_stem = Rc::new(RefCell::new(None::<String>));
    let viewport = Rc::new(RefCell::new(Viewport::default()));
    let pointer_position = Rc::new(RefCell::new(None::<(f64, f64)>));
    let current_settings = Rc::new(RefCell::new(settings));

    // Keep the Rust state and calculated frame in step with edits made in the UI.
    let weak_board = Rc::downgrade(&board_bounds);
    let weak_frame = Rc::downgrade(&frame_geometry);
    let weak_settings = Rc::downgrade(&current_settings);
    let weak_home = Rc::downgrade(&source_home);
    let window_weak = main_window.as_weak();
    main_window.on_sync_settings(move || {
        let Some(window) = window_weak.upgrade() else {
            return;
        };
        let Some(settings_rc) = weak_settings.upgrade() else {
            return;
        };

        let ui_settings = window.global::<UiSettings>();
        let source_settings = settings_rc.borrow().clone();
        let new_settings = settings::Settings {
            offset_x: ui_settings.get_offset_x() as f64,
            offset_y: ui_settings.get_offset_y() as f64,
            tab_width: ui_settings.get_tab_width() as f64,
            minimum_tabs: (ui_settings.get_minimum_tabs() as f64).round().max(3.0) as usize,
            maximum_tab_gap: ui_settings.get_maximum_tab_gap() as f64,
            score_tabs: ui_settings.get_score_tabs(),
            local_offset_enabled: ui_settings.get_local_offset_enabled(),
            local_offset_x: ui_settings.get_local_offset_x() as f64,
            local_offset_y: ui_settings.get_local_offset_y() as f64,
            material_width: ui_settings.get_material_width() as f64,
            material_height: ui_settings.get_material_height() as f64,
            ..source_settings
        };

        *settings_rc.borrow_mut() = new_settings.clone();

        let Some(board_rc) = weak_board.upgrade() else {
            return;
        };
        let Some(frame_rc) = weak_frame.upgrade() else {
            return;
        };
        let Some(home_rc) = weak_home.upgrade() else {
            return;
        };
        let bounds = board_rc.borrow().clone();
        if let Some(bounds) = bounds {
            if let Some(frame) = frame::FrameGeometry::calculate(&bounds, &new_settings) {
                window.set_frame_width(format!("{:.3} mm", frame.width()).into());
                window.set_frame_height(format!("{:.3} mm", frame.height()).into());
                // Keep the editor in sync with export: the displayed program
                // is already expressed in global machine coordinates.
                window.set_final_gcode(
                    exporter::generate_frame_gcode(
                        &bounds,
                        &frame,
                        &new_settings,
                        *home_rc.borrow(),
                    )
                    .into(),
                );
                *frame_rc.borrow_mut() = Some(frame);
            }
        }
    });

    // Open TAP file handler
    let weak_board = Rc::downgrade(&board_bounds);
    let weak_frame = Rc::downgrade(&frame_geometry);
    let weak_settings = Rc::downgrade(&current_settings);
    let weak_home = Rc::downgrade(&source_home);
    let weak_toolpath = Rc::downgrade(&toolpath);
    let weak_rapid_path = Rc::downgrade(&rapid_path);
    let weak_source_file_stem = Rc::downgrade(&source_file_stem);
    let window_weak = main_window.as_weak();

    main_window.on_open_tap_file(move || {
        let Some(window) = window_weak.upgrade() else {
            return;
        };
        let Some(board_rc) = weak_board.upgrade() else {
            return;
        };
        let Some(frame_rc) = weak_frame.upgrade() else {
            return;
        };
        let Some(settings_rc) = weak_settings.upgrade() else {
            return;
        };
        let Some(home_rc) = weak_home.upgrade() else {
            return;
        };
        let Some(path_rc) = weak_toolpath.upgrade() else {
            return;
        };
        let Some(rapid_rc) = weak_rapid_path.upgrade() else {
            return;
        };
        let Some(source_file_stem_rc) = weak_source_file_stem.upgrade() else {
            return;
        };

        let Some(file_path) = rfd::FileDialog::new()
            .add_filter("TAP Files", &["tap", "nc", "gcode", "ngc"])
            .pick_file()
        else {
            return;
        };

        let content = match std::fs::read_to_string(&file_path) {
            Ok(c) => c,
            Err(e) => {
                window.invoke_show_error(format!("Failed to read file: {}", e).into());
                return;
            }
        };

        *source_file_stem_rc.borrow_mut() = file_path
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .filter(|stem| !stem.is_empty());

        let bounds = frame::parse_gcode_bounds(&content);
        let home = frame::parse_gcode_home_position(&content);
        let path = frame::parse_gcode_toolpath(&content);
        let rapid = frame::parse_gcode_rapid_path(&content);

        frame::apply_source_cutting_parameters(&content, &mut settings_rc.borrow_mut());

        if !bounds.is_valid() {
            window.invoke_show_error("No valid G-code coordinates found in file.".into());
            return;
        }

        let frame = match frame::FrameGeometry::calculate(&bounds, &settings_rc.borrow()) {
            Some(f) => f,
            None => {
                window.invoke_show_error("Could not calculate frame geometry.".into());
                return;
            }
        };

        // Update UI info
        let board_width = bounds.width();
        let board_height = bounds.height();
        let frame_width = frame.width();
        let frame_height = frame.height();

        window.set_board_width(format!("{:.3} mm", board_width).into());
        window.set_board_height(format!("{:.3} mm", board_height).into());
        window.set_x_min(format!("{:.3}", bounds.x_min).into());
        window.set_x_max(format!("{:.3}", bounds.x_max).into());
        window.set_y_min(format!("{:.3}", bounds.y_min).into());
        window.set_y_max(format!("{:.3}", bounds.y_max).into());
        window.set_frame_width(format!("{:.3} mm", frame_width).into());
        window.set_frame_height(format!("{:.3} mm", frame_height).into());
        window.set_file_loaded(true);

        *board_rc.borrow_mut() = Some(bounds.clone());
        *frame_rc.borrow_mut() = Some(frame.clone());
        *home_rc.borrow_mut() = home;
        *path_rc.borrow_mut() = path;
        *rapid_rc.borrow_mut() = rapid;
        let (source_gcode, source_gcode_truncated) = source_gcode_for_editor(&content);
        window.set_source_gcode(source_gcode.into());
        window.set_source_gcode_truncated(source_gcode_truncated);
        window.set_final_gcode(
            exporter::generate_frame_gcode(&bounds, &frame, &settings_rc.borrow(), home).into(),
        );

        // Trigger preview update
        window.invoke_update_preview();
    });

    // Applying the source editor replaces the source program and recomputes
    // every derived value: bounds, parking position, cutting parameters and frame.
    let weak_board = Rc::downgrade(&board_bounds);
    let weak_frame = Rc::downgrade(&frame_geometry);
    let weak_settings = Rc::downgrade(&current_settings);
    let weak_home = Rc::downgrade(&source_home);
    let weak_toolpath = Rc::downgrade(&toolpath);
    let weak_rapid_path = Rc::downgrade(&rapid_path);
    let window_weak = main_window.as_weak();
    main_window.on_apply_source_gcode(move || {
        let (
            Some(window),
            Some(board_rc),
            Some(frame_rc),
            Some(settings_rc),
            Some(home_rc),
            Some(path_rc),
            Some(rapid_rc),
        ) = (
            window_weak.upgrade(),
            weak_board.upgrade(),
            weak_frame.upgrade(),
            weak_settings.upgrade(),
            weak_home.upgrade(),
            weak_toolpath.upgrade(),
            weak_rapid_path.upgrade(),
        )
        else {
            return;
        };
        let content = window.get_source_gcode().to_string();
        let bounds = frame::parse_gcode_bounds(&content);
        if !bounds.is_valid() {
            window
                .invoke_show_error("В исходном G-code не найдена корректная траектория G1.".into());
            return;
        }
        let home = frame::parse_gcode_home_position(&content);
        let path = frame::parse_gcode_toolpath(&content);
        let rapid = frame::parse_gcode_rapid_path(&content);
        let settings = {
            let mut settings = settings_rc.borrow_mut();
            frame::apply_source_cutting_parameters(&content, &mut settings);
            settings.clone()
        };
        let Some(generated_frame) = frame::FrameGeometry::calculate(&bounds, &settings) else {
            window.invoke_show_error("Не удалось вычислить рамку.".into());
            return;
        };
        window.set_board_width(format!("{:.3} mm", bounds.width()).into());
        window.set_board_height(format!("{:.3} mm", bounds.height()).into());
        window.set_x_min(format!("{:.3}", bounds.x_min).into());
        window.set_x_max(format!("{:.3}", bounds.x_max).into());
        window.set_y_min(format!("{:.3}", bounds.y_min).into());
        window.set_y_max(format!("{:.3}", bounds.y_max).into());
        window.set_frame_width(format!("{:.3} mm", generated_frame.width()).into());
        window.set_frame_height(format!("{:.3} mm", generated_frame.height()).into());
        *board_rc.borrow_mut() = Some(bounds.clone());
        *frame_rc.borrow_mut() = Some(generated_frame.clone());
        *home_rc.borrow_mut() = home;
        *path_rc.borrow_mut() = path;
        *rapid_rc.borrow_mut() = rapid;
        window.set_final_gcode(
            exporter::generate_frame_gcode(&bounds, &generated_frame, &settings, home).into(),
        );
        window.invoke_update_preview();
    });

    // Applying final text is deliberately preview-only: source geometry and
    // generation settings stay untouched.
    let weak_toolpath = Rc::downgrade(&toolpath);
    let weak_rapid_path = Rc::downgrade(&rapid_path);
    let window_weak = main_window.as_weak();
    main_window.on_apply_final_gcode(move || {
        let (Some(window), Some(path_rc), Some(rapid_rc)) = (
            window_weak.upgrade(),
            weak_toolpath.upgrade(),
            weak_rapid_path.upgrade(),
        ) else {
            return;
        };
        let content = window.get_final_gcode().to_string();
        let path = frame::parse_gcode_toolpath(&content);
        if path.is_empty() {
            window.invoke_show_error("В финальном G-code не найдена траектория G1.".into());
            return;
        }
        *path_rc.borrow_mut() = path;
        *rapid_rc.borrow_mut() = frame::parse_gcode_rapid_path(&content);
        window.invoke_update_preview();
    });

    let viewport_state = viewport.clone();
    let weak_board = Rc::downgrade(&board_bounds);
    let weak_frame = Rc::downgrade(&frame_geometry);
    let weak_settings = Rc::downgrade(&current_settings);
    let window_weak = main_window.as_weak();
    main_window.on_zoom_preview(move |direction, mouse_x, mouse_y, width, height| {
        if width <= 0.0 || height <= 0.0 {
            return;
        }
        let (Some(window), Some(board_rc), Some(frame_rc), Some(settings_rc)) = (
            window_weak.upgrade(),
            weak_board.upgrade(),
            weak_frame.upgrade(),
            weak_settings.upgrade(),
        ) else {
            return;
        };
        let (Some(bounds), Some(frame)) = (
            board_rc.borrow().as_ref().cloned(),
            frame_rc.borrow().as_ref().cloned(),
        ) else {
            return;
        };
        let settings = settings_rc.borrow().clone();
        let (shift_x, shift_y) = settings.local_offset();
        let expanded = frame::FrameGeometry::expanded(&bounds, &settings);
        let preview_left = expanded
            .as_ref()
            .map_or(frame.left, |value| frame.left.min(value.left));
        let preview_right = expanded
            .as_ref()
            .map_or(frame.right, |value| frame.right.max(value.right));
        let preview_bottom = expanded
            .as_ref()
            .map_or(frame.bottom, |value| frame.bottom.min(value.bottom));
        let preview_top = expanded
            .as_ref()
            .map_or(frame.top, |value| frame.top.max(value.top));
        let data = preview::PreviewData::from_bounds_with_material(
            bounds.x_min + shift_x,
            bounds.x_max + shift_x,
            bounds.y_min + shift_y,
            bounds.y_max + shift_y,
            preview_left + shift_x,
            preview_right + shift_x,
            preview_bottom + shift_y,
            preview_top + shift_y,
            settings.material_width,
            settings.material_height,
        );
        let (min_x, max_x, min_y, max_y) = data.world_bounds();
        let content_width = max_x - min_x;
        let content_height = max_y - min_y;
        let base_scale = data.calculate_scale(width, height);
        let mut camera = *viewport_state.borrow();
        let old_scale = base_scale * camera.zoom;
        if old_scale <= 0.0 {
            return;
        }
        let world_x = min_x
            + (mouse_x as f64 - (width as f64 - content_width * old_scale) / 2.0 - camera.pan_x)
                / old_scale;
        let world_y = min_y
            + ((height as f64 + content_height * old_scale) / 2.0 + camera.pan_y - mouse_y as f64)
                / old_scale;
        camera.zoom_by(direction);
        let new_scale = base_scale * camera.zoom;
        camera.pan_x = mouse_x as f64
            - (width as f64 - content_width * new_scale) / 2.0
            - (world_x - min_x) * new_scale;
        camera.pan_y = mouse_y as f64 - (height as f64 + content_height * new_scale) / 2.0
            + (world_y - min_y) * new_scale;
        *viewport_state.borrow_mut() = camera;
        window.invoke_update_preview();
    });
    let viewport_state = viewport.clone();
    let weak_board = Rc::downgrade(&board_bounds);
    let weak_frame = Rc::downgrade(&frame_geometry);
    let weak_settings = Rc::downgrade(&current_settings);
    let window_weak = main_window.as_weak();
    main_window.on_fit_preview(move || {
        let (Some(window), Some(board_rc), Some(frame_rc), Some(settings_rc)) = (
            window_weak.upgrade(),
            weak_board.upgrade(),
            weak_frame.upgrade(),
            weak_settings.upgrade(),
        ) else {
            return;
        };
        let (Some(bounds), Some(frame)) = (
            board_rc.borrow().as_ref().cloned(),
            frame_rc.borrow().as_ref().cloned(),
        ) else {
            return;
        };
        let settings = settings_rc.borrow().clone();
        let (shift_x, shift_y) = settings.local_offset();
        let preview = preview::PreviewData::from_bounds_with_material(
            bounds.x_min + shift_x,
            bounds.x_max + shift_x,
            bounds.y_min + shift_y,
            bounds.y_max + shift_y,
            frame.left + shift_x,
            frame.right + shift_x,
            frame.bottom + shift_y,
            frame.top + shift_y,
            settings.material_width,
            settings.material_height,
        );
        let width = window.get_preview_width().max(1.0) as f64;
        let height = window.get_preview_height().max(1.0) as f64;
        let desired_scale = ((width - 40.0) / frame.width().max(1.0))
            .min((height - 40.0) / frame.height().max(1.0));
        let base_scale = preview
            .calculate_scale(width as f32, height as f32)
            .max(f64::MIN_POSITIVE);
        let (min_x, max_x, min_y, max_y) = preview.world_bounds();
        let center_x = (frame.left + frame.right) / 2.0 + shift_x;
        let center_y = (frame.bottom + frame.top) / 2.0 + shift_y;
        let content_width = max_x - min_x;
        let content_height = max_y - min_y;
        let screen_x =
            (width - content_width * desired_scale) / 2.0 + (center_x - min_x) * desired_scale;
        let screen_y =
            (height + content_height * desired_scale) / 2.0 - (center_y - min_y) * desired_scale;
        *viewport_state.borrow_mut() = Viewport {
            zoom: (desired_scale / base_scale).clamp(0.1, 20.0),
            pan_x: width / 2.0 - screen_x,
            pan_y: height / 2.0 - screen_y,
        };
        window.invoke_update_preview();
    });
    let pointer_state = pointer_position.clone();
    main_window.on_begin_pan(move |x, y| {
        *pointer_state.borrow_mut() = Some((x as f64, y as f64));
    });
    let pointer_state = pointer_position.clone();
    let viewport_state = viewport.clone();
    let window_weak = main_window.as_weak();
    main_window.on_pan_preview(move |x, y| {
        let current = (x as f64, y as f64);
        if let Some(previous) = *pointer_state.borrow() {
            viewport_state
                .borrow_mut()
                .pan_by(current.0 - previous.0, current.1 - previous.1);
        }
        *pointer_state.borrow_mut() = Some(current);
        if let Some(window) = window_weak.upgrade() {
            window.invoke_update_preview();
        }
    });
    let weak_board = Rc::downgrade(&board_bounds);
    let weak_frame = Rc::downgrade(&frame_geometry);
    let weak_viewport = Rc::downgrade(&viewport);
    let weak_settings = Rc::downgrade(&current_settings);
    let window_weak = main_window.as_weak();
    main_window.on_cursor_moved(move |x, y, width, height| {
        if width <= 0.0 || height <= 0.0 {
            return;
        }
        let (Some(window), Some(board), Some(frame), Some(viewport), Some(settings_rc)) = (
            window_weak.upgrade(),
            weak_board.upgrade(),
            weak_frame.upgrade(),
            weak_viewport.upgrade(),
            weak_settings.upgrade(),
        ) else {
            return;
        };
        let (Some(bounds), Some(frame)) = (
            board.borrow().as_ref().cloned(),
            frame.borrow().as_ref().cloned(),
        ) else {
            return;
        };
        let settings = settings_rc.borrow().clone();
        let expanded = frame::FrameGeometry::expanded(&bounds, &settings);
        let preview_left = expanded
            .as_ref()
            .map_or(frame.left, |value| frame.left.min(value.left));
        let preview_right = expanded
            .as_ref()
            .map_or(frame.right, |value| frame.right.max(value.right));
        let preview_bottom = expanded
            .as_ref()
            .map_or(frame.bottom, |value| frame.bottom.min(value.bottom));
        let preview_top = expanded
            .as_ref()
            .map_or(frame.top, |value| frame.top.max(value.top));
        let (shift_x, shift_y) = settings.local_offset();
        let data = preview::PreviewData::from_bounds_with_material(
            bounds.x_min + shift_x,
            bounds.x_max + shift_x,
            bounds.y_min + shift_y,
            bounds.y_max + shift_y,
            preview_left + shift_x,
            preview_right + shift_x,
            preview_bottom + shift_y,
            preview_top + shift_y,
            settings.material_width,
            settings.material_height,
        );
        let camera = *viewport.borrow();
        let scale = data.calculate_scale(width, height) * camera.zoom;
        if scale <= 0.0 {
            return;
        }
        let (min_x, max_x, min_y, max_y) = data.world_bounds();
        let content_width = max_x - min_x;
        let content_height = max_y - min_y;
        let world_x = min_x
            + (x as f64 - (width as f64 - content_width * scale) / 2.0 - camera.pan_x) / scale;
        let world_y = min_y
            + ((height as f64 + content_height * scale) / 2.0 + camera.pan_y - y as f64) / scale;
        let text = if settings.local_offset_enabled {
            format!(
                "Локальные: X: {:.3}   Y: {:.3}\nГлобальные: X: {world_x:.3}   Y: {world_y:.3}",
                world_x - settings.local_offset_x,
                world_y - settings.local_offset_y,
            )
        } else {
            format!("Глобальные: X: {world_x:.3}   Y: {world_y:.3}")
        };
        window.set_cursor_coordinates(text.into());
    });

    // Save settings handler
    let board_bounds_clone = board_bounds.clone();
    let frame_geometry_clone = frame_geometry.clone();
    let weak_settings = Rc::downgrade(&current_settings);
    let window_weak = main_window.as_weak();

    main_window.on_save_settings(move || {
        let Some(window) = window_weak.upgrade() else {
            return;
        };
        let Some(settings_rc) = weak_settings.upgrade() else {
            return;
        };

        let ui_settings = window.global::<UiSettings>();

        let source_settings = settings_rc.borrow().clone();
        let new_settings = settings::Settings {
            offset_x: ui_settings.get_offset_x() as f64,
            offset_y: ui_settings.get_offset_y() as f64,
            tab_width: ui_settings.get_tab_width() as f64,
            minimum_tabs: (ui_settings.get_minimum_tabs() as f64).round().max(3.0) as usize,
            maximum_tab_gap: ui_settings.get_maximum_tab_gap() as f64,
            score_tabs: ui_settings.get_score_tabs(),
            local_offset_enabled: ui_settings.get_local_offset_enabled(),
            local_offset_x: ui_settings.get_local_offset_x() as f64,
            local_offset_y: ui_settings.get_local_offset_y() as f64,
            material_width: ui_settings.get_material_width() as f64,
            material_height: ui_settings.get_material_height() as f64,
            ..source_settings
        };

        if let Err(e) = new_settings.save() {
            window.invoke_show_error(format!("Failed to save settings: {}", e).into());
            return;
        }

        *settings_rc.borrow_mut() = new_settings.clone();

        // Recalculate frame if we have board data
        let bounds = board_bounds_clone.borrow().clone();
        if let Some(bounds) = bounds {
            if let Some(frame) = frame::FrameGeometry::calculate(&bounds, &new_settings) {
                *frame_geometry_clone.borrow_mut() = Some(frame);
            }
        }
        window.invoke_update_preview();
    });

    // Export TAP handler
    let weak_board = Rc::downgrade(&board_bounds);
    let weak_frame = Rc::downgrade(&frame_geometry);
    let weak_toolpath = Rc::downgrade(&toolpath);
    let weak_rapid_path = Rc::downgrade(&rapid_path);
    let weak_viewport = Rc::downgrade(&viewport);
    let weak_settings = Rc::downgrade(&current_settings);
    let weak_home = Rc::downgrade(&source_home);
    let weak_source_file_stem = Rc::downgrade(&source_file_stem);
    let window_weak = main_window.as_weak();

    main_window.on_export_tap(move || {
        let Some(window) = window_weak.upgrade() else { return; };
        let Some(board_rc) = weak_board.upgrade() else { return; };
        let Some(frame_rc) = weak_frame.upgrade() else { return; };
        let Some(settings_rc) = weak_settings.upgrade() else { return; };
        let Some(home_rc) = weak_home.upgrade() else { return; };
        let Some(source_file_stem_rc) = weak_source_file_stem.upgrade() else { return; };

        let board_borrow = board_rc.borrow();
        let Some(bounds) = board_borrow.as_ref() else {
            window.invoke_show_error("No board data loaded.".into());
            return;
        };

        let frame_borrow = frame_rc.borrow();
        let Some(frame) = frame_borrow.as_ref() else {
            window.invoke_show_error("No frame geometry calculated.".into());
            return;
        };

        let settings = settings_rc.borrow().clone();
        let (shift_x, shift_y) = settings.local_offset();
        let exceeds_material = frame.left + shift_x < -settings.material_width.max(0.0)
            || frame.bottom + shift_y < 0.0
            || frame.right + shift_x > 0.0
            || frame.top + shift_y > settings.material_height.max(0.0);
        if exceeds_material {
            let accepted = rfd::MessageDialog::new()
                .set_level(rfd::MessageLevel::Warning)
                .set_title("Рамка выходит за текстолит")
                .set_description(format!(
                    "Экспортируемая рамка выходит за пределы текстолита {:.1} × {:.1} мм. Продолжить экспорт?",
                    settings.material_width, settings.material_height
                ))
                .set_buttons(rfd::MessageButtons::OkCancel)
                .show();
            if !matches!(accepted, rfd::MessageDialogResult::Ok) { return; }
        }

        let suggested_file_name = source_file_stem_rc
            .borrow()
            .as_deref()
            .map(|stem| format!("{stem}_frame.tap"))
            .unwrap_or_else(|| "frame.tap".to_owned());
        let Some(file_path) = rfd::FileDialog::new()
            .set_file_name(&suggested_file_name)
            .save_file()
        else {
            return;
        };

        let gcode = exporter::generate_frame_gcode(
            bounds,
            frame,
            &settings,
            *home_rc.borrow(),
        );

        if let Err(e) = exporter::save_gcode(&gcode, &file_path) {
            window.invoke_show_error(format!("Failed to export G-code: {}", e).into());
            return;
        }
    });

    // Preview render handler - creates a simple line representation
    let weak_board = Rc::downgrade(&board_bounds);
    let weak_frame = Rc::downgrade(&frame_geometry);
    let weak_settings = Rc::downgrade(&current_settings);
    let window_weak = main_window.as_weak();

    main_window.on_render_preview(move |width, height| {
        // Camera panning is expressed in canvas pixels, so the software buffer
        // must use the exact canvas size. Downscaling it here causes dragging
        // and the displayed image to move by different amounts after resize.
        let width = width.max(1.0).round() as u32;
        let height = height.max(1.0).round() as u32;

        if width == 0 || height == 0 {
            return slint::Image::from_rgb8(SharedPixelBuffer::new(1, 1));
        }

        let Some(_window) = window_weak.upgrade() else {
            return slint::Image::from_rgb8(SharedPixelBuffer::new(1, 1));
        };

        let Some(board_rc) = weak_board.upgrade() else {
            return slint::Image::from_rgb8(SharedPixelBuffer::new(width, height));
        };
        let Some(frame_rc) = weak_frame.upgrade() else {
            return slint::Image::from_rgb8(SharedPixelBuffer::new(width, height));
        };
        let Some(settings_rc) = weak_settings.upgrade() else {
            return slint::Image::from_rgb8(SharedPixelBuffer::new(width, height));
        };
        let Some(path_rc) = weak_toolpath.upgrade() else {
            return slint::Image::from_rgb8(SharedPixelBuffer::new(width, height));
        };
        let Some(rapid_rc) = weak_rapid_path.upgrade() else {
            return slint::Image::from_rgb8(SharedPixelBuffer::new(width, height));
        };
        let Some(viewport_rc) = weak_viewport.upgrade() else {
            return slint::Image::from_rgb8(SharedPixelBuffer::new(width, height));
        };

        let mut buffer = SharedPixelBuffer::<slint::Rgb8Pixel>::new(width, height);

        // Dark editor-like background and a neutral grid.
        for pixel in buffer.make_mut_slice() {
            *pixel = slint::Rgb8Pixel::new(14, 17, 22);
        }
        let board_borrow = board_rc.borrow();
        let Some(bounds) = board_borrow.as_ref() else {
            return slint::Image::from_rgb8(buffer);
        };

        let frame_borrow = frame_rc.borrow();
        let Some(frame) = frame_borrow.as_ref() else {
            return slint::Image::from_rgb8(buffer);
        };

        let settings = settings_rc.borrow().clone();
        let (shift_x, shift_y) = settings.local_offset();
        let expanded = frame::FrameGeometry::expanded(&bounds, &settings);
        let preview_left = expanded
            .as_ref()
            .map_or(frame.left, |value| frame.left.min(value.left));
        let preview_right = expanded
            .as_ref()
            .map_or(frame.right, |value| frame.right.max(value.right));
        let preview_bottom = expanded
            .as_ref()
            .map_or(frame.bottom, |value| frame.bottom.min(value.bottom));
        let preview_top = expanded
            .as_ref()
            .map_or(frame.top, |value| frame.top.max(value.top));
        let preview_data = preview::PreviewData::from_bounds_with_material(
            bounds.x_min + shift_x,
            bounds.x_max + shift_x,
            bounds.y_min + shift_y,
            bounds.y_max + shift_y,
            preview_left + shift_x,
            preview_right + shift_x,
            preview_bottom + shift_y,
            preview_top + shift_y,
            settings.material_width,
            settings.material_height,
        );

        let viewport = *viewport_rc.borrow();
        let scale = preview_data.calculate_scale(width as f32, height as f32) * viewport.zoom;
        let pan = (viewport.pan_x, viewport.pan_y);
        preview_renderer::draw_grid(&mut buffer, &preview_data, scale, pan);
        preview_renderer::axes(
            &mut buffer,
            &preview_data,
            scale,
            width as f32,
            height as f32,
            pan,
        );
        if settings.local_offset_enabled {
            preview_renderer::local_axes(
                &mut buffer,
                &preview_data,
                scale,
                width as f32,
                height as f32,
                pan,
                shift_x,
                shift_y,
            );
        }

        // Draw board (blue) - thicker lines

        // Draw frame (red) - thicker lines
        let path = path_rc.borrow();
        let shifted_path: Vec<_> = path
            .iter()
            .map(|&(x, y)| (x + shift_x, y + shift_y))
            .collect();
        preview_renderer::polyline(
            &mut buffer,
            &shifted_path,
            &preview_data,
            scale,
            width as f32,
            height as f32,
            (0, 210, 255),
            2,
            pan,
        );
        let material = [
            (-settings.material_width, 0.0),
            (0.0, 0.0),
            (0.0, settings.material_height),
            (-settings.material_width, settings.material_height),
            (-settings.material_width, 0.0),
        ];
        preview_renderer::rectangle(
            &mut buffer,
            &material,
            &preview_data,
            scale,
            width as f32,
            height as f32,
            (120, 170, 120),
            pan,
        );
        if let Some(expanded) = expanded {
            let corners = [
                (expanded.left + shift_x, expanded.bottom + shift_y),
                (expanded.right + shift_x, expanded.bottom + shift_y),
                (expanded.right + shift_x, expanded.top + shift_y),
                (expanded.left + shift_x, expanded.top + shift_y),
                (expanded.left + shift_x, expanded.bottom + shift_y),
            ];
            preview_renderer::dotted_rectangle(
                &mut buffer,
                &corners,
                &preview_data,
                scale,
                width as f32,
                height as f32,
                pan,
            );
        }
        let anchored_corners = [
            (frame.left + shift_x, frame.bottom + shift_y),
            (frame.right + shift_x, frame.bottom + shift_y),
            (frame.right + shift_x, frame.top + shift_y),
            (frame.left + shift_x, frame.top + shift_y),
            (frame.left + shift_x, frame.bottom + shift_y),
        ];
        preview_renderer::rectangle(
            &mut buffer,
            &anchored_corners,
            &preview_data,
            scale,
            width as f32,
            height as f32,
            (255, 70, 100),
            pan,
        );
        let corner_radius = (settings.tool_diameter / 2.0)
            .min(1.0)
            .min(frame.width() / 4.0)
            .min(frame.height() / 4.0)
            .max(0.0);
        for (left, right) in frame::top_tab_intervals(frame, corner_radius, &settings) {
            preview_renderer::polyline(
                &mut buffer,
                &[
                    (left + shift_x, frame.top + shift_y),
                    (right + shift_x, frame.top + shift_y),
                ],
                &preview_data,
                scale,
                width as f32,
                height as f32,
                (255, 220, 70),
                4,
                pan,
            );
        }
        let rapid = rapid_rc.borrow();
        let shifted_rapid: Vec<_> = rapid
            .iter()
            .map(|&(x, y)| (x + shift_x, y + shift_y))
            .collect();
        preview_renderer::polyline(
            &mut buffer,
            &shifted_rapid,
            &preview_data,
            scale,
            width as f32,
            height as f32,
            (255, 190, 0),
            1,
            pan,
        );

        slint::Image::from_rgb8(buffer)
    });

    main_window.run()?;
    Ok(())
}
