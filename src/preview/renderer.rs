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

/// Complete implementation template for a preview backend.
///
/// The application creates one implementation at startup and talks only to
/// this interface. A backend may ignore the rendering lifecycle (software),
/// or use it to allocate resources in the context supplied by Slint (OpenGL).
pub trait PreviewRenderer {
    /// Accept an immutable scene snapshot. Implementations own their caches;
    /// neither this method nor the backend is allowed to mutate the scene.
    fn render(&mut self, frame: &RenderFrame) -> slint::Image;
    fn notify(
        &mut self,
        state: slint::RenderingState,
        api: &slint::GraphicsAPI<'_>,
    ) -> Option<slint::Image> {
        let _ = (state, api);
        None
    }
}

/// Runtime-selected renderer. New backends require no application changes:
/// they only implement `PreviewRenderer` and are selected here.
pub struct PreviewRendererBackend {
    implementation: Box<dyn PreviewRenderer>,
}

impl Default for PreviewRendererBackend {
    fn default() -> Self {
        Self::from_settings(true)
    }
}

impl PreviewRendererBackend {
    pub fn from_settings(use_opengl_renderer: bool) -> Self {
        let implementation: Box<dyn PreviewRenderer> = if use_opengl_renderer {
            Box::new(OpenGlPreviewRenderer::default())
        } else {
            Box::new(SoftwarePreviewRenderer)
        };
        Self { implementation }
    }
    pub fn render(&mut self, frame: &RenderFrame) -> slint::Image {
        self.implementation.render(frame)
    }

    pub fn notify(
        &mut self,
        state: slint::RenderingState,
        api: &slint::GraphicsAPI<'_>,
    ) -> Option<slint::Image> {
        self.implementation.notify(state, api)
    }
}
