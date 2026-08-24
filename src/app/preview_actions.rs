use std::cell::RefCell;
use std::rc::Rc;

use slint::ComponentHandle;

use crate::domain::frame::{BoardBounds, FrameGeometry};
use crate::domain::settings::Settings;
use crate::preview::data::PreviewData;
use crate::preview::input::{PreviewInput, PreviewTransform};
use crate::preview::viewport::Viewport;
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
