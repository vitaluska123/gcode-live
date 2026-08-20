//! Temporary software renderer isolated from application wiring.
//! It will be replaced by the OpenGL backend without touching UI callbacks.

use crate::preview::PreviewData;
use slint::SharedPixelBuffer;

pub fn draw_grid(
    buffer: &mut SharedPixelBuffer<slint::Rgb8Pixel>,
    preview: &PreviewData,
    scale: f64,
    pan: (f64, f64),
) {
    let (width, height) = (buffer.width(), buffer.height());
    if scale <= 0.0 {
        return;
    }
    let (min_x, max_x, min_y, max_y) = preview.world_bounds();
    let base_x = (width as f64 - (max_x - min_x) * scale) / 2.0 + pan.0;
    let base_y = (height as f64 + (max_y - min_y) * scale) / 2.0 + pan.1;
    let world_left = min_x - base_x / scale;
    let world_right = min_x + (width as f64 - base_x) / scale;
    let world_bottom = min_y + (base_y - height as f64) / scale;
    let world_top = min_y + base_y / scale;
    let step = nice_grid_step(80.0 / scale);
    let mut labels = Vec::new();
    let pixels = buffer.make_mut_slice();
    let first_x = (world_left / step).floor() as i64;
    let last_x = (world_right / step).ceil() as i64;
    for index in first_x..=last_x {
        let x = (base_x + (index as f64 * step - min_x) * scale).round() as i32;
        if x < 0 || x >= width as i32 {
            continue;
        }
        for y in 0..height {
            pixels[(y * width + x as u32) as usize] = slint::Rgb8Pixel::new(42, 48, 58);
        }
        labels.push((x + 3, height as i32 - 11, index as f64 * step));
    }
    let first_y = (world_bottom / step).floor() as i64;
    let last_y = (world_top / step).ceil() as i64;
    for index in first_y..=last_y {
        let y = (base_y - (index as f64 * step - min_y) * scale).round() as i32;
        if y < 0 || y >= height as i32 {
            continue;
        }
        for x in 0..width {
            pixels[(y as u32 * width + x) as usize] = slint::Rgb8Pixel::new(42, 48, 58);
        }
        labels.push((3, y - 9, index as f64 * step));
    }
    for (x, y, value) in labels {
        draw_number(buffer, x, y, value);
    }
}

fn draw_number(buffer: &mut SharedPixelBuffer<slint::Rgb8Pixel>, x: i32, y: i32, value: f64) {
    let text = if value.abs() < 0.0001 {
        "0".to_owned()
    } else if value.abs() < 1.0 {
        format!("{value:.1}")
    } else {
        format!("{value:.0}")
    };
    for (offset, ch) in text.chars().enumerate() {
        draw_glyph(buffer, x + offset as i32 * 4, y, ch);
    }
}

fn draw_glyph(buffer: &mut SharedPixelBuffer<slint::Rgb8Pixel>, x: i32, y: i32, ch: char) {
    let glyph = match ch {
        '0' => [7, 5, 5, 5, 7],
        '1' => [2, 6, 2, 2, 7],
        '2' => [7, 1, 7, 4, 7],
        '3' => [7, 1, 7, 1, 7],
        '4' => [5, 5, 7, 1, 1],
        '5' => [7, 4, 7, 1, 7],
        '6' => [7, 4, 7, 5, 7],
        '7' => [7, 1, 2, 2, 2],
        '8' => [7, 5, 7, 5, 7],
        '9' => [7, 5, 7, 1, 7],
        '-' => [0, 0, 7, 0, 0],
        '.' => [0, 0, 0, 0, 2],
        _ => [0; 5],
    };
    let (width, height) = (buffer.width() as i32, buffer.height() as i32);
    let pixels = buffer.make_mut_slice();
    for (row, bits) in glyph.iter().enumerate() {
        for col in 0..3 {
            let (px, py) = (x + col, y + row as i32);
            if bits & (1 << (2 - col)) != 0 && px >= 0 && py >= 0 && px < width && py < height {
                pixels[(py * width + px) as usize] = slint::Rgb8Pixel::new(150, 160, 175);
            }
        }
    }
}

fn nice_grid_step(target: f64) -> f64 {
    let exponent = target.max(f64::MIN_POSITIVE).log10().floor();
    let base = 10_f64.powf(exponent);
    for multiplier in [1.0, 2.0, 5.0, 10.0] {
        let step = multiplier * base;
        if step >= target {
            return step;
        }
    }
    base * 10.0
}

/// Draw world-space axes. X is red, Y is green, and both follow the camera.
pub fn axes(
    buffer: &mut SharedPixelBuffer<slint::Rgb8Pixel>,
    preview: &PreviewData,
    scale: f64,
    width: f32,
    height: f32,
    pan: (f64, f64),
) {
    let (origin_x, origin_y) = preview.world_to_screen(0.0, 0.0, scale, width, height);
    let (x_axis_end, _) = preview.world_to_screen(1.0, 0.0, scale, width, height);
    let (_, y_axis_end) = preview.world_to_screen(0.0, 1.0, scale, width, height);
    let x_direction = (x_axis_end - origin_x).signum();
    let y_direction = (y_axis_end - origin_y).signum();
    let origin_x = origin_x + pan.0 as f32;
    let origin_y = origin_y + pan.1 as f32;
    line(buffer, 0.0, origin_y, width, origin_y, (220, 70, 70), 2);
    line(buffer, origin_x, 0.0, origin_x, height, (70, 210, 120), 2);
    // Arrow tips make the positive direction unambiguous.
    line(
        buffer,
        width - 10.0 * x_direction,
        origin_y - 5.0,
        width,
        origin_y,
        (220, 70, 70),
        2,
    );
    line(
        buffer,
        origin_x - 5.0,
        10.0 * y_direction,
        origin_x,
        0.0,
        (70, 210, 120),
        2,
    );
}

pub fn polyline(
    buffer: &mut SharedPixelBuffer<slint::Rgb8Pixel>,
    points: &[(f64, f64)],
    preview: &PreviewData,
    scale: f64,
    width: f32,
    height: f32,
    color: (u8, u8, u8),
    thickness: i32,
    pan: (f64, f64),
) {
    for pair in points.windows(2) {
        let (x1, y1) = preview.world_to_screen(pair[0].0, pair[0].1, scale, width, height);
        let (x2, y2) = preview.world_to_screen(pair[1].0, pair[1].1, scale, width, height);
        line(
            buffer,
            x1 + pan.0 as f32,
            y1 + pan.1 as f32,
            x2 + pan.0 as f32,
            y2 + pan.1 as f32,
            color,
            thickness,
        );
    }
}

pub fn dotted_rectangle(
    buffer: &mut SharedPixelBuffer<slint::Rgb8Pixel>,
    corners: &[(f64, f64)],
    preview: &PreviewData,
    scale: f64,
    width: f32,
    height: f32,
    pan: (f64, f64),
) {
    for pair in corners.windows(2) {
        let (x1, y1) = preview.world_to_screen(pair[0].0, pair[0].1, scale, width, height);
        let (x2, y2) = preview.world_to_screen(pair[1].0, pair[1].1, scale, width, height);
        let dx = x2 - x1;
        let dy = y2 - y1;
        let length = (dx * dx + dy * dy).sqrt().max(1.0);
        let segments = (length / 12.0).ceil() as i32;
        for segment in (0..segments).step_by(2) {
            let t1 = segment as f32 / segments as f32;
            let t2 = ((segment + 1).min(segments)) as f32 / segments as f32;
            line(
                buffer,
                x1 + dx * t1 + pan.0 as f32,
                y1 + dy * t1 + pan.1 as f32,
                x1 + dx * t2 + pan.0 as f32,
                y1 + dy * t2 + pan.1 as f32,
                (255, 210, 0),
                2,
            );
        }
    }
}

pub fn rectangle(
    buffer: &mut SharedPixelBuffer<slint::Rgb8Pixel>,
    corners: &[(f64, f64)],
    preview: &PreviewData,
    scale: f64,
    width: f32,
    height: f32,
    color: (u8, u8, u8),
    pan: (f64, f64),
) {
    polyline(
        buffer, corners, preview, scale, width, height, color, 2, pan,
    );
}

fn line(
    buffer: &mut SharedPixelBuffer<slint::Rgb8Pixel>,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    color: (u8, u8, u8),
    thickness: i32,
) {
    let buffer_width = buffer.width() as i32;
    let buffer_height = buffer.height() as i32;
    let pixels = buffer.make_mut_slice();
    for offset in 0..thickness {
        let (mut x, mut y) = (x0.round() as i32, y0.round() as i32 + offset);
        let (end_x, end_y) = (x1.round() as i32, y1.round() as i32 + offset);
        let (dx, dy) = ((end_x - x).abs(), -(end_y - y).abs());
        let (step_x, step_y) = (
            if x < end_x { 1 } else { -1 },
            if y < end_y { 1 } else { -1 },
        );
        let mut error = dx + dy;
        loop {
            if x >= 0 && y >= 0 && x < buffer_width && y < buffer_height {
                pixels[(y * buffer_width + x) as usize] =
                    slint::Rgb8Pixel::new(color.0, color.1, color.2);
            }
            if x == end_x && y == end_y {
                break;
            }
            let twice = 2 * error;
            if twice >= dy {
                error += dy;
                x += step_x;
            }
            if twice <= dx {
                error += dx;
                y += step_y;
            }
        }
    }
}
