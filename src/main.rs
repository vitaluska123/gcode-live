slint::include_modules!();

mod app;
mod exporter;
mod frame;
mod preview;
mod preview_renderer;
mod settings;
mod viewport;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    slint::BackendSelector::new()
        .backend_name("winit".into())
        .select()?;
    app::run(MainWindow::new()?)
}
