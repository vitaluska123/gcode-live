use crate::preview::PreviewData;
use crate::viewport::Viewport;

/// Pointer interaction state for the preview canvas.
#[derive(Default)]
pub struct PreviewInput {
    last_pointer: Option<(f64, f64)>,
}

impl PreviewInput {
    pub fn begin_pan(&mut self, x: f64, y: f64) {
        self.last_pointer = Some((x, y));
    }

    pub fn pan_to(&mut self, viewport: &mut Viewport, x: f64, y: f64) {
        if let Some((previous_x, previous_y)) = self.last_pointer {
            viewport.pan_by(x - previous_x, y - previous_y);
        }
        self.last_pointer = Some((x, y));
    }
}

/// Mapping between the preview canvas and immutable world geometry.
pub struct PreviewTransform {
    data: PreviewData,
    width: f64,
    height: f64,
}

impl PreviewTransform {
    pub fn new(data: PreviewData, width: f32, height: f32) -> Option<Self> {
        if width <= 0.0 || height <= 0.0 {
            return None;
        }
        Some(Self {
            data,
            width: width as f64,
            height: height as f64,
        })
    }

    pub fn screen_to_world(&self, viewport: Viewport, x: f64, y: f64) -> Option<(f64, f64)> {
        let scale = self.scale(viewport);
        if scale <= 0.0 {
            return None;
        }
        let (min_x, max_x, min_y, max_y) = self.data.world_bounds();
        let content_width = max_x - min_x;
        let content_height = max_y - min_y;
        Some((
            min_x + (x - (self.width - content_width * scale) / 2.0 - viewport.pan_x) / scale,
            min_y + ((self.height + content_height * scale) / 2.0 + viewport.pan_y - y) / scale,
        ))
    }

    pub fn zoom_at(&self, viewport: &mut Viewport, direction: i32, x: f64, y: f64) {
        let Some((world_x, world_y)) = self.screen_to_world(*viewport, x, y) else {
            return;
        };
        let (min_x, max_x, min_y, max_y) = self.data.world_bounds();
        let content_width = max_x - min_x;
        let content_height = max_y - min_y;
        viewport.zoom_by(direction);
        let scale = self.scale(*viewport);
        viewport.pan_x = x - (self.width - content_width * scale) / 2.0 - (world_x - min_x) * scale;
        viewport.pan_y =
            y - (self.height + content_height * scale) / 2.0 + (world_y - min_y) * scale;
    }

    fn scale(&self, viewport: Viewport) -> f64 {
        self.data
            .calculate_scale(self.width as f32, self.height as f32)
            * viewport.zoom
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panning_uses_pointer_delta() {
        let mut input = PreviewInput::default();
        let mut viewport = Viewport::default();
        input.begin_pan(10.0, 15.0);
        input.pan_to(&mut viewport, 16.0, 11.0);

        assert_eq!((viewport.pan_x, viewport.pan_y), (6.0, -4.0));
    }

    #[test]
    fn zoom_keeps_the_cursor_world_position() {
        let data = PreviewData::from_bounds_with_material(
            0.0, 100.0, 0.0, 100.0, 0.0, 100.0, 0.0, 100.0, 0.0, 0.0, 0.0, 0.0,
        );
        let transform = PreviewTransform::new(data, 400.0, 400.0).unwrap();
        let mut viewport = Viewport::default();
        let before = transform.screen_to_world(viewport, 120.0, 180.0).unwrap();

        transform.zoom_at(&mut viewport, 1, 120.0, 180.0);
        let after = transform.screen_to_world(viewport, 120.0, 180.0).unwrap();

        assert!((before.0 - after.0).abs() < f64::EPSILON);
        assert!((before.1 - after.1).abs() < f64::EPSILON);
    }
}
