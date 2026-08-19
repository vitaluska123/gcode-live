slint::include_modules!();

use slint::{ComponentHandle, SharedPixelBuffer};
use std::rc::Rc;
use std::cell::RefCell;

mod settings;
mod frame;
mod exporter;
mod preview;
mod preview_renderer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let main_window = MainWindow::new()?;

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
    let preview_zoom = Rc::new(RefCell::new(1.0_f64));
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

    let zoom_state = preview_zoom.clone();
    let window_weak = main_window.as_weak();
    main_window.on_zoom_preview(move |direction| {
        {
            let mut zoom = zoom_state.borrow_mut();
            *zoom = (*zoom * if direction > 0 { 1.25 } else { 0.8 }).clamp(0.1, 20.0);
        }
        if let Some(window) = window_weak.upgrade() { window.invoke_update_preview(); }
    });
    let zoom_state = preview_zoom.clone();
    let window_weak = main_window.as_weak();
    main_window.on_fit_preview(move || {
        *zoom_state.borrow_mut() = 1.0;
        if let Some(window) = window_weak.upgrade() { window.invoke_update_preview(); }
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
    let weak_zoom = Rc::downgrade(&preview_zoom);
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
    let window_weak = main_window.as_weak();

    main_window.on_render_preview(move |width, height| {
        let width = width as u32;
        let height = height as u32;

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
        let Some(path_rc) = weak_toolpath.upgrade() else { return slint::Image::from_rgb8(SharedPixelBuffer::new(width, height)); };
        let Some(rapid_rc) = weak_rapid_path.upgrade() else { return slint::Image::from_rgb8(SharedPixelBuffer::new(width, height)); };
        let Some(zoom_rc) = weak_zoom.upgrade() else { return slint::Image::from_rgb8(SharedPixelBuffer::new(width, height)); };

        let mut buffer = SharedPixelBuffer::<slint::Rgb8Pixel>::new(width, height);

        // Dark editor-like background and a neutral grid.
        for pixel in buffer.make_mut_slice() {
            *pixel = slint::Rgb8Pixel::new(14, 17, 22);
        }
        preview_renderer::draw_grid(&mut buffer);

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

        let scale = preview_data.calculate_scale(width as f32, height as f32) * *zoom_rc.borrow();

        // Draw board (blue) - thicker lines
        preview_renderer::rectangle(&mut buffer, &preview_data.board_corners(), &preview_data, scale,
                       width as f32, height as f32, (0, 100, 200));

        // Draw frame (red) - thicker lines
        preview_renderer::rectangle(&mut buffer, &preview_data.frame_corners(), &preview_data, scale,
                       width as f32, height as f32, (200, 0, 0));
        let path = path_rc.borrow();
        preview_renderer::polyline(&mut buffer, &path, &preview_data, scale, width as f32, height as f32, (0, 210, 255), 2);
        let rapid = rapid_rc.borrow();
        preview_renderer::polyline(&mut buffer, &rapid, &preview_data, scale, width as f32, height as f32, (255, 190, 0), 1);

        slint::Image::from_rgb8(buffer)
    });

    main_window.run()?;
    Ok(())
}

fn draw_grid(buffer: &mut SharedPixelBuffer<slint::Rgb8Pixel>, spacing: u32, color: (u8, u8, u8)) {
    let width = buffer.width();
    let height = buffer.height();
    let pixels = buffer.make_mut_slice();
    for x in (0..width).step_by(spacing as usize) {
        for y in 0..height { pixels[(y * width + x) as usize] = slint::Rgb8Pixel::new(color.0, color.1, color.2); }
    }
    for y in (0..height).step_by(spacing as usize) {
        for x in 0..width { pixels[(y * width + x) as usize] = slint::Rgb8Pixel::new(color.0, color.1, color.2); }
    }
}

fn draw_toolpath(buffer: &mut SharedPixelBuffer<slint::Rgb8Pixel>, points: &[(f64, f64)], preview: &preview::PreviewData, scale: f64, width: f32, height: f32) {
    draw_toolpath_color(buffer, points, preview, scale, width, height, (0, 210, 255), 2);
}

fn draw_toolpath_color(buffer: &mut SharedPixelBuffer<slint::Rgb8Pixel>, points: &[(f64, f64)], preview: &preview::PreviewData, scale: f64, width: f32, height: f32, color: (u8, u8, u8), thickness: i32) {
    for pair in points.windows(2) {
        let (x1, y1) = preview.world_to_screen(pair[0].0, pair[0].1, scale, width, height);
        let (x2, y2) = preview.world_to_screen(pair[1].0, pair[1].1, scale, width, height);
        draw_thick_line(buffer, x1, y1, x2, y2, color, thickness);
    }
}

/// Draw a rectangle on the pixel buffer with centered positioning
fn draw_rectangle(
    buffer: &mut SharedPixelBuffer<slint::Rgb8Pixel>,
    corners: &[(f64, f64)],
    preview_data: &preview::PreviewData,
    scale: f64,
    width: f32,
    height: f32,
    color: (u8, u8, u8),
) {
    if corners.len() < 2 {
        return;
    }

    for i in 0..corners.len() - 1 {
        let (x1, y1) = corners[i];
        let (x2, y2) = corners[i + 1];

        let (sx1, sy1) = preview_data.world_to_screen(x1, y1, scale, width, height);
        let (sx2, sy2) = preview_data.world_to_screen(x2, y2, scale, width, height);

        draw_thick_line(buffer, sx1, sy1, sx2, sy2, color, 2);
    }
}

/// Draw a thick line using Bresenham's algorithm with line width
fn draw_thick_line(
    buffer: &mut SharedPixelBuffer<slint::Rgb8Pixel>,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    color: (u8, u8, u8),
    thickness: i32,
) {
    let width = buffer.width() as i32;
    let height = buffer.height() as i32;
    
    // For thicker lines, draw multiple parallel lines
    for offset in 0..thickness {
        let off = offset as f32;
        draw_line_basic(buffer, x0, y0 + off, x1, y1 + off, color, width, height);
    }
}

/// Basic line drawing without thickness
fn draw_line_basic(
    buffer: &mut SharedPixelBuffer<slint::Rgb8Pixel>,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    color: (u8, u8, u8),
    buf_width: i32,
    buf_height: i32,
) {
    let mut x0 = x0.round() as i32;
    let mut y0 = y0.round() as i32;
    let x1 = x1.round() as i32;
    let y1 = y1.round() as i32;

    let dx = (x1 - x0).abs();
    let dy = (y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx - dy;

    loop {
        if x0 >= 0 && x0 < buf_width && y0 >= 0 && y0 < buf_height {
            let pixels = buffer.make_mut_slice();
            let idx = (y0 * buf_width + x0) as usize;
            if idx < pixels.len() {
                pixels[idx] = slint::Rgb8Pixel::new(color.0, color.1, color.2);
            }
        }

        if x0 == x1 && y0 == y1 {
            break;
        }

        let e2 = 2 * err;
        if e2 > -dy {
            err -= dy;
            x0 += sx;
        }
        if e2 < dx {
            err += dx;
            y0 += sy;
        }
    }
}

/// Draw a line using Bresenham's algorithm (kept for compatibility)
#[allow(dead_code)]
fn draw_line(
    buffer: &mut SharedPixelBuffer<slint::Rgb8Pixel>,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    color: (u8, u8, u8),
) {
    draw_thick_line(buffer, x0, y0, x1, y1, color, 1);
}
