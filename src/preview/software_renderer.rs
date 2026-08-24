//! CPU preview rasterizer.

use crate::domain::frame;
use crate::preview::data::PreviewData;
use crate::preview::renderer::{PreviewRenderer, RenderFrame};
use slint::SharedPixelBuffer;

type Rgba = (u8, u8, u8, u8);

/// The existing CPU rasterizer, retained as the dependable fallback backend.
#[derive(Default)]
pub struct SoftwarePreviewRenderer;

impl PreviewRenderer for SoftwarePreviewRenderer {
    fn render(&mut self, frame: &RenderFrame) -> slint::Image {
        render_software_frame(frame)
    }
}

fn preview_color(value: &str, fallback: (u8, u8, u8)) -> Rgba {
    let hex = value.trim().trim_start_matches('#');
    if hex.len() != 6 && hex.len() != 8 {
        return (fallback.0, fallback.1, fallback.2, 255);
    }
    let parse = |range| u8::from_str_radix(&hex[range], 16).ok();
    match (parse(0..2), parse(2..4), parse(4..6)) {
        (Some(r), Some(g), Some(b)) => {
            let alpha = if hex.len() == 8 {
                parse(6..8).unwrap_or(255)
            } else {
                255
            };
            (r, g, b, alpha)
        }
        _ => (fallback.0, fallback.1, fallback.2, 255),
    }
}

pub fn draw_grid(
    buffer: &mut SharedPixelBuffer<slint::Rgb8Pixel>,
    preview: &PreviewData,
    scale: f64,
    pan: (f64, f64),
    line_color: Rgba,
    label_color: Rgba,
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
            blend_pixel(
                &mut pixels[(y * width + x as u32) as usize],
                line_color,
                1.0,
            );
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
            blend_pixel(
                &mut pixels[(y as u32 * width + x) as usize],
                line_color,
                1.0,
            );
        }
        labels.push((3, y - 9, index as f64 * step));
    }
    for (x, y, value) in labels {
        draw_number(buffer, x, y, value, label_color);
    }
}

fn draw_number(
    buffer: &mut SharedPixelBuffer<slint::Rgb8Pixel>,
    x: i32,
    y: i32,
    value: f64,
    color: Rgba,
) {
    let text = if value.abs() < 0.0001 {
        "0".to_owned()
    } else if value.abs() < 1.0 {
        format!("{value:.1}")
    } else {
        format!("{value:.0}")
    };
    for (offset, ch) in text.chars().enumerate() {
        draw_glyph(buffer, x + offset as i32 * 4, y, ch, color);
    }
}

fn draw_glyph(
    buffer: &mut SharedPixelBuffer<slint::Rgb8Pixel>,
    x: i32,
    y: i32,
    ch: char,
    color: Rgba,
) {
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
                blend_pixel(&mut pixels[(py * width + px) as usize], color, 1.0);
            }
        }
    }
}

fn render_software_frame(frame: &RenderFrame) -> slint::Image {
    let width = frame.width;
    let height = frame.height;
    let mut buffer = SharedPixelBuffer::<slint::Rgb8Pixel>::new(width, height);

    // Dark editor-like background and a neutral grid.
    for pixel in buffer.make_mut_slice() {
        let color = preview_color(&frame.settings.background_color, (14, 17, 22));
        blend_pixel(pixel, color, 1.0);
    }
    let Some(bounds) = frame.scene.board_bounds.as_ref() else {
        return slint::Image::from_rgb8(buffer);
    };

    let Some(frame_geometry) = frame.scene.frame_geometry.as_ref() else {
        return slint::Image::from_rgb8(buffer);
    };

    let settings = &frame.settings;
    let (shift_x, shift_y) = settings.local_offset();
    let expanded = frame::FrameGeometry::expanded(bounds, settings);
    let preview_left = expanded.as_ref().map_or(frame_geometry.left, |value| {
        frame_geometry.left.min(value.left)
    });
    let preview_right = expanded.as_ref().map_or(frame_geometry.right, |value| {
        frame_geometry.right.max(value.right)
    });
    let preview_bottom = expanded.as_ref().map_or(frame_geometry.bottom, |value| {
        frame_geometry.bottom.min(value.bottom)
    });
    let preview_top = expanded.as_ref().map_or(frame_geometry.top, |value| {
        frame_geometry.top.max(value.top)
    });
    let preview_data = PreviewData::from_bounds_with_material(
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
        settings.material_offset_x,
        settings.material_offset_y,
    );

    let viewport = frame.viewport;
    let scale = preview_data.calculate_scale(width as f32, height as f32) * viewport.zoom;
    let pan = (viewport.pan_x, viewport.pan_y);
    if settings.show_grid {
        draw_grid(
            &mut buffer,
            &preview_data,
            scale,
            pan,
            preview_color(&settings.grid_color, (42, 48, 58)),
            preview_color(&settings.grid_label_color, (150, 160, 175)),
        );
    }
    if settings.show_axes {
        axes(
            &mut buffer,
            &preview_data,
            scale,
            width as f32,
            height as f32,
            pan,
            preview_color(&settings.axis_x_color, (220, 70, 70)),
            preview_color(&settings.axis_y_color, (70, 210, 120)),
        );
    }
    if settings.show_local_axes && settings.local_offset_enabled {
        local_axes(
            &mut buffer,
            &preview_data,
            scale,
            width as f32,
            height as f32,
            pan,
            shift_x,
            shift_y,
            preview_color(&settings.local_axis_x_color, (220, 70, 70)),
            preview_color(&settings.local_axis_y_color, (70, 210, 120)),
        );
    }

    // Draw board (blue) - thicker lines

    // Draw frame (red) - thicker lines
    let shifted_path: Vec<_> = frame
        .scene
        .toolpath
        .iter()
        .map(|&(x, y)| (x + shift_x, y + shift_y))
        .collect();
    if settings.show_toolpath {
        polyline(
            &mut buffer,
            &shifted_path,
            &preview_data,
            scale,
            width as f32,
            height as f32,
            preview_color(&settings.toolpath_color, (0, 210, 255)),
            2,
            pan,
        );
    }
    let material = [
        (
            settings.material_offset_x - settings.material_width,
            settings.material_offset_y,
        ),
        (settings.material_offset_x, settings.material_offset_y),
        (
            settings.material_offset_x,
            settings.material_offset_y + settings.material_height,
        ),
        (
            settings.material_offset_x - settings.material_width,
            settings.material_offset_y + settings.material_height,
        ),
        (
            settings.material_offset_x - settings.material_width,
            settings.material_offset_y,
        ),
    ];
    if settings.show_material {
        dotted_rectangle(
            &mut buffer,
            &material,
            &preview_data,
            scale,
            width as f32,
            height as f32,
            preview_color(&settings.material_color, (190, 100, 255)),
            pan,
        );
    }
    let edge_margin_x = settings.material_edge_margin_x.max(0.0);
    let edge_margin_y = settings.material_edge_margin_y.max(0.0);
    let safe_area = [
        (
            settings.material_offset_x - settings.material_width + edge_margin_x,
            settings.material_offset_y + edge_margin_y,
        ),
        (
            settings.material_offset_x - edge_margin_x,
            settings.material_offset_y + edge_margin_y,
        ),
        (
            settings.material_offset_x - edge_margin_x,
            settings.material_offset_y + settings.material_height - edge_margin_y,
        ),
        (
            settings.material_offset_x - settings.material_width + edge_margin_x,
            settings.material_offset_y + settings.material_height - edge_margin_y,
        ),
        (
            settings.material_offset_x - settings.material_width + edge_margin_x,
            settings.material_offset_y + edge_margin_y,
        ),
    ];
    if settings.show_margin_hatch {
        hatched_margin(
            &mut buffer,
            settings.material_offset_x - settings.material_width,
            settings.material_offset_x,
            settings.material_offset_y,
            settings.material_offset_y + settings.material_height,
            settings.material_offset_x - settings.material_width + edge_margin_x,
            settings.material_offset_x - edge_margin_x,
            settings.material_offset_y + edge_margin_y,
            settings.material_offset_y + settings.material_height - edge_margin_y,
            &preview_data,
            scale,
            width as f32,
            height as f32,
            pan,
            preview_color(&settings.margin_hatch_color, (255, 70, 70)),
        );
    }
    if settings.show_safe_area {
        dotted_rectangle(
            &mut buffer,
            &safe_area,
            &preview_data,
            scale,
            width as f32,
            height as f32,
            preview_color(&settings.safe_area_color, (255, 70, 70)),
            pan,
        );
    }
    if settings.show_expanded_frame {
        if let Some(expanded) = expanded {
            let corners = [
                (expanded.left + shift_x, expanded.bottom + shift_y),
                (expanded.right + shift_x, expanded.bottom + shift_y),
                (expanded.right + shift_x, expanded.top + shift_y),
                (expanded.left + shift_x, expanded.top + shift_y),
                (expanded.left + shift_x, expanded.bottom + shift_y),
            ];
            dotted_rectangle(
                &mut buffer,
                &corners,
                &preview_data,
                scale,
                width as f32,
                height as f32,
                preview_color(&settings.expanded_frame_color, (255, 210, 0)),
                pan,
            );
        }
    }
    let anchored_corners = [
        (
            frame_geometry.left + shift_x,
            frame_geometry.bottom + shift_y,
        ),
        (
            frame_geometry.right + shift_x,
            frame_geometry.bottom + shift_y,
        ),
        (frame_geometry.right + shift_x, frame_geometry.top + shift_y),
        (frame_geometry.left + shift_x, frame_geometry.top + shift_y),
        (
            frame_geometry.left + shift_x,
            frame_geometry.bottom + shift_y,
        ),
    ];
    if settings.show_frame {
        rectangle(
            &mut buffer,
            &anchored_corners,
            &preview_data,
            scale,
            width as f32,
            height as f32,
            preview_color(&settings.frame_color, (255, 70, 100)),
            pan,
        );
    }
    let corner_radius = (settings.tool_diameter / 2.0)
        .min(1.0)
        .min(frame_geometry.width() / 4.0)
        .min(frame_geometry.height() / 4.0)
        .max(0.0);
    if settings.show_tabs {
        for (left, right) in frame::top_tab_intervals(frame_geometry, corner_radius, settings) {
            polyline(
                &mut buffer,
                &[
                    (left + shift_x, frame_geometry.top + shift_y),
                    (right + shift_x, frame_geometry.top + shift_y),
                ],
                &preview_data,
                scale,
                width as f32,
                height as f32,
                preview_color(&settings.tab_color, (255, 220, 70)),
                4,
                pan,
            );
        }
    }
    let shifted_rapid: Vec<_> = frame
        .scene
        .rapid_path
        .iter()
        .map(|&(x, y)| (x + shift_x, y + shift_y))
        .collect();
    if settings.show_rapid {
        polyline(
            &mut buffer,
            &shifted_rapid,
            &preview_data,
            scale,
            width as f32,
            height as f32,
            preview_color(&settings.rapid_color, (255, 190, 0)),
            1,
            pan,
        );
    }

    slint::Image::from_rgb8(buffer)
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
#[allow(clippy::too_many_arguments)] // Axis colors are independent render settings.
pub fn axes(
    buffer: &mut SharedPixelBuffer<slint::Rgb8Pixel>,
    preview: &PreviewData,
    scale: f64,
    width: f32,
    height: f32,
    pan: (f64, f64),
    x_color: Rgba,
    y_color: Rgba,
) {
    axes_at(
        buffer, preview, scale, width, height, pan, 0.0, 0.0, 1.0, x_color, y_color,
    );
}

/// Draw the local coordinate plane at its global origin. Half opacity keeps it
/// visually distinct from the machine/global axes.
#[allow(clippy::too_many_arguments)] // Renderer primitive parameters remain allocation-free.
pub fn local_axes(
    buffer: &mut SharedPixelBuffer<slint::Rgb8Pixel>,
    preview: &PreviewData,
    scale: f64,
    width: f32,
    height: f32,
    pan: (f64, f64),
    origin_x_world: f64,
    origin_y_world: f64,
    x_color: Rgba,
    y_color: Rgba,
) {
    axes_at(
        buffer,
        preview,
        scale,
        width,
        height,
        pan,
        origin_x_world,
        origin_y_world,
        0.5,
        x_color,
        y_color,
    );
}

#[allow(clippy::too_many_arguments)] // Shared renderer primitive parameters remain explicit.
fn axes_at(
    buffer: &mut SharedPixelBuffer<slint::Rgb8Pixel>,
    preview: &PreviewData,
    scale: f64,
    width: f32,
    height: f32,
    pan: (f64, f64),
    origin_x_world: f64,
    origin_y_world: f64,
    opacity: f32,
    x_color: Rgba,
    y_color: Rgba,
) {
    let (origin_x, origin_y) =
        preview.world_to_screen(origin_x_world, origin_y_world, scale, width, height);
    let (x_axis_end, _) =
        preview.world_to_screen(origin_x_world + 1.0, origin_y_world, scale, width, height);
    let (_, y_axis_end) =
        preview.world_to_screen(origin_x_world, origin_y_world + 1.0, scale, width, height);
    let x_direction = (x_axis_end - origin_x).signum();
    let y_direction = (y_axis_end - origin_y).signum();
    let origin_x = origin_x + pan.0 as f32;
    let origin_y = origin_y + pan.1 as f32;
    line_alpha(buffer, 0.0, origin_y, width, origin_y, x_color, 2, opacity);
    line_alpha(buffer, origin_x, 0.0, origin_x, height, y_color, 2, opacity);
    // Arrow tips make the positive direction unambiguous.
    line_alpha(
        buffer,
        width - 10.0 * x_direction,
        origin_y - 5.0,
        width,
        origin_y,
        x_color,
        2,
        opacity,
    );
    line_alpha(
        buffer,
        origin_x - 5.0,
        10.0 * y_direction,
        origin_x,
        0.0,
        y_color,
        2,
        opacity,
    );
}

#[allow(clippy::too_many_arguments)] // Renderer primitive parameters remain allocation-free.
pub fn polyline(
    buffer: &mut SharedPixelBuffer<slint::Rgb8Pixel>,
    points: &[(f64, f64)],
    preview: &PreviewData,
    scale: f64,
    width: f32,
    height: f32,
    color: Rgba,
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

#[allow(clippy::too_many_arguments)] // Renderer primitive parameters remain allocation-free.
pub fn dotted_rectangle(
    buffer: &mut SharedPixelBuffer<slint::Rgb8Pixel>,
    corners: &[(f64, f64)],
    preview: &PreviewData,
    scale: f64,
    width: f32,
    height: f32,
    color: Rgba,
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
            line_alpha(
                buffer,
                x1 + dx * t1 + pan.0 as f32,
                y1 + dy * t1 + pan.1 as f32,
                x1 + dx * t2 + pan.0 as f32,
                y1 + dy * t2 + pan.1 as f32,
                color,
                2,
                0.3,
            );
        }
    }
}

/// Fill the area between two axis-aligned rectangles with a translucent hatch.
#[allow(clippy::too_many_arguments)] // The two rectangles are passed as explicit bounds.
pub fn hatched_margin(
    buffer: &mut SharedPixelBuffer<slint::Rgb8Pixel>,
    outer_left: f64,
    outer_right: f64,
    outer_bottom: f64,
    outer_top: f64,
    inner_left: f64,
    inner_right: f64,
    inner_bottom: f64,
    inner_top: f64,
    preview: &PreviewData,
    scale: f64,
    width: f32,
    height: f32,
    pan: (f64, f64),
    color: Rgba,
) {
    if inner_left <= outer_left
        || inner_right >= outer_right
        || inner_bottom <= outer_bottom
        || inner_top >= outer_top
    {
        return;
    }

    let (outer_x1, outer_y1) = preview.world_to_screen(outer_left, outer_top, scale, width, height);
    let (outer_x2, outer_y2) =
        preview.world_to_screen(outer_right, outer_bottom, scale, width, height);
    let (inner_x1, inner_y1) = preview.world_to_screen(inner_left, inner_top, scale, width, height);
    let (inner_x2, inner_y2) =
        preview.world_to_screen(inner_right, inner_bottom, scale, width, height);
    let (left, right) = (
        outer_x1.min(outer_x2) + pan.0 as f32,
        outer_x1.max(outer_x2) + pan.0 as f32,
    );
    let (top, bottom) = (
        outer_y1.min(outer_y2) + pan.1 as f32,
        outer_y1.max(outer_y2) + pan.1 as f32,
    );
    let excluded = (
        inner_x1.min(inner_x2) + pan.0 as f32,
        inner_x1.max(inner_x2) + pan.0 as f32,
        inner_y1.min(inner_y2) + pan.1 as f32,
        inner_y1.max(inner_y2) + pan.1 as f32,
    );

    // Do not trace hatching outside the viewport. At high zoom, the material
    // can be thousands of screen pixels wide even though only a small part is
    // visible; drawing those off-screen pixels made panning noticeably slow.
    let visible_left = left.max(0.0);
    let visible_right = right.min(width);
    let visible_top = top.max(0.0);
    let visible_bottom = bottom.min(height);
    if visible_left >= visible_right || visible_top >= visible_bottom {
        return;
    }

    let visible_diagonal_height = visible_bottom - visible_top;
    // Keep the preview responsive: never draw more than 80 hatch strokes.
    // The spacing grows only when necessary at large zoom levels.
    let hatch_step = 12.0_f32.max((visible_right - visible_left + visible_diagonal_height) / 80.0);
    let mut start_x = visible_left - visible_diagonal_height;
    while start_x < visible_right {
        line_alpha_outside_rect(
            buffer,
            start_x,
            visible_top,
            start_x + visible_diagonal_height,
            visible_bottom,
            color,
            1,
            0.25,
            (left, right, top, bottom),
            excluded,
        );
        start_x += hatch_step;
    }
}

#[allow(clippy::too_many_arguments)] // Renderer primitive parameters remain allocation-free.
pub fn rectangle(
    buffer: &mut SharedPixelBuffer<slint::Rgb8Pixel>,
    corners: &[(f64, f64)],
    preview: &PreviewData,
    scale: f64,
    width: f32,
    height: f32,
    color: Rgba,
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
    color: Rgba,
    thickness: i32,
) {
    line_alpha(buffer, x0, y0, x1, y1, color, thickness, 1.0);
}

#[allow(clippy::too_many_arguments)] // Raster line primitive parameters remain explicit.
fn line_alpha(
    buffer: &mut SharedPixelBuffer<slint::Rgb8Pixel>,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    color: Rgba,
    thickness: i32,
    opacity: f32,
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
                blend_pixel(&mut pixels[(y * buffer_width + x) as usize], color, opacity);
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

#[allow(clippy::too_many_arguments)] // Clipping rectangles are passed without temporary allocations.
fn line_alpha_outside_rect(
    buffer: &mut SharedPixelBuffer<slint::Rgb8Pixel>,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    color: Rgba,
    thickness: i32,
    opacity: f32,
    outer: (f32, f32, f32, f32),
    excluded: (f32, f32, f32, f32),
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
            let inside_outer = x as f32 >= outer.0
                && x as f32 <= outer.1
                && y as f32 >= outer.2
                && y as f32 <= outer.3;
            let inside_excluded = x as f32 >= excluded.0
                && x as f32 <= excluded.1
                && y as f32 >= excluded.2
                && y as f32 <= excluded.3;
            if x >= 0
                && y >= 0
                && x < buffer_width
                && y < buffer_height
                && inside_outer
                && !inside_excluded
            {
                blend_pixel(&mut pixels[(y * buffer_width + x) as usize], color, opacity);
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

fn blend_pixel(pixel: &mut slint::Rgb8Pixel, color: Rgba, opacity: f32) {
    let effective_opacity = opacity * color.3 as f32 / 255.0;
    let blend = |source: u8, destination: u8| {
        (source as f32 * effective_opacity + destination as f32 * (1.0 - effective_opacity)).round()
            as u8
    };
    *pixel = slint::Rgb8Pixel::new(
        blend(color.0, pixel.r),
        blend(color.1, pixel.g),
        blend(color.2, pixel.b),
    );
}

#[cfg(test)]
mod tests {
    use super::preview_color;

    #[test]
    fn parses_eight_digit_hex_with_alpha() {
        assert_eq!(preview_color("#12345680", (0, 0, 0)), (18, 52, 86, 128));
        assert_eq!(preview_color("#123456", (0, 0, 0)), (18, 52, 86, 255));
    }
}
