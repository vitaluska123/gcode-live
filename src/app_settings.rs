use crate::{settings::Settings, UiSettings};

/// Populate Slint's settings bridge from the persisted application settings.
pub(crate) fn initialize_ui(ui_settings: UiSettings, settings: &Settings) {
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
    ui_settings.set_show_material(settings.show_material);
    ui_settings.set_show_safe_area(settings.show_safe_area);
    ui_settings.set_show_margin_hatch(settings.show_margin_hatch);
    ui_settings.set_material_color(settings.material_color.clone().into());
    ui_settings.set_safe_area_color(settings.safe_area_color.clone().into());
    ui_settings.set_frame_color(settings.frame_color.clone().into());
    ui_settings
        .set_material_preview_color(preview_color(&settings.material_color, (190, 100, 255)));
    ui_settings
        .set_safe_area_preview_color(preview_color(&settings.safe_area_color, (255, 70, 70)));
    ui_settings.set_frame_preview_color(preview_color(&settings.frame_color, (255, 70, 100)));
}

/// Copy UI values into a new settings value while retaining non-UI fields.
pub(crate) fn read_ui(ui_settings: UiSettings, source: Settings) -> Settings {
    Settings {
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
        show_material: ui_settings.get_show_material(),
        show_safe_area: ui_settings.get_show_safe_area(),
        show_margin_hatch: ui_settings.get_show_margin_hatch(),
        material_color: ui_settings.get_material_color().to_string(),
        safe_area_color: ui_settings.get_safe_area_color().to_string(),
        frame_color: ui_settings.get_frame_color().to_string(),
        ..source
    }
}

fn preview_color(value: &str, fallback: (u8, u8, u8)) -> slint::Color {
    let hex = value.trim().trim_start_matches('#');
    if hex.len() != 6 {
        return slint::Color::from_rgb_u8(fallback.0, fallback.1, fallback.2);
    }
    let parse = |range| u8::from_str_radix(&hex[range], 16).ok();
    match (parse(0..2), parse(2..4), parse(4..6)) {
        (Some(r), Some(g), Some(b)) => slint::Color::from_rgb_u8(r, g, b),
        _ => slint::Color::from_rgb_u8(fallback.0, fallback.1, fallback.2),
    }
}
