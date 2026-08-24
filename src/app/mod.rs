use slint::ComponentHandle;

use crate::{domain::settings::Settings, MainWindow, SettingsWindow, UiSettings};

mod export_actions;
mod file_actions;
mod preview_actions;
mod settings;
mod state;

use state::AppState;

/// Composes persistent settings, application state, and UI callback adapters.
pub fn run(main_window: MainWindow) -> Result<(), Box<dyn std::error::Error>> {
    let settings = match Settings::load() {
        Ok(settings) => settings,
        Err(error) => {
            eprintln!("Warning: Could not load settings: {error}");
            Settings::default()
        }
    };
    settings::initialize_ui(main_window.global::<UiSettings>(), &settings);
    let settings_window = SettingsWindow::new()?;
    settings::initialize_ui(settings_window.global::<UiSettings>(), &settings);

    let state = AppState::new(settings);
    settings::install_callbacks(&main_window, &state);
    settings::install_settings_window_callbacks(&main_window, &settings_window, &state);
    file_actions::install_callbacks(&main_window, &state);
    preview_actions::install_callbacks(&main_window, &state);
    export_actions::install_callbacks(&main_window, &state);

    main_window.run()?;
    Ok(())
}
