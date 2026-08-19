slint::include_modules!();

use slint::{ComponentHandle, SharedPixelBuffer};
use std::rc::Rc;
use std::cell::RefCell;

mod settings;
mod frame;
mod exporter;
mod preview;

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
    ui_settings.set_feed_rate(settings.feed_rate as i32);
    ui_settings.set_spindle_speed(settings.spindle_speed as i32);

    // State management
    let board_bounds = Rc::new(RefCell::new(None));
    let frame_geometry = Rc::new(RefCell::new(None));
    let current_settings = Rc::new(RefCell::new(settings));

    // Open TAP file handler
    let weak_board = Rc::downgrade(&board_bounds);
    let weak_frame = Rc::downgrade(&frame_geometry);
    let weak_settings = Rc::downgrade(&current_settings);
    let window_weak = main_window.as_weak();

    main_window.on_open_tap_file(move || {
        let Some(window) = window_weak.upgrade() else { return; };
        let Some(board_rc) = weak_board.upgrade() else { return; };
        let Some(frame_rc) = weak_frame.upgrade() else { return; };
        let Some(settings_rc) = weak_settings.upgrade() else { return; };

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

        if !bounds.is_valid() {
            window.invoke_show_error("No valid G-code coordinates found in file.".into());
            return;
        }

        let settings = settings_rc.borrow();
        let frame = match frame::FrameGeometry::calculate(&bounds, &settings) {
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

        // Trigger preview update
        window.invoke_update_preview();
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
    let weak_settings = Rc::downgrade(&current_settings);
    let window_weak = main_window.as_weak();

    main_window.on_export_tap(move || {
        let Some(window) = window_weak.upgrade() else { return; };
        let Some(board_rc) = weak_board.upgrade() else { return; };
        let Some(frame_rc) = weak_frame.upgrade() else { return; };
        let Some(settings_rc) = weak_settings.upgrade() else { return; };

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

        let gcode = exporter::generate_frame_gcode(bounds, frame, &*settings_rc.borrow());

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

        let mut buffer = SharedPixelBuffer::<slint::Rgb8Pixel>::new(width, height);

        // Fill with white background
        for pixel in buffer.make_mut_slice() {
            *pixel = slint::Rgb8Pixel::new(255, 255, 255);
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

        let scale = preview_data.calculate_scale(width as f32, height as f32);

        // Calculate common offset for centering
        let all_x_min = bounds.x_min.min(frame.left);
        let all_y_min = bounds.y_min.min(frame.bottom);
        
        // Draw board (blue) - thicker lines
        draw_rectangle_centered(&mut buffer, &preview_data.board_corners(), scale, 
                                width as f32, height as f32, (0, 100, 200), all_x_min, all_y_min);

        // Draw frame (red) - thicker lines
        draw_rectangle_centered(&mut buffer, &preview_data.frame_corners(), scale,
                                width as f32, height as f32, (200, 0, 0), all_x_min, all_y_min);

        slint::Image::from_rgb8(buffer)
    });

    main_window.run()?;
    Ok(())
}

/// Draw a rectangle on the pixel buffer with centered positioning
fn draw_rectangle_centered(
    buffer: &mut SharedPixelBuffer<slint::Rgb8Pixel>,
    corners: &[(f64, f64)],
    scale: f64,
    width: f32,
    height: f32,
    color: (u8, u8, u8),
    offset_x: f64,
    offset_y: f64,
) {
    if corners.len() < 2 {
        return;
    }

    for i in 0..corners.len() - 1 {
        let (x1, y1) = corners[i];
        let (x2, y2) = corners[i + 1];

        let (sx1, sy1) = transform_point_centered(x1, y1, scale, width, height, offset_x, offset_y);
        let (sx2, sy2) = transform_point_centered(x2, y2, scale, width, height, offset_x, offset_y);

        draw_thick_line(buffer, sx1, sy1, sx2, sy2, color, 2);
    }
}

/// Transform a point to screen coordinates with common offset
fn transform_point_centered(
    x: f64,
    y: f64,
    scale: f64,
    _width: f32,
    height: f32,
    offset_x: f64,
    offset_y: f64,
) -> (f32, f32) {
    let padding = 20.0;
    let sx = ((x - offset_x) * scale) as f32 + padding;
    let sy = height - ((y - offset_y) * scale) as f32 - padding;

    (sx, sy)
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
