//! OpenGL-compositor preview backend.

use crate::preview::renderer::{PreviewRenderer, RenderFrame};
use crate::preview::software_renderer::SoftwarePreviewRenderer;

#[derive(Default)]
pub struct OpenGlPreviewRenderer {
    software_fallback: SoftwarePreviewRenderer,
}

impl PreviewRenderer for OpenGlPreviewRenderer {
    fn render(&mut self, frame: &RenderFrame) -> slint::Image {
        self.software_fallback.render(frame)
    }
}
