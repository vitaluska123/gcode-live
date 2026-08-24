//! Shared immutable preview-rendering contract and backend selection.

use crate::domain::settings::Settings;
use crate::preview::opengl_renderer::OpenGlPreviewRenderer;
use crate::preview::scene::PreviewSceneSnapshot;
use crate::preview::software_renderer::SoftwarePreviewRenderer;
use crate::preview::viewport::Viewport;

pub struct RenderFrame {
    pub width: u32,
    pub height: u32,
    pub scene: PreviewSceneSnapshot,
    pub settings: Settings,
    pub viewport: Viewport,
}

pub trait PreviewRenderer {
    fn render(&mut self, frame: &RenderFrame) -> slint::Image;
}

pub enum PreviewRendererBackend {
    OpenGl(OpenGlPreviewRenderer),
    Software(SoftwarePreviewRenderer),
}

impl Default for PreviewRendererBackend {
    fn default() -> Self {
        match std::env::var("CNC_PREVIEW_RENDERER").as_deref() {
            Ok("software") => Self::Software(SoftwarePreviewRenderer),
            _ => Self::OpenGl(OpenGlPreviewRenderer::default()),
        }
    }
}

impl PreviewRendererBackend {
    pub fn render(&mut self, frame: &RenderFrame) -> slint::Image {
        match self {
            Self::OpenGl(renderer) => renderer.render(frame),
            Self::Software(renderer) => renderer.render(frame),
        }
    }
}
