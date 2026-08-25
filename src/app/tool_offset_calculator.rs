use std::rc::Rc;

use slint::ComponentHandle;

use crate::{
    domain::tool_offset_calculator::{ToolOffsetInputs, ToolOffsetResults},
    MainWindow, ToolOffsetCalculatorWindow, UiSettings,
};

use super::settings::{initialize_ui, read_ui};

pub(crate) fn install_callbacks(
    main_window: &MainWindow,
    calculator_window: &ToolOffsetCalculatorWindow,
    app_state: &super::state::AppState,
) {
    let calculator_window_weak = calculator_window.as_weak();
    calculator_window.on_recalculate(move || {
        let Some(calculator_window) = calculator_window_weak.upgrade() else {
            return;
        };
        update_results(&calculator_window);
    });

    let calculator_window_weak = calculator_window.as_weak();
    let main_window_weak = main_window.as_weak();
    let settings_weak = Rc::downgrade(&app_state.settings);
    calculator_window.on_apply_results(move || {
        let (Some(calculator_window), Some(main_window), Some(settings)) = (
            calculator_window_weak.upgrade(),
            main_window_weak.upgrade(),
            settings_weak.upgrade(),
        ) else {
            return;
        };
        let results = inputs_from(&calculator_window).calculate();
        let ui_settings = main_window.global::<UiSettings>();
        apply_results(ui_settings, results);
        let new_settings = read_ui(
            main_window.global::<UiSettings>(),
            settings.borrow().clone(),
        );
        *settings.borrow_mut() = new_settings.clone();
        initialize_ui(main_window.global::<UiSettings>(), &new_settings);
        main_window.invoke_update_preview();
    });

    let calculator_window = calculator_window.clone_strong();
    let main_window_weak = main_window.as_weak();
    main_window.on_open_offset_calculator(move || {
        let Some(main_window) = main_window_weak.upgrade() else {
            return;
        };
        update_results(&calculator_window);
        if let Err(error) = calculator_window.show() {
            main_window
                .invoke_show_error(format!("Failed to open offset calculator: {error}").into());
        }
    });
}

fn inputs_from(window: &ToolOffsetCalculatorWindow) -> ToolOffsetInputs {
    ToolOffsetInputs {
        indicator_x: parse_input(window.get_indicator_x().as_str()),
        indicator_y: parse_input(window.get_indicator_y().as_str()),
        clamp_x: parse_input(window.get_clamp_x().as_str()),
        clamp_y: parse_input(window.get_clamp_y().as_str()),
        safety_x: parse_input(window.get_safety_x().as_str()),
        safety_y: parse_input(window.get_safety_y().as_str()),
    }
}

fn update_results(window: &ToolOffsetCalculatorWindow) {
    let results = inputs_from(window).calculate();
    window.set_program_zero_x(format_value(results.program_zero_x).into());
    window.set_program_zero_y(format_value(results.program_zero_y).into());
    window.set_program_home_x(format_value(results.program_home_x).into());
    window.set_program_home_y(format_value(results.program_home_y).into());
    window.set_material_offset_x(format_value(results.material_offset_x).into());
    window.set_material_offset_y(format_value(results.material_offset_y).into());
    window.set_material_edge_margin_x(format_value(results.material_edge_margin_x).into());
    window.set_material_edge_margin_y(format_value(results.material_edge_margin_y).into());
}

fn apply_results(ui_settings: UiSettings, results: ToolOffsetResults) {
    ui_settings.set_material_offset_x(results.material_offset_x as f32);
    ui_settings.set_material_offset_y(results.material_offset_y as f32);
    ui_settings.set_offset_x(results.material_edge_margin_x as f32);
    ui_settings.set_offset_y(results.material_edge_margin_y as f32);
}

fn format_value(value: f64) -> String {
    format!("{value:.6}")
}

fn parse_input(value: &str) -> f64 {
    value.trim().replace(',', ".").parse().unwrap_or(0.0)
}
