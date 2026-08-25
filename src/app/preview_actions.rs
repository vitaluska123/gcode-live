use std::cell::RefCell;
use std::rc::Rc;

use slint::ComponentHandle;

use crate::domain::frame;
use crate::domain::frame::{BoardBounds, FrameGeometry};
use crate::domain::settings::Settings;
use crate::preview::data::PreviewData;
use crate::preview::input::{PreviewInput, PreviewTransform};
use crate::preview::viewport::Viewport;
use crate::preview::{data as preview, input as preview_input, renderer as preview_renderer};
use crate::MainWindow;

/// Connect pointer panning to the viewport without exposing input details to
/// the application composition module.
pub(crate) fn install_pan_callbacks(
    main_window: &MainWindow,
    preview_input: Rc<RefCell<PreviewInput>>,
    viewport: Rc<RefCell<Viewport>>,
) {
    let input_state = preview_input.clone();
    main_window.on_begin_pan(move |x, y| {
        input_state.borrow_mut().begin_pan(x as f64, y as f64);
    });

    let window_weak = main_window.as_weak();
    main_window.on_pan_preview(move |x, y| {
        preview_input
            .borrow_mut()
            .pan_to(&mut viewport.borrow_mut(), x as f64, y as f64);
        if let Some(window) = window_weak.upgrade() {
            window.invoke_update_preview();
            window.window().request_redraw();
        }
    });
}

pub(crate) fn cursor_text(
    bounds: &BoardBounds,
    frame: &FrameGeometry,
    settings: &Settings,
    viewport: Viewport,
    screen: (f32, f32),
    size: (f32, f32),
) -> Option<String> {
    let (shift_x, shift_y) = settings.local_offset();
    let expanded = FrameGeometry::expanded(bounds, settings);
    let left = expanded
        .as_ref()
        .map_or(frame.left, |value| frame.left.min(value.left));
    let right = expanded
        .as_ref()
        .map_or(frame.right, |value| frame.right.max(value.right));
    let bottom = expanded
        .as_ref()
        .map_or(frame.bottom, |value| frame.bottom.min(value.bottom));
    let top = expanded
        .as_ref()
        .map_or(frame.top, |value| frame.top.max(value.top));
    let data = PreviewData::from_bounds_with_material(
        bounds.x_min + shift_x,
        bounds.x_max + shift_x,
        bounds.y_min + shift_y,
        bounds.y_max + shift_y,
        left + shift_x,
        right + shift_x,
        bottom + shift_y,
        top + shift_y,
        settings.material_width,
        settings.material_height,
        settings.material_offset_x,
        settings.material_offset_y,
    );
    let transform = PreviewTransform::new(data, size.0, size.1)?;
    let (world_x, world_y) =
        transform.screen_to_world(viewport, screen.0 as f64, screen.1 as f64)?;
    Some(if settings.local_offset_enabled {
        format!(
            "Локальные: X: {:.3}   Y: {:.3}\nГлобальные: X: {world_x:.3}   Y: {world_y:.3}",
            world_x - settings.local_offset_x,
            world_y - settings.local_offset_y
        )
    } else {
        format!("Глобальные: X: {world_x:.3}   Y: {world_y:.3}")
    })
}

pub(crate) fn install_callbacks(main_window: &MainWindow, app_state: &crate::app::state::AppState) {
    let preview_scene = app_state.preview_scene.clone();
    let board_bounds = preview_scene.board_bounds.clone();
    let frame_geometry = preview_scene.frame_geometry.clone();
    let viewport = app_state.viewport.clone();
    let preview_input = app_state.preview_input.clone();
    let current_settings = app_state.settings.clone();

    let renderer = Rc::new(RefCell::new(
        preview_renderer::PreviewRendererBackend::from_settings(
            current_settings.borrow().use_opengl_renderer,
        ),
    ));
    let notifier_renderer = renderer.clone();
    let notifier_window = main_window.as_weak();
    let _ = main_window
        .window()
        .set_rendering_notifier(move |state, api| {
            if let Some(image) = notifier_renderer.borrow_mut().notify(state, api) {
                if let Some(window) = notifier_window.upgrade() {
                    window.set_preview_image(image);
                }
            }
        });
    let weak_scene = Rc::downgrade(&preview_scene);
    let weak_viewport = Rc::downgrade(&viewport);
    let weak_settings = Rc::downgrade(&current_settings);
    main_window.on_render_preview(move |width, height| {
        let width = width.max(1.0).round() as u32;
        let height = height.max(1.0).round() as u32;
        let (Some(scene), Some(settings), Some(viewport)) = (
            weak_scene.upgrade(),
            weak_settings.upgrade(),
            weak_viewport.upgrade(),
        ) else {
            return slint::Image::from_rgb8(slint::SharedPixelBuffer::new(width, height));
        };
        let frame = preview_renderer::RenderFrame {
            width,
            height,
            scene: scene.snapshot(),
            settings: settings.borrow().clone(),
            viewport: *viewport.borrow(),
        };
        let image = renderer.borrow_mut().render(&frame);
        image
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
            settings.material_offset_x,
            settings.material_offset_y,
        );
        let mut camera = *viewport_state.borrow();
        let Some(transform) = preview_input::PreviewTransform::new(data, width, height) else {
            return;
        };
        transform.zoom_at(&mut camera, direction, mouse_x as f64, mouse_y as f64);
        *viewport_state.borrow_mut() = camera;
        window.invoke_update_preview();
        window.window().request_redraw();
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
            settings.material_offset_x,
            settings.material_offset_y,
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
        window.window().request_redraw();
    });
    install_pan_callbacks(main_window, preview_input.clone(), viewport.clone());
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
        let camera = *viewport.borrow();
        if let Some(text) = cursor_text(&bounds, &frame, &settings, camera, (x, y), (width, height))
        {
            window.set_cursor_coordinates(text.into());
        }
    });
}
