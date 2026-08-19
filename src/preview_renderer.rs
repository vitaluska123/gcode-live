//! Temporary software renderer isolated from application wiring.
//! It will be replaced by the OpenGL backend without touching UI callbacks.

use crate::preview::PreviewData;
use slint::SharedPixelBuffer;

pub fn draw_grid(buffer: &mut SharedPixelBuffer<slint::Rgb8Pixel>) {
    let (width, height) = (buffer.width(), buffer.height());
    let pixels = buffer.make_mut_slice();
    for x in (0..width).step_by(50) {
        for y in 0..height { pixels[(y * width + x) as usize] = slint::Rgb8Pixel::new(42, 48, 58); }
    }
    for y in (0..height).step_by(50) {
        for x in 0..width { pixels[(y * width + x) as usize] = slint::Rgb8Pixel::new(42, 48, 58); }
    }
}

pub fn polyline(buffer: &mut SharedPixelBuffer<slint::Rgb8Pixel>, points: &[(f64, f64)], preview: &PreviewData, scale: f64, width: f32, height: f32, color: (u8, u8, u8), thickness: i32, pan: (f64, f64)) {
    for pair in points.windows(2) {
        let (x1, y1) = preview.world_to_screen(pair[0].0, pair[0].1, scale, width, height);
        let (x2, y2) = preview.world_to_screen(pair[1].0, pair[1].1, scale, width, height);
        line(buffer, x1 + pan.0 as f32, y1 + pan.1 as f32, x2 + pan.0 as f32, y2 + pan.1 as f32, color, thickness);
    }
}

pub fn rectangle(buffer: &mut SharedPixelBuffer<slint::Rgb8Pixel>, corners: &[(f64, f64)], preview: &PreviewData, scale: f64, width: f32, height: f32, color: (u8, u8, u8), pan: (f64, f64)) {
    polyline(buffer, corners, preview, scale, width, height, color, 2, pan);
}

fn line(buffer: &mut SharedPixelBuffer<slint::Rgb8Pixel>, x0: f32, y0: f32, x1: f32, y1: f32, color: (u8, u8, u8), thickness: i32) {
    let buffer_width = buffer.width() as i32;
    let buffer_height = buffer.height() as i32;
    for offset in 0..thickness {
        let (mut x, mut y) = (x0.round() as i32, y0.round() as i32 + offset);
        let (end_x, end_y) = (x1.round() as i32, y1.round() as i32 + offset);
        let (dx, dy) = ((end_x - x).abs(), -(end_y - y).abs());
        let (step_x, step_y) = (if x < end_x { 1 } else { -1 }, if y < end_y { 1 } else { -1 });
        let mut error = dx + dy;
        loop {
            if x >= 0 && y >= 0 && x < buffer_width && y < buffer_height {
                buffer.make_mut_slice()[(y * buffer_width + x) as usize] = slint::Rgb8Pixel::new(color.0, color.1, color.2);
            }
            if x == end_x && y == end_y { break; }
            let twice = 2 * error;
            if twice >= dy { error += dy; x += step_x; }
            if twice <= dx { error += dx; y += step_y; }
        }
    }
}
