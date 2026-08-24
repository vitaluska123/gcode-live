use std::cell::RefCell;
use std::rc::Rc;

use crate::domain::frame::{BoardBounds, FrameGeometry};

/// Immutable world-coordinate data consumed by a preview renderer for one frame.
#[derive(Clone, Default)]
pub struct PreviewSceneSnapshot {
    pub board_bounds: Option<BoardBounds>,
    pub frame_geometry: Option<FrameGeometry>,
    pub toolpath: Vec<(f64, f64)>,
    pub rapid_path: Vec<(f64, f64)>,
}

/// World-coordinate geometry displayed by the preview renderer.
///
/// The scene deliberately does not include camera state: pan and zoom belong
/// to `Viewport`, while this type remains the source of truth for previewed
/// objects.
pub struct PreviewScene {
    pub board_bounds: Rc<RefCell<Option<BoardBounds>>>,
    pub frame_geometry: Rc<RefCell<Option<FrameGeometry>>>,
    pub toolpath: Rc<RefCell<Vec<(f64, f64)>>>,
    pub rapid_path: Rc<RefCell<Vec<(f64, f64)>>>,
}

impl PreviewScene {
    pub fn new() -> Self {
        Self {
            board_bounds: Rc::new(RefCell::new(None)),
            frame_geometry: Rc::new(RefCell::new(None)),
            toolpath: Rc::new(RefCell::new(Vec::new())),
            rapid_path: Rc::new(RefCell::new(Vec::new())),
        }
    }

    /// Capture the scene without exposing its mutable storage to a renderer.
    pub fn snapshot(&self) -> PreviewSceneSnapshot {
        PreviewSceneSnapshot {
            board_bounds: self.board_bounds.borrow().clone(),
            frame_geometry: self.frame_geometry.borrow().clone(),
            toolpath: self.toolpath.borrow().clone(),
            rapid_path: self.rapid_path.borrow().clone(),
        }
    }
}

impl Default for PreviewScene {
    fn default() -> Self {
        Self::new()
    }
}
