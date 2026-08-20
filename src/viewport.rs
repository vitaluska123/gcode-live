/// Camera state shared by the UI interaction layer and the renderer.
#[derive(Debug, Clone, Copy)]
pub struct Viewport {
    pub zoom: f64,
    pub pan_x: f64,
    pub pan_y: f64,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan_x: 0.0,
            pan_y: 0.0,
        }
    }
}

impl Viewport {
    pub fn zoom_by(&mut self, direction: i32) {
        let factor = if direction > 0 { 1.25 } else { 0.8 };
        self.zoom = (self.zoom * factor).clamp(0.1, 20.0);
    }

    pub fn pan_by(&mut self, delta_x: f64, delta_y: f64) {
        self.pan_x += delta_x;
        self.pan_y += delta_y;
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_and_pan_are_bounded_and_resettable() {
        let mut viewport = Viewport::default();
        viewport.zoom_by(1);
        viewport.pan_by(12.0, -8.0);
        assert_eq!(viewport.zoom, 1.25);
        assert_eq!((viewport.pan_x, viewport.pan_y), (12.0, -8.0));
        viewport.reset();
        assert_eq!(viewport.zoom, 1.0);
        assert_eq!((viewport.pan_x, viewport.pan_y), (0.0, 0.0));
    }
}
