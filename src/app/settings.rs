use std::rc::Rc;

use slint::ComponentHandle;

use crate::{
    domain::{frame, settings::Settings},
    export::gcode as export_gcode,
    MainWindow, SettingsWindow, UiSettings,
};

/// Populate Slint's settings bridge from the persisted application settings.
pub(crate) fn initialize_ui(ui_settings: UiSettings, settings: &Settings) {
    ui_settings.set_use_opengl_renderer(settings.use_opengl_renderer);
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
    ui_settings.set_material_offset_x(settings.material_offset_x as f32);
    ui_settings.set_material_offset_y(settings.material_offset_y as f32);
    ui_settings.set_material_edge_margin_x(settings.material_edge_margin_x as f32);
    ui_settings.set_material_edge_margin_y(settings.material_edge_margin_y as f32);
    ui_settings.set_show_grid(settings.show_grid);
    ui_settings.set_show_axes(settings.show_axes);
    ui_settings.set_show_local_axes(settings.show_local_axes);
    ui_settings.set_show_material(settings.show_material);
    ui_settings.set_show_safe_area(settings.show_safe_area);
    ui_settings.set_show_margin_hatch(settings.show_margin_hatch);
    ui_settings.set_material_color(settings.material_color.clone().into());
    ui_settings.set_safe_area_color(settings.safe_area_color.clone().into());
    ui_settings.set_frame_color(settings.frame_color.clone().into());
    ui_settings.set_background_color(settings.background_color.clone().into());
    ui_settings.set_grid_color(settings.grid_color.clone().into());
    ui_settings.set_grid_label_color(settings.grid_label_color.clone().into());
    ui_settings.set_axis_x_color(settings.axis_x_color.clone().into());
    ui_settings.set_axis_y_color(settings.axis_y_color.clone().into());
    ui_settings.set_local_axis_x_color(settings.local_axis_x_color.clone().into());
    ui_settings.set_local_axis_y_color(settings.local_axis_y_color.clone().into());
    ui_settings.set_toolpath_color(settings.toolpath_color.clone().into());
    ui_settings.set_rapid_color(settings.rapid_color.clone().into());
    ui_settings.set_expanded_frame_color(settings.expanded_frame_color.clone().into());
    ui_settings.set_tab_color(settings.tab_color.clone().into());
    ui_settings.set_margin_hatch_color(settings.margin_hatch_color.clone().into());
    ui_settings.set_show_toolpath(settings.show_toolpath);
    ui_settings.set_show_rapid(settings.show_rapid);
    ui_settings.set_show_expanded_frame(settings.show_expanded_frame);
    ui_settings.set_show_frame(settings.show_frame);
    ui_settings.set_show_tabs(settings.show_tabs);
    ui_settings
        .set_material_preview_color(preview_color(&settings.material_color, (190, 100, 255)));
    ui_settings
        .set_safe_area_preview_color(preview_color(&settings.safe_area_color, (255, 70, 70)));
    ui_settings.set_frame_preview_color(preview_color(&settings.frame_color, (255, 70, 100)));
    ui_settings
        .set_background_preview_color(preview_color(&settings.background_color, (14, 17, 22)));
    ui_settings.set_grid_preview_color(preview_color(&settings.grid_color, (42, 48, 58)));
    ui_settings
        .set_grid_label_preview_color(preview_color(&settings.grid_label_color, (150, 160, 175)));
    ui_settings.set_axis_x_preview_color(preview_color(&settings.axis_x_color, (220, 70, 70)));
    ui_settings.set_axis_y_preview_color(preview_color(&settings.axis_y_color, (70, 210, 120)));
    ui_settings
        .set_local_axis_x_preview_color(preview_color(&settings.local_axis_x_color, (220, 70, 70)));
    ui_settings.set_local_axis_y_preview_color(preview_color(
        &settings.local_axis_y_color,
        (70, 210, 120),
    ));
    ui_settings.set_toolpath_preview_color(preview_color(&settings.toolpath_color, (0, 210, 255)));
    ui_settings.set_rapid_preview_color(preview_color(&settings.rapid_color, (255, 190, 0)));
    ui_settings.set_expanded_frame_preview_color(preview_color(
        &settings.expanded_frame_color,
        (255, 210, 0),
    ));
    ui_settings.set_tab_preview_color(preview_color(&settings.tab_color, (255, 220, 70)));
    ui_settings
        .set_margin_hatch_preview_color(preview_color(&settings.margin_hatch_color, (255, 70, 70)));
}

/// Copy UI values into a new settings value while retaining non-UI fields.
pub(crate) fn read_ui(ui_settings: UiSettings, source: Settings) -> Settings {
    Settings {
        use_opengl_renderer: ui_settings.get_use_opengl_renderer(),
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
        material_offset_x: ui_settings.get_material_offset_x() as f64,
        material_offset_y: ui_settings.get_material_offset_y() as f64,
        material_edge_margin_x: ui_settings.get_material_edge_margin_x().max(0.0) as f64,
        material_edge_margin_y: ui_settings.get_material_edge_margin_y().max(0.0) as f64,
        show_grid: ui_settings.get_show_grid(),
        show_axes: ui_settings.get_show_axes(),
        show_local_axes: ui_settings.get_show_local_axes(),
        show_material: ui_settings.get_show_material(),
        show_safe_area: ui_settings.get_show_safe_area(),
        show_margin_hatch: ui_settings.get_show_margin_hatch(),
        material_color: ui_settings.get_material_color().to_string(),
        safe_area_color: ui_settings.get_safe_area_color().to_string(),
        frame_color: ui_settings.get_frame_color().to_string(),
        background_color: ui_settings.get_background_color().to_string(),
        grid_color: ui_settings.get_grid_color().to_string(),
        grid_label_color: ui_settings.get_grid_label_color().to_string(),
        axis_x_color: ui_settings.get_axis_x_color().to_string(),
        axis_y_color: ui_settings.get_axis_y_color().to_string(),
        local_axis_x_color: ui_settings.get_local_axis_x_color().to_string(),
        local_axis_y_color: ui_settings.get_local_axis_y_color().to_string(),
        toolpath_color: ui_settings.get_toolpath_color().to_string(),
        rapid_color: ui_settings.get_rapid_color().to_string(),
        expanded_frame_color: ui_settings.get_expanded_frame_color().to_string(),
        tab_color: ui_settings.get_tab_color().to_string(),
        margin_hatch_color: ui_settings.get_margin_hatch_color().to_string(),
        show_toolpath: ui_settings.get_show_toolpath(),
        show_rapid: ui_settings.get_show_rapid(),
        show_expanded_frame: ui_settings.get_show_expanded_frame(),
        show_frame: ui_settings.get_show_frame(),
        show_tabs: ui_settings.get_show_tabs(),
        ..source
    }
}

pub(crate) fn install_callbacks(main_window: &MainWindow, app_state: &super::state::AppState) {
    let board_bounds = app_state.preview_scene.board_bounds.clone();
    let frame_geometry = app_state.preview_scene.frame_geometry.clone();
    let source_home = app_state.source_home.clone();
    let current_settings = app_state.settings.clone();

    let weak_board = Rc::downgrade(&board_bounds);
    let weak_frame = Rc::downgrade(&frame_geometry);
    let weak_settings = Rc::downgrade(&current_settings);
    let weak_home = Rc::downgrade(&source_home);
    let window_weak = main_window.as_weak();
    main_window.on_sync_settings(move || {
        let (Some(window), Some(settings_rc), Some(board_rc), Some(frame_rc), Some(home_rc)) = (
            window_weak.upgrade(),
            weak_settings.upgrade(),
            weak_board.upgrade(),
            weak_frame.upgrade(),
            weak_home.upgrade(),
        ) else {
            return;
        };
        let new_settings = read_ui(window.global::<UiSettings>(), settings_rc.borrow().clone());
        *settings_rc.borrow_mut() = new_settings.clone();
        let Some(bounds) = board_rc.borrow().clone() else {
            return;
        };
        if let Some(frame) = frame::FrameGeometry::calculate(&bounds, &new_settings) {
            window.set_frame_width(format!("{:.3} mm", frame.width()).into());
            window.set_frame_height(format!("{:.3} mm", frame.height()).into());
            window.set_final_gcode(
                export_gcode::generate_frame_gcode(
                    &bounds,
                    &frame,
                    &new_settings,
                    *home_rc.borrow(),
                )
                .into(),
            );
            *frame_rc.borrow_mut() = Some(frame);
        }
    });

    let weak_settings = Rc::downgrade(&current_settings);
    let window_weak = main_window.as_weak();
    main_window.on_save_settings(move || {
        let (Some(window), Some(settings_rc)) = (window_weak.upgrade(), weak_settings.upgrade())
        else {
            return;
        };
        let new_settings = read_ui(window.global::<UiSettings>(), settings_rc.borrow().clone());
        if let Err(error) = new_settings.save() {
            window.invoke_show_error(format!("Failed to save settings: {error}").into());
            return;
        }
        *settings_rc.borrow_mut() = new_settings.clone();
        if let Some(bounds) = board_bounds.borrow().clone() {
            if let Some(frame) = frame::FrameGeometry::calculate(&bounds, &new_settings) {
                *frame_geometry.borrow_mut() = Some(frame);
            }
        }
        window.invoke_update_preview();
    });
}

/// Connect the standalone settings window to the main UI and application state.
pub(crate) fn install_settings_window_callbacks(
    main_window: &MainWindow,
    settings_window: &SettingsWindow,
    app_state: &super::state::AppState,
) {
    let settings_window_weak = settings_window.as_weak();
    let main_window_weak = main_window.as_weak();
    let settings_weak = Rc::downgrade(&app_state.settings);
    settings_window.on_preview_changed(move || {
        let (Some(settings_window), Some(main_window), Some(settings)) = (
            settings_window_weak.upgrade(),
            main_window_weak.upgrade(),
            settings_weak.upgrade(),
        ) else {
            return;
        };

        let new_settings = read_ui(
            settings_window.global::<UiSettings>(),
            settings.borrow().clone(),
        );
        *settings.borrow_mut() = new_settings.clone();
        initialize_ui(settings_window.global::<UiSettings>(), &new_settings);
        initialize_ui(main_window.global::<UiSettings>(), &new_settings);
        main_window.invoke_update_preview();
    });

    let main_window_weak = main_window.as_weak();
    settings_window.on_save_settings(move || {
        let Some(main_window) = main_window_weak.upgrade() else {
            return;
        };
        main_window.invoke_save_settings();
    });

    let settings_window = settings_window.clone_strong();
    let settings_weak = Rc::downgrade(&app_state.settings);
    let main_window_weak = main_window.as_weak();
    main_window.on_open_settings(move || {
        let (Some(settings), Some(main_window)) =
            (settings_weak.upgrade(), main_window_weak.upgrade())
        else {
            return;
        };
        initialize_ui(settings_window.global::<UiSettings>(), &settings.borrow());
        if let Err(error) = settings_window.show() {
            main_window
                .invoke_show_error(format!("Failed to open settings window: {error}").into());
        }
    });
}

fn preview_color(value: &str, fallback: (u8, u8, u8)) -> slint::Color {
    let hex = value.trim().trim_start_matches('#');
    if hex.len() != 6 && hex.len() != 8 {
        return slint::Color::from_rgb_u8(fallback.0, fallback.1, fallback.2);
    }
    let parse = |range| u8::from_str_radix(&hex[range], 16).ok();
    match (parse(0..2), parse(2..4), parse(4..6)) {
        (Some(r), Some(g), Some(b)) => {
            let alpha = if hex.len() == 8 {
                parse(6..8).unwrap_or(255)
            } else {
                255
            };
            slint::Color::from_argb_u8(alpha, r, g, b)
        }
        _ => slint::Color::from_rgb_u8(fallback.0, fallback.1, fallback.2),
    }
}
