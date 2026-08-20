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
