use crate::config::{INITIAL_SCALE, MAX_SCALE, MIN_SCALE};

/// View state: where the camera is looking in world-cell coordinates, and
/// the current zoom level. Deliberately independent of any `Universe`
/// implementation - simulation state and view state are orthogonal, and
/// every `Universe` impl (sparse, and eventually change-list / QuickLife /
/// HashLife / dense-grid) shares one `Camera` rather than reimplementing
/// its own pan/zoom bookkeeping.
pub struct Camera {
    x: i32,
    y: i32,
    scale: i32,
}

impl Camera {
    pub fn new() -> Self {
        Self {
            x: 0,
            y: 0,
            scale: INITIAL_SCALE,
        }
    }

    pub fn x(&self) -> i32 {
        self.x
    }

    pub fn y(&self) -> i32 {
        self.y
    }

    pub fn scale(&self) -> i32 {
        self.scale
    }

    pub fn pan(&mut self, dx: i32, dy: i32) {
        self.x += dx;
        self.y += dy;
    }

    pub fn zoom_in(&mut self) {
        if self.scale > MIN_SCALE {
            self.scale -= 1;
        }
    }

    pub fn zoom_out(&mut self) {
        if self.scale < MAX_SCALE {
            self.scale += 1;
        }
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_camera_starts_at_origin_and_default_zoom() {
        let camera = Camera::new();
        assert_eq!((camera.x(), camera.y()), (0, 0));
        assert_eq!(camera.scale(), INITIAL_SCALE);
    }

    #[test]
    fn pan_moves_by_exact_delta_and_accumulates() {
        let mut camera = Camera::new();
        camera.pan(3, -2);
        assert_eq!((camera.x(), camera.y()), (3, -2));
        camera.pan(-1, -1);
        assert_eq!((camera.x(), camera.y()), (2, -3));
    }

    #[test]
    fn zoom_in_decreases_scale_by_one() {
        let mut camera = Camera::new();
        let before = camera.scale();
        camera.zoom_in();
        assert_eq!(camera.scale(), before - 1);
    }

    #[test]
    fn zoom_out_increases_scale_by_one() {
        let mut camera = Camera::new();
        let before = camera.scale();
        camera.zoom_out();
        assert_eq!(camera.scale(), before + 1);
    }

    #[test]
    fn zoom_in_clamps_at_min_scale() {
        let mut camera = Camera::new();
        for _ in 0..32 {
            camera.zoom_in();
        }
        assert_eq!(camera.scale(), MIN_SCALE);
    }

    #[test]
    fn zoom_out_clamps_at_max_scale() {
        let mut camera = Camera::new();
        for _ in 0..32 {
            camera.zoom_out();
        }
        assert_eq!(camera.scale(), MAX_SCALE);
    }
}
