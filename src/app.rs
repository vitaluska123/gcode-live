use slint::{ComponentHandle, SharedPixelBuffer};
use std::rc::Rc;
use std::cell::RefCell;

use crate::{exporter, frame, preview, preview_renderer, settings, MainWindow, UiSettings};
use crate::viewport::Viewport;

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
    ui_settings.set_clamp_zone(settings.clamp_zone as f32);
    ui_settings.set_safe_zone(settings.safe_zone as f32);
    ui_settings.set_tool_diameter(settings.tool_diameter as f32);
    ui_settings.set_cut_depth(settings.cut_depth as f32);
    ui_settings.set_step_depth(settings.step_depth as f32);
    ui_settings.set_feed_rate(settings.feed_rate as f32);
    ui_settings.set_spindle_speed(settings.spindle_speed as f32);

    // State management
    let board_bounds = Rc::new(RefCell::new(None));
    let frame_geometry = Rc::new(RefCell::new(None));
    let source_home = Rc::new(RefCell::new(None));
    let toolpath = Rc::new(RefCell::new(Vec::new()));
    let rapid_path = Rc::new(RefCell::new(Vec::new()));
    let viewport = Rc::new(RefCell::new(Viewport::default()));
    let pointer_position = Rc::new(RefCell::new(None::<(f64, f64)>));
    let current_settings = Rc::new(RefCell::new(settings));

    // Keep the Rust state and calculated frame in step with edits made in the UI.
    let weak_board = Rc::downgrade(&board_bounds);
    let weak_frame = Rc::downgrade(&frame_geometry);
    let weak_settings = Rc::downgrade(&current_settings);
    let window_weak = main_window.as_weak();
    main_window.on_sync_settings(move || {
        let Some(window) = window_weak.upgrade() else { return; };
        let Some(settings_rc) = weak_settings.upgrade() else { return; };

        let ui_settings = window.global::<UiSettings>();
        let new_settings = settings::Settings {
            offset_x: ui_settings.get_offset_x() as f64,
            offset_y: ui_settings.get_offset_y() as f64,
            clamp_zone: ui_settings.get_clamp_zone() as f64,
            safe_zone: ui_settings.get_safe_zone() as f64,
            tool_diameter: ui_settings.get_tool_diameter() as f64,
            cut_depth: ui_settings.get_cut_depth() as f64,
            step_depth: ui_settings.get_step_depth() as f64,
            feed_rate: ui_settings.get_feed_rate() as f64,
            spindle_speed: ui_settings.get_spindle_speed() as f64,
        };

        *settings_rc.borrow_mut() = new_settings.clone();

        let Some(board_rc) = weak_board.upgrade() else { return; };
        let Some(frame_rc) = weak_frame.upgrade() else { return; };
        let bounds = board_rc.borrow();
        if let Some(bounds) = bounds.as_ref() {
            if let Some(frame) = frame::FrameGeometry::calculate(bounds, &new_settings) {
                window.set_frame_width(format!("{:.3} mm", frame.width()).into());
                window.set_frame_height(format!("{:.3} mm", frame.height()).into());
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
    let window_weak = main_window.as_weak();

    main_window.on_open_tap_file(move || {
        let Some(window) = window_weak.upgrade() else { return; };
        let Some(board_rc) = weak_board.upgrade() else { return; };
        let Some(frame_rc) = weak_frame.upgrade() else { return; };
        let Some(settings_rc) = weak_settings.upgrade() else { return; };
        let Some(home_rc) = weak_home.upgrade() else { return; };
        let Some(path_rc) = weak_toolpath.upgrade() else { return; };
        let Some(rapid_rc) = weak_rapid_path.upgrade() else { return; };

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

        let bounds = frame::parse_gcode_bounds(&content);
        let home = frame::parse_gcode_home_position(&content);
        let path = frame::parse_gcode_toolpath(&content);
        let rapid = frame::parse_gcode_rapid_path(&content);

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

        *board_rc.borrow_mut() = Some(bounds);
        *frame_rc.borrow_mut() = Some(frame);
        *home_rc.borrow_mut() = home;
        *path_rc.borrow_mut() = path;
        *rapid_rc.borrow_mut() = rapid;

        // Trigger preview update
        window.invoke_update_preview();
    });

    let viewport_state = viewport.clone();
    let window_weak = main_window.as_weak();
    main_window.on_zoom_preview(move |direction| {
        {
            viewport_state.borrow_mut().zoom_by(direction);
        }
        if let Some(window) = window_weak.upgrade() { window.invoke_update_preview(); }
    });
    let viewport_state = viewport.clone();
    let window_weak = main_window.as_weak();
    main_window.on_fit_preview(move || {
        viewport_state.borrow_mut().reset();
        if let Some(window) = window_weak.upgrade() { window.invoke_update_preview(); }
    });
    let pointer_state = pointer_position.clone();
    main_window.on_begin_pan(move |x, y| { *pointer_state.borrow_mut() = Some((x as f64, y as f64)); });
    let pointer_state = pointer_position.clone();
    let viewport_state = viewport.clone();
    let window_weak = main_window.as_weak();
    main_window.on_pan_preview(move |x, y| {
        let current = (x as f64, y as f64);
        if let Some(previous) = *pointer_state.borrow() {
            viewport_state.borrow_mut().pan_by(current.0 - previous.0, current.1 - previous.1);
        }
        *pointer_state.borrow_mut() = Some(current);
        if let Some(window) = window_weak.upgrade() { window.invoke_update_preview(); }
    });
    let weak_board = Rc::downgrade(&board_bounds);
    let weak_frame = Rc::downgrade(&frame_geometry);
    let weak_viewport = Rc::downgrade(&viewport);
    let window_weak = main_window.as_weak();
    main_window.on_cursor_moved(move |x, y, width, height| {
        if width <= 0.0 || height <= 0.0 { return; }
        let (Some(window), Some(board), Some(frame), Some(viewport)) = (
            window_weak.upgrade(), weak_board.upgrade(), weak_frame.upgrade(), weak_viewport.upgrade(),
        ) else { return; };
        let (Some(bounds), Some(frame)) = (board.borrow().as_ref().cloned(), frame.borrow().as_ref().cloned()) else { return; };
        let data = preview::PreviewData::from_bounds(bounds.x_min, bounds.x_max, bounds.y_min, bounds.y_max, frame.left, frame.right, frame.bottom, frame.top);
        let camera = *viewport.borrow();
        let scale = data.calculate_scale(width, height) * camera.zoom;
        if scale <= 0.0 { return; }
        let min_x = data.board_x_min.min(data.frame_left);
        let min_y = data.board_y_min.min(data.frame_bottom);
        let content_width = data.board_x_max.max(data.frame_right) - min_x;
        let content_height = data.board_y_max.max(data.frame_top) - min_y;
        let world_x = min_x + (x as f64 - (width as f64 - content_width * scale) / 2.0 - camera.pan_x) / scale;
        let world_y = min_y + ((height as f64 + content_height * scale) / 2.0 + camera.pan_y - y as f64) / scale;
        window.set_cursor_coordinates(format!("X: {world_x:.3}   Y: {world_y:.3}").into());
    });

    // Save settings handler
    let board_bounds_clone = board_bounds.clone();
    let frame_geometry_clone = frame_geometry.clone();
    let weak_settings = Rc::downgrade(&current_settings);
    let window_weak = main_window.as_weak();

    main_window.on_save_settings(move || {
        let Some(window) = window_weak.upgrade() else { return; };
        let Some(settings_rc) = weak_settings.upgrade() else { return; };

        let ui_settings = window.global::<UiSettings>();
        
        let new_settings = settings::Settings {
            offset_x: ui_settings.get_offset_x() as f64,
            offset_y: ui_settings.get_offset_y() as f64,
            clamp_zone: ui_settings.get_clamp_zone() as f64,
            safe_zone: ui_settings.get_safe_zone() as f64,
            tool_diameter: ui_settings.get_tool_diameter() as f64,
            cut_depth: ui_settings.get_cut_depth() as f64,
            step_depth: ui_settings.get_step_depth() as f64,
            feed_rate: ui_settings.get_feed_rate() as f64,
            spindle_speed: ui_settings.get_spindle_speed() as f64,
        };

        if let Err(e) = new_settings.save() {
            window.invoke_show_error(format!("Failed to save settings: {}", e).into());
            return;
        }

        *settings_rc.borrow_mut() = new_settings;

        // Recalculate frame if we have board data
        if let Some(bounds) = board_bounds_clone.borrow().as_ref() {
            if let Some(frame) = frame::FrameGeometry::calculate(bounds, &*settings_rc.borrow()) {
                *frame_geometry_clone.borrow_mut() = Some(frame);
                window.invoke_update_preview();
            }
        }
    });

    // Export TAP handler
    let weak_board = Rc::downgrade(&board_bounds);
    let weak_frame = Rc::downgrade(&frame_geometry);
    let weak_toolpath = Rc::downgrade(&toolpath);
    let weak_rapid_path = Rc::downgrade(&rapid_path);
    let weak_viewport = Rc::downgrade(&viewport);
    let weak_settings = Rc::downgrade(&current_settings);
    let weak_home = Rc::downgrade(&source_home);
    let window_weak = main_window.as_weak();

    main_window.on_export_tap(move || {
        let Some(window) = window_weak.upgrade() else { return; };
        let Some(board_rc) = weak_board.upgrade() else { return; };
        let Some(frame_rc) = weak_frame.upgrade() else { return; };
        let Some(settings_rc) = weak_settings.upgrade() else { return; };
        let Some(home_rc) = weak_home.upgrade() else { return; };

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

        let Some(file_path) = rfd::FileDialog::new()
            .set_file_name("frame.tap")
            .save_file()
        else {
            return;
        };

        let gcode = exporter::generate_frame_gcode(
            bounds,
            frame,
            &settings_rc.borrow(),
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
        let width = width as u32;
        let height = height as u32;

        if width == 0 || height == 0 {
            return slint::Image::from_rgb8(SharedPixelBuffer::new(1, 1));
        }

        let Some(window) = window_weak.upgrade() else {
            return slint::Image::from_rgb8(SharedPixelBuffer::new(1, 1));
        };

        let Some(board_rc) = weak_board.upgrade() else {
            return slint::Image::from_rgb8(SharedPixelBuffer::new(width, height));
        };
        let Some(frame_rc) = weak_frame.upgrade() else {
            return slint::Image::from_rgb8(SharedPixelBuffer::new(width, height));
        };
        let Some(settings_rc) = weak_settings.upgrade() else { return slint::Image::from_rgb8(SharedPixelBuffer::new(width, height)); };
        let Some(path_rc) = weak_toolpath.upgrade() else { return slint::Image::from_rgb8(SharedPixelBuffer::new(width, height)); };
        let Some(rapid_rc) = weak_rapid_path.upgrade() else { return slint::Image::from_rgb8(SharedPixelBuffer::new(width, height)); };
        let Some(viewport_rc) = weak_viewport.upgrade() else { return slint::Image::from_rgb8(SharedPixelBuffer::new(width, height)); };

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

        let preview_data = preview::PreviewData::from_bounds(
            bounds.x_min, bounds.x_max, bounds.y_min, bounds.y_max,
            frame.left, frame.right, frame.bottom, frame.top,
        );

        let viewport = *viewport_rc.borrow();
        let scale = preview_data.calculate_scale(width as f32, height as f32) * viewport.zoom;
        let pan = (viewport.pan_x, viewport.pan_y);
        preview_renderer::draw_grid(&mut buffer, &preview_data, scale, pan);
        preview_renderer::axes(&mut buffer, &preview_data, scale, width as f32, height as f32, pan);

        let settings = settings_rc.borrow().clone();
        // Draw board (blue) - thicker lines

        // Draw frame (red) - thicker lines
        let path = path_rc.borrow();
        preview_renderer::dotted_polyline(&mut buffer, &path, &preview_data, scale, width as f32, height as f32, pan);
        let board_width = bounds.width().max(f64::MIN_POSITIVE);
        let board_height = bounds.height().max(f64::MIN_POSITIVE);
        let shifted_path: Vec<_> = path.iter().map(|&(x, y)| (
            frame.left + (x - bounds.x_min) * frame.width() / board_width,
            frame.bottom + (y - bounds.y_min) * frame.height() / board_height,
        )).collect();
        preview_renderer::polyline(&mut buffer, &shifted_path, &preview_data, scale, width as f32, height as f32, (0, 210, 255), 2, pan);
        let rapid = rapid_rc.borrow();
        preview_renderer::polyline(&mut buffer, &rapid, &preview_data, scale, width as f32, height as f32, (255, 190, 0), 1, pan);

        slint::Image::from_rgb8(buffer)
    });

    main_window.run()?;
    Ok(())
}
