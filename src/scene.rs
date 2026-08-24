use std::cell::RefCell;
use std::rc::Rc;

use crate::frame::{BoardBounds, FrameGeometry};

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
}

impl Default for PreviewScene {
    fn default() -> Self {
        Self::new()
    }
}
