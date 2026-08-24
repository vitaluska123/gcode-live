#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

slint::include_modules!();

use slint::winit_030::{winit::window::Icon, WinitWindowAccessor};

mod app;
mod domain {
    pub mod frame;
    pub mod gcode;
    pub mod settings;
}
mod export {
    pub mod gcode;
}
mod preview {
    pub mod data;
    pub mod input;
    pub mod opengl_renderer;
    pub mod renderer;
    pub mod scene;
    pub mod software_renderer;
    pub mod viewport;
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    slint::BackendSelector::new()
        // FemtoVG creates and owns the window's OpenGL context. Slint's winit
        // backend automatically falls back to the compiled software renderer
        // if this context cannot be created on the current system.
        .backend_name("gl".into())
        .select()?;
    let main_window = MainWindow::new()?;
    set_window_icon(&main_window)?;
    app::run(main_window)
}

/// Use the supplied PNG at runtime so the window, taskbar and Alt+Tab entry
/// show the same application logo.
fn set_window_icon(main_window: &MainWindow) -> Result<(), Box<dyn std::error::Error>> {
    let image = image::load_from_memory(include_bytes!("../icons/GcodeFrameGen.png"))?.into_rgba8();
    let (width, height) = image.dimensions();
    let icon = Icon::from_rgba(image.into_raw(), width, height)?;
    main_window
        .window()
        .with_winit_window(|window| window.set_window_icon(Some(icon)));
    Ok(())
}
