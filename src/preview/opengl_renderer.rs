//! OpenGL-compositor preview backend.
//!
//! Slint owns the native surface and OpenGL context. This module therefore
//! only creates objects while called from Slint's rendering notifier; it never
//! creates a second winit window or a competing GL context.

use std::ffi::CStr;

use glow::HasContext;
use slint::{GraphicsAPI, RenderingState};

use crate::domain::frame;
use crate::preview::data::PreviewData;
use crate::preview::renderer::{PreviewRenderer, RenderFrame};
use crate::preview::software_renderer::SoftwarePreviewRenderer;

/// GPU compositor state. The CPU renderer is used only after an OpenGL setup
/// failure; it is never used to compose a normal OpenGL frame.
#[derive(Default)]
pub struct OpenGlPreviewRenderer {
    software_fallback: SoftwarePreviewRenderer,
    pending: Option<RenderFrame>,
    texture_image: Option<slint::Image>,
    gpu: Option<GpuTarget>,
    unavailable: bool,
}

impl PreviewRenderer for OpenGlPreviewRenderer {
    fn render(&mut self, frame: &RenderFrame) -> slint::Image {
        self.pending = Some(RenderFrame {
            width: frame.width,
            height: frame.height,
            scene: frame.scene.clone(),
            settings: frame.settings.clone(),
            viewport: frame.viewport,
        });
        if self.unavailable {
            return self.software_fallback.render(frame);
        }
        self.texture_image.clone().unwrap_or_else(|| {
            slint::Image::from_rgb8(slint::SharedPixelBuffer::new(frame.width, frame.height))
        })
    }
    /// Receives Slint's current OpenGL context. `Some(image)` must be assigned
    /// to the preview property before Slint renders its image item.
    fn notify(&mut self, state: RenderingState, api: &GraphicsAPI<'_>) -> Option<slint::Image> {
        if self.unavailable {
            return None;
        }
        match state {
            RenderingState::RenderingSetup => {
                let GraphicsAPI::NativeOpenGL { get_proc_address } = api else {
                    self.unavailable = true;
                    return None;
                };
                // SAFETY: Slint documents that this callback runs with its
                // native OpenGL context current.
                let gl = unsafe {
                    glow::Context::from_loader_function(|name| {
                        let mut bytes = name.as_bytes().to_vec();
                        bytes.push(0);
                        CStr::from_bytes_with_nul(&bytes)
                            .map_or(core::ptr::null(), get_proc_address)
                    })
                };
                self.gpu = GpuTarget::new(gl).ok();
                self.unavailable = self.gpu.is_none();
                None
            }
            RenderingState::BeforeRendering => {
                let (Some(target), Some(frame)) = (self.gpu.as_mut(), self.pending.as_ref()) else {
                    return None;
                };
                match target.render(frame) {
                    Ok(image) => {
                        self.texture_image = Some(image.clone());
                        Some(image)
                    }
                    Err(()) => {
                        self.unavailable = true;
                        None
                    }
                }
            }
            RenderingState::RenderingTeardown => {
                self.gpu = None;
                self.texture_image = None;
                None
            }
            _ => None,
        }
    }
}

struct GpuTarget {
    gl: glow::Context,
    program: glow::NativeProgram,
    vertices: glow::NativeBuffer,
    texture: glow::NativeTexture,
    framebuffer: glow::NativeFramebuffer,
    size: (u32, u32),
}

impl GpuTarget {
    fn new(gl: glow::Context) -> Result<Self, ()> {
        // SAFETY: called by the notifier with Slint's context current.
        unsafe {
            let program = create_program(&gl)?;
            Ok(Self {
                texture: gl.create_texture().map_err(|_| ())?,
                framebuffer: gl.create_framebuffer().map_err(|_| ())?,
                vertices: gl.create_buffer().map_err(|_| ())?,
                program,
                gl,
                size: (0, 0),
            })
        }
    }

    fn render(&mut self, frame: &RenderFrame) -> Result<slint::Image, ()> {
        self.resize(frame.width, frame.height)?;
        let prepared = PreparedGeometry::new(frame);
        // SAFETY: the framebuffer and texture were created in the current
        // Slint context. Every state value changed below is restored before
        // returning to FemtoVG.
        unsafe {
            let previous = OpenGlState::capture(&self.gl);
            self.gl
                .bind_framebuffer(glow::FRAMEBUFFER, Some(self.framebuffer));
            self.gl
                .viewport(0, 0, frame.width as i32, frame.height as i32);
            self.gl.clear_color(
                prepared.background[0],
                prepared.background[1],
                prepared.background[2],
                prepared.background[3],
            );
            self.gl.clear(glow::COLOR_BUFFER_BIT);
            self.gl.enable(glow::BLEND);
            self.gl
                .blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
            self.gl.use_program(Some(self.program));
            self.gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vertices));
            self.gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                f32_as_bytes(&prepared.vertices),
                glow::DYNAMIC_DRAW,
            );
            let stride = (6 * std::mem::size_of::<f32>()) as i32;
            self.gl.enable_vertex_attrib_array(0);
            self.gl
                .vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, stride, 0);
            self.gl.enable_vertex_attrib_array(1);
            self.gl.vertex_attrib_pointer_f32(
                1,
                4,
                glow::FLOAT,
                false,
                stride,
                2 * std::mem::size_of::<f32>() as i32,
            );
            if let Some(view_size) = self.gl.get_uniform_location(self.program, "u_view_size") {
                self.gl
                    .uniform_2_f32(Some(&view_size), frame.width as f32, frame.height as f32);
            }
            self.gl
                .draw_arrays(glow::LINES, 0, (prepared.vertices.len() / 6) as i32);
            self.gl.disable_vertex_attrib_array(0);
            self.gl.disable_vertex_attrib_array(1);
            self.gl.bind_buffer(glow::ARRAY_BUFFER, None);
            self.gl.use_program(None);
            previous.restore(&self.gl);
            // The texture belongs to this context and lives until teardown.
            Ok(slint::BorrowedOpenGLTextureBuilder::new_gl_2d_rgba_texture(
                self.texture.0,
                euclid::default::Size2D::new(frame.width, frame.height),
            )
            .origin(slint::BorrowedOpenGLTextureOrigin::BottomLeft)
            .build())
        }
    }

    fn resize(&mut self, width: u32, height: u32) -> Result<(), ()> {
        if self.size == (width, height) {
            return Ok(());
        }
        // SAFETY: resource allocation is performed with the current context.
        unsafe {
            let previous = OpenGlState::capture(&self.gl);
            self.gl.bind_texture(glow::TEXTURE_2D, Some(self.texture));
            self.gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::LINEAR as i32,
            );
            self.gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::LINEAR as i32,
            );
            self.gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA as i32,
                width as i32,
                height as i32,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(None),
            );
            self.gl
                .bind_framebuffer(glow::FRAMEBUFFER, Some(self.framebuffer));
            self.gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(self.texture),
                0,
            );
            if self.gl.check_framebuffer_status(glow::FRAMEBUFFER) != glow::FRAMEBUFFER_COMPLETE {
                return Err(());
            }
            previous.restore(&self.gl);
        }
        self.size = (width, height);
        Ok(())
    }
}

struct OpenGlState {
    framebuffer: Option<glow::NativeFramebuffer>,
    program: Option<glow::NativeProgram>,
    array_buffer: Option<glow::NativeBuffer>,
    texture_2d: Option<glow::NativeTexture>,
    blend_enabled: bool,
}

impl OpenGlState {
    unsafe fn capture(gl: &glow::Context) -> Self {
        Self {
            framebuffer: framebuffer_handle(gl.get_parameter_i32(glow::FRAMEBUFFER_BINDING)),
            program: program_handle(gl.get_parameter_i32(glow::CURRENT_PROGRAM)),
            array_buffer: buffer_handle(gl.get_parameter_i32(glow::ARRAY_BUFFER_BINDING)),
            texture_2d: texture_handle(gl.get_parameter_i32(glow::TEXTURE_BINDING_2D)),
            blend_enabled: gl.is_enabled(glow::BLEND),
        }
    }

    unsafe fn restore(&self, gl: &glow::Context) {
        gl.bind_framebuffer(glow::FRAMEBUFFER, self.framebuffer);
        gl.use_program(self.program);
        gl.bind_buffer(glow::ARRAY_BUFFER, self.array_buffer);
        gl.bind_texture(glow::TEXTURE_2D, self.texture_2d);
        if self.blend_enabled {
            gl.enable(glow::BLEND);
        } else {
            gl.disable(glow::BLEND);
        }
    }
}

fn native_id(value: i32) -> Option<std::num::NonZeroU32> {
    std::num::NonZeroU32::new(value.max(0) as u32)
}
fn framebuffer_handle(value: i32) -> Option<glow::NativeFramebuffer> {
    native_id(value).map(glow::NativeFramebuffer)
}
fn program_handle(value: i32) -> Option<glow::NativeProgram> {
    native_id(value).map(glow::NativeProgram)
}
fn buffer_handle(value: i32) -> Option<glow::NativeBuffer> {
    native_id(value).map(glow::NativeBuffer)
}
fn texture_handle(value: i32) -> Option<glow::NativeTexture> {
    native_id(value).map(glow::NativeTexture)
}

fn create_program(gl: &glow::Context) -> Result<glow::NativeProgram, ()> {
    const VERTEX: &str = "attribute vec2 a_position; attribute vec4 a_color; uniform vec2 u_view_size; varying vec4 v_color; void main() { vec2 clip = vec2(a_position.x / u_view_size.x * 2.0 - 1.0, 1.0 - a_position.y / u_view_size.y * 2.0); gl_Position = vec4(clip, 0.0, 1.0); v_color = a_color; }";
    const FRAGMENT: &str =
        "precision mediump float; varying vec4 v_color; void main() { gl_FragColor = v_color; }";
    // SAFETY: the program is created while Slint's OpenGL context is current.
    unsafe {
        let vertex = compile_shader(gl, glow::VERTEX_SHADER, VERTEX)?;
        let fragment = compile_shader(gl, glow::FRAGMENT_SHADER, FRAGMENT)?;
        let program = gl.create_program().map_err(|_| ())?;
        gl.attach_shader(program, vertex);
        gl.attach_shader(program, fragment);
        gl.bind_attrib_location(program, 0, "a_position");
        gl.bind_attrib_location(program, 1, "a_color");
        gl.link_program(program);
        let linked = gl.get_program_link_status(program);
        gl.delete_shader(vertex);
        gl.delete_shader(fragment);
        if linked {
            Ok(program)
        } else {
            gl.delete_program(program);
            Err(())
        }
    }
}

unsafe fn compile_shader(
    gl: &glow::Context,
    kind: u32,
    source: &str,
) -> Result<glow::NativeShader, ()> {
    let shader = gl.create_shader(kind).map_err(|_| ())?;
    gl.shader_source(shader, source);
    gl.compile_shader(shader);
    if gl.get_shader_compile_status(shader) {
        Ok(shader)
    } else {
        gl.delete_shader(shader);
        Err(())
    }
}

fn f32_as_bytes(values: &[f32]) -> &[u8] {
    // SAFETY: `f32` has no padding and the output lifetime is that of values.
    unsafe {
        std::slice::from_raw_parts(values.as_ptr().cast::<u8>(), std::mem::size_of_val(values))
    }
}

struct PreparedGeometry {
    background: [f32; 4],
    vertices: Vec<f32>,
}

impl PreparedGeometry {
    fn new(frame: &RenderFrame) -> Self {
        let background = color_f32(&frame.settings.background_color, [14, 17, 22, 255]);
        let (Some(bounds), Some(frame_geometry)) = (
            frame.scene.board_bounds.as_ref(),
            frame.scene.frame_geometry.as_ref(),
        ) else {
            return Self {
                background,
                vertices: Vec::new(),
            };
        };
        let settings = &frame.settings;
        let (shift_x, shift_y) = settings.local_offset();
        let data = PreviewData::from_bounds_with_material(
            bounds.x_min + shift_x,
            bounds.x_max + shift_x,
            bounds.y_min + shift_y,
            bounds.y_max + shift_y,
            frame_geometry.left + shift_x,
            frame_geometry.right + shift_x,
            frame_geometry.bottom + shift_y,
            frame_geometry.top + shift_y,
            settings.material_width,
            settings.material_height,
            settings.material_offset_x,
            settings.material_offset_y,
        );
        let scale =
            data.calculate_scale(frame.width as f32, frame.height as f32) * frame.viewport.zoom;
        let map = |point: (f64, f64)| {
            let (x, y) = data.world_to_screen(
                point.0,
                point.1,
                scale,
                frame.width as f32,
                frame.height as f32,
            );
            (
                x + frame.viewport.pan_x as f32,
                y + frame.viewport.pan_y as f32,
            )
        };
        let mut vertices = Vec::new();
        if settings.show_grid {
            add_grid(
                &mut vertices,
                &data,
                scale,
                frame.width,
                frame.height,
                frame.viewport.pan_x,
                frame.viewport.pan_y,
                color_f32(&settings.grid_color, [42, 48, 58, 255]),
                &map,
            );
        }
        if settings.show_axes {
            add_axes(
                &mut vertices,
                color_f32(&settings.axis_x_color, [220, 70, 70, 255]),
                color_f32(&settings.axis_y_color, [70, 210, 120, 255]),
                &map,
                0.0,
                0.0,
            );
        }
        if settings.show_local_axes && settings.local_offset_enabled {
            add_axes(
                &mut vertices,
                color_f32(&settings.local_axis_x_color, [220, 70, 70, 128]),
                color_f32(&settings.local_axis_y_color, [70, 210, 120, 128]),
                &map,
                shift_x,
                shift_y,
            );
        }
        if settings.show_toolpath {
            add_path(
                &mut vertices,
                &frame.scene.toolpath,
                shift_x,
                shift_y,
                color_f32(&settings.toolpath_color, [0, 210, 255, 255]),
                &map,
            );
        }
        let material = material_rect(settings);
        if settings.show_material {
            add_dashed_path(
                &mut vertices,
                &material,
                0.0,
                0.0,
                color_f32(&settings.material_color, [190, 100, 255, 255]),
                &map,
            );
        }
        let safe = safe_rect(settings);
        if settings.show_margin_hatch {
            add_hatch(
                &mut vertices,
                &material,
                &safe,
                with_opacity(
                    color_f32(&settings.margin_hatch_color, [255, 70, 70, 255]),
                    0.25,
                ),
                &map,
            );
        }
        if settings.show_safe_area {
            add_dashed_path(
                &mut vertices,
                &safe,
                0.0,
                0.0,
                color_f32(&settings.safe_area_color, [255, 70, 70, 255]),
                &map,
            );
        }
        if settings.show_expanded_frame {
            if let Some(expanded) = frame::FrameGeometry::expanded(bounds, settings) {
                let r = [
                    (expanded.left + shift_x, expanded.bottom + shift_y),
                    (expanded.right + shift_x, expanded.bottom + shift_y),
                    (expanded.right + shift_x, expanded.top + shift_y),
                    (expanded.left + shift_x, expanded.top + shift_y),
                    (expanded.left + shift_x, expanded.bottom + shift_y),
                ];
                add_dashed_path(
                    &mut vertices,
                    &r,
                    0.,
                    0.,
                    color_f32(&settings.expanded_frame_color, [255, 210, 0, 255]),
                    &map,
                );
            }
        }
        let outline = [
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
            add_path(
                &mut vertices,
                &outline,
                0.,
                0.,
                color_f32(&settings.frame_color, [255, 70, 100, 255]),
                &map,
            );
        }
        if settings.show_tabs {
            let radius = (settings.tool_diameter / 2.0)
                .min(1.0)
                .min(frame_geometry.width() / 4.0)
                .min(frame_geometry.height() / 4.0)
                .max(0.0);
            for (left, right) in frame::top_tab_intervals(frame_geometry, radius, settings) {
                add_segment(
                    &mut vertices,
                    map((left + shift_x, frame_geometry.top + shift_y)),
                    map((right + shift_x, frame_geometry.top + shift_y)),
                    color_f32(&settings.tab_color, [255, 220, 70, 255]),
                );
            }
        }
        if settings.show_rapid {
            add_path(
                &mut vertices,
                &frame.scene.rapid_path,
                shift_x,
                shift_y,
                color_f32(&settings.rapid_color, [255, 190, 0, 255]),
                &map,
            );
        }
        Self {
            background,
            vertices,
        }
    }
}

fn add_segment(out: &mut Vec<f32>, a: (f32, f32), b: (f32, f32), c: [f32; 4]) {
    out.extend_from_slice(&[
        a.0, a.1, c[0], c[1], c[2], c[3], b.0, b.1, c[0], c[1], c[2], c[3],
    ]);
}
fn add_path(
    out: &mut Vec<f32>,
    points: &[(f64, f64)],
    dx: f64,
    dy: f64,
    c: [f32; 4],
    map: &impl Fn((f64, f64)) -> (f32, f32),
) {
    for pair in points.windows(2) {
        add_segment(
            out,
            map((pair[0].0 + dx, pair[0].1 + dy)),
            map((pair[1].0 + dx, pair[1].1 + dy)),
            c,
        );
    }
}

/// Add a stable screen-space dash pattern. The line endpoints are still
/// derived from world geometry; only dash subdivision depends on the camera.
fn add_dashed_path(
    out: &mut Vec<f32>,
    points: &[(f64, f64)],
    dx: f64,
    dy: f64,
    c: [f32; 4],
    map: &impl Fn((f64, f64)) -> (f32, f32),
) {
    const DASH_PIXELS: f32 = 6.0;
    const GAP_PIXELS: f32 = 6.0;
    for pair in points.windows(2) {
        let start = map((pair[0].0 + dx, pair[0].1 + dy));
        let end = map((pair[1].0 + dx, pair[1].1 + dy));
        let delta = (end.0 - start.0, end.1 - start.1);
        let length = (delta.0 * delta.0 + delta.1 * delta.1).sqrt();
        if length <= f32::EPSILON {
            continue;
        }
        let direction = (delta.0 / length, delta.1 / length);
        let mut offset = 0.0;
        while offset < length {
            let dash_end = (offset + DASH_PIXELS).min(length);
            add_segment(
                out,
                (
                    start.0 + direction.0 * offset,
                    start.1 + direction.1 * offset,
                ),
                (
                    start.0 + direction.0 * dash_end,
                    start.1 + direction.1 * dash_end,
                ),
                c,
            );
            offset += DASH_PIXELS + GAP_PIXELS;
        }
    }
}
fn add_axes(
    out: &mut Vec<f32>,
    x: [f32; 4],
    y: [f32; 4],
    map: &impl Fn((f64, f64)) -> (f32, f32),
    ox: f64,
    oy: f64,
) {
    add_segment(out, map((-100000., oy)), map((100000., oy)), x);
    add_segment(out, map((ox, -100000.)), map((ox, 100000.)), y);
}
#[allow(clippy::too_many_arguments)] // Grid conversion needs viewport and camera inputs.
fn add_grid(
    out: &mut Vec<f32>,
    data: &PreviewData,
    scale: f64,
    width: u32,
    height: u32,
    pan_x: f64,
    pan_y: f64,
    c: [f32; 4],
    map: &impl Fn((f64, f64)) -> (f32, f32),
) {
    if scale <= 0. {
        return;
    };
    let (min_x, max_x, min_y, max_y) = data.world_bounds();
    let base_x = (width as f64 - (max_x - min_x) * scale) / 2.0 + pan_x;
    let base_y = (height as f64 + (max_y - min_y) * scale) / 2.0 + pan_y;
    let step = nice_grid_step(80.0 / scale);
    let first_x = ((min_x - base_x / scale) / step).floor() as i64;
    let last_x = ((min_x + (width as f64 - base_x) / scale) / step).ceil() as i64;
    for i in first_x..=last_x {
        let x = i as f64 * step;
        add_segment(out, map((x, min_y - 100000.)), map((x, max_y + 100000.)), c);
        add_grid_label(
            out,
            (
                base_x as f32 + (x - min_x) as f32 * scale as f32 + 3.0,
                height as f32 - 11.0,
            ),
            x,
            c,
        );
    }
    let first_y = ((min_y + (base_y - height as f64) / scale) / step).floor() as i64;
    let last_y = ((min_y + base_y / scale) / step).ceil() as i64;
    for i in first_y..=last_y {
        let y = i as f64 * step;
        add_segment(out, map((min_x - 100000., y)), map((max_x + 100000., y)), c);
        add_grid_label(
            out,
            (3.0, base_y as f32 - (y - min_y) as f32 * scale as f32 - 9.0),
            y,
            c,
        );
    }
}

fn add_grid_label(out: &mut Vec<f32>, origin: (f32, f32), value: f64, color: [f32; 4]) {
    let text = if value.abs() < 0.0001 {
        "0".to_owned()
    } else if value.abs() < 1.0 {
        format!("{value:.1}")
    } else {
        format!("{value:.0}")
    };
    for (index, character) in text.chars().enumerate() {
        add_glyph(
            out,
            (origin.0 + index as f32 * 4.0, origin.1),
            character,
            color,
        );
    }
}

fn add_glyph(out: &mut Vec<f32>, origin: (f32, f32), character: char, color: [f32; 4]) {
    let glyph = match character {
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
    for (row, bits) in glyph.into_iter().enumerate() {
        for column in 0..3 {
            if bits & (1 << (2 - column)) != 0 {
                let x = origin.0 + column as f32;
                let y = origin.1 + row as f32;
                add_segment(out, (x, y), (x + 1.0, y), color);
            }
        }
    }
}
fn material_rect(s: &crate::domain::settings::Settings) -> [(f64, f64); 5] {
    let l = s.material_offset_x - s.material_width;
    let r = s.material_offset_x;
    let b = s.material_offset_y;
    let t = b + s.material_height;
    [(l, b), (r, b), (r, t), (l, t), (l, b)]
}
fn safe_rect(s: &crate::domain::settings::Settings) -> [(f64, f64); 5] {
    let l = s.material_offset_x - s.material_width + s.material_edge_margin_x.max(0.);
    let r = s.material_offset_x - s.material_edge_margin_x.max(0.);
    let b = s.material_offset_y + s.material_edge_margin_y.max(0.);
    let t = s.material_offset_y + s.material_height - s.material_edge_margin_y.max(0.);
    [(l, b), (r, b), (r, t), (l, t), (l, b)]
}
fn add_hatch(
    out: &mut Vec<f32>,
    outer: &[(f64, f64); 5],
    inner: &[(f64, f64); 5],
    c: [f32; 4],
    map: &impl Fn((f64, f64)) -> (f32, f32),
) {
    let l = outer[0].0;
    let r = outer[1].0;
    let b = outer[0].1;
    let t = outer[2].1;
    let il = inner[0].0;
    let ir = inner[1].0;
    let ib = inner[0].1;
    let it = inner[2].1;
    add_hatch_band(out, (l, il, b, t), c, map);
    add_hatch_band(out, (ir, r, b, t), c, map);
    add_hatch_band(out, (il, ir, b, ib), c, map);
    add_hatch_band(out, (il, ir, it, t), c, map);
}

const HATCH_STROKES_PER_BAND: usize = 96;
const HATCH_THICKNESS_PX: f32 = 2.0;

fn add_hatch_band(
    out: &mut Vec<f32>,
    bounds: (f64, f64, f64, f64),
    color: [f32; 4],
    map: &impl Fn((f64, f64)) -> (f32, f32),
) {
    let (left, right, bottom, top) = bounds;
    if left >= right || bottom >= top {
        return;
    }
    let start = bottom - right;
    let end = top - left;
    for index in 0..HATCH_STROKES_PER_BAND {
        let fraction = index as f64 / (HATCH_STROKES_PER_BAND - 1) as f64;
        let intercept = start + (end - start) * fraction;
        let x0 = left.max(bottom - intercept);
        let x1 = right.min(top - intercept);
        if x0 < x1 {
            let start = map((x0, x0 + intercept));
            let end = map((x1, x1 + intercept));
            add_thick_segment(out, start, end, color, HATCH_THICKNESS_PX);
        }
    }
}

fn add_thick_segment(
    out: &mut Vec<f32>,
    start: (f32, f32),
    end: (f32, f32),
    color: [f32; 4],
    thickness: f32,
) {
    let delta = (end.0 - start.0, end.1 - start.1);
    let length = (delta.0 * delta.0 + delta.1 * delta.1).sqrt();
    if length <= f32::EPSILON {
        return;
    }
    let offset = (
        -delta.1 / length * thickness / 2.0,
        delta.0 / length * thickness / 2.0,
    );
    add_segment(
        out,
        (start.0 - offset.0, start.1 - offset.1),
        (end.0 - offset.0, end.1 - offset.1),
        color,
    );
    add_segment(
        out,
        (start.0 + offset.0, start.1 + offset.1),
        (end.0 + offset.0, end.1 + offset.1),
        color,
    );
}
fn nice_grid_step(target: f64) -> f64 {
    let base = 10_f64.powf(target.max(f64::MIN_POSITIVE).log10().floor());
    [1., 2., 5., 10.]
        .into_iter()
        .map(|m| m * base)
        .find(|s| *s >= target)
        .unwrap_or(base * 10.)
}
fn color_f32(value: &str, fallback: [u8; 4]) -> [f32; 4] {
    let c = parse_color(value, fallback);
    [
        c[0] as f32 / 255.,
        c[1] as f32 / 255.,
        c[2] as f32 / 255.,
        c[3] as f32 / 255.,
    ]
}

fn with_opacity(mut color: [f32; 4], opacity: f32) -> [f32; 4] {
    color[3] *= opacity;
    color
}

fn parse_color(value: &str, fallback: [u8; 4]) -> [u8; 4] {
    let hex = value.trim().trim_start_matches('#');
    if hex.len() != 6 && hex.len() != 8 {
        return fallback;
    }
    let byte = |start| u8::from_str_radix(&hex[start..start + 2], 16).ok();
    match (byte(0), byte(2), byte(4)) {
        (Some(red), Some(green), Some(blue)) => [
            red,
            green,
            blue,
            if hex.len() == 8 {
                byte(6).unwrap_or(255)
            } else {
                255
            },
        ],
        _ => fallback,
    }
}

#[cfg(test)]
mod tests {
    use super::parse_color;

    #[test]
    fn parses_rgb_and_rgba_colors() {
        assert_eq!(parse_color("#123456", [0; 4]), [18, 52, 86, 255]);
        assert_eq!(parse_color("#12345680", [0; 4]), [18, 52, 86, 128]);
    }
}
