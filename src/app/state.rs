use std::cell::RefCell;
use std::rc::Rc;

use crate::domain::settings::Settings;
use crate::preview::input::{PreviewInput, RulerMeasurement};
use crate::preview::scene::PreviewScene;
use crate::preview::viewport::Viewport;

/// Mutable application model shared by the UI callback adapters.
///
/// Geometry, camera state, interaction state, and settings remain grouped so
/// callbacks do not independently own competing pieces of application state.
pub(crate) struct AppState {
    pub(crate) preview_scene: Rc<PreviewScene>,
    pub(crate) source_home: Rc<RefCell<Option<(f64, f64)>>>,
    pub(crate) source_file_stem: Rc<RefCell<Option<String>>>,
    pub(crate) viewport: Rc<RefCell<Viewport>>,
    pub(crate) preview_input: Rc<RefCell<PreviewInput>>,
    pub(crate) ruler: Rc<RefCell<RulerMeasurement>>,
    pub(crate) settings: Rc<RefCell<Settings>>,
}

impl AppState {
    pub(crate) fn new(settings: Settings) -> Self {
        Self {
            preview_scene: Rc::new(PreviewScene::new()),
            source_home: Rc::new(RefCell::new(None)),
            source_file_stem: Rc::new(RefCell::new(None)),
            viewport: Rc::new(RefCell::new(Viewport::default())),
            preview_input: Rc::new(RefCell::new(PreviewInput::default())),
            ruler: Rc::new(RefCell::new(RulerMeasurement::default())),
            settings: Rc::new(RefCell::new(settings)),
        }
    }
}
