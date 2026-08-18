use crate::universe::{Cell, Universe};
use rustc_hash::FxHashSet;
use wasm_bindgen::Clamped;
use web_sys::CanvasRenderingContext2d;
use web_sys::ImageData;

// Color palette (R, G, B). Tweak these to change the wallpaper look.
const BG_COLOR: [u8; 3] = [10, 12, 22]; // deep navy background
const GRID_COLOR: [u8; 3] = [26, 30, 46]; // faint grid lines, just above bg
const CELL_COLOR: [u8; 3] = [175, 220, 255]; // soft cyan-white live cells

/// Precomputed screen-space geometry for a given camera/zoom, shared by both
/// the full-redraw and incremental-redraw paths so they always agree on
/// exactly where a given world cell lands on screen.
struct Geometry {
    width: u32,
    height: u32,
    half_w: i32,
    half_h: i32,
    ps: i32,
    camera_x: i32,
    camera_y: i32,
}

impl Geometry {
    fn new(universe: &dyn Universe, width: u32, height: u32) -> Self {
        let scale = universe.scale();
        let pixel_size: usize = if scale < 0 {
            1usize << (-scale) as u32 // Zoomed in
        } else {
            1 // Zoomed out (1:1)
        };
        Self {
            width,
            height,
            half_w: (width / 2) as i32,
            half_h: (height / 2) as i32,
            ps: pixel_size as i32,
            camera_x: universe.camera_x(),
            camera_y: universe.camera_y(),
        }
    }

    /// Unclamped top-left screen coordinate of a world cell's block.
    fn cell_screen_pos(&self, wx: i32, wy: i32) -> (i32, i32) {
        (
            self.half_w + (wx - self.camera_x) * self.ps,
            self.half_h + (wy - self.camera_y) * self.ps,
        )
    }
}

/// Stateful renderer: remembers what it last painted so it only has to touch
/// pixels that actually changed, instead of refilling the whole buffer every
/// frame. A full repaint only happens when the camera pans or the zoom level
/// changes, since those invalidate the screen-to-world mapping for every
/// pixel; a plain simulation tick only touches the handful of cells that
/// were born or died.
pub struct Renderer {
    width: u32,
    height: u32,
    prev_live_cells: FxHashSet<Cell>,
    prev_camera: (i32, i32),
    prev_scale: i32,
    initialized: bool,
}

impl Renderer {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            prev_live_cells: FxHashSet::default(),
            prev_camera: (0, 0),
            prev_scale: 0,
            initialized: false,
        }
    }

    pub fn render(
        &mut self,
        universe: &dyn Universe,
        context: &CanvasRenderingContext2d,
        rgba: &mut [u8],
    ) {
        let geo = Geometry::new(universe, self.width, self.height);
        let camera_or_zoom_changed = !self.initialized
            || geo.camera_x != self.prev_camera.0
            || geo.camera_y != self.prev_camera.1
            || universe.scale() != self.prev_scale;

        let mut painted_anything = camera_or_zoom_changed;

        if camera_or_zoom_changed {
            self.full_redraw(&geo, universe, rgba);
            self.prev_live_cells = universe.live_cells().clone();
            self.prev_camera = (geo.camera_x, geo.camera_y);
            self.prev_scale = universe.scale();
            self.initialized = true;
        } else {
            let current = universe.live_cells();

            // Cells that were on screen last frame but aren't anymore.
            let died: Vec<Cell> = self.prev_live_cells.difference(current).copied().collect();
            // Cells that are newly on screen this frame.
            let born: Vec<Cell> = current.difference(&self.prev_live_cells).copied().collect();

            for &(wx, wy) in &died {
                self.restore_background(&geo, rgba, wx, wy);
            }
            for &(wx, wy) in &born {
                self.paint_live(&geo, rgba, wx, wy);
            }

            if !died.is_empty() || !born.is_empty() {
                painted_anything = true;
                for cell in died {
                    self.prev_live_cells.remove(&cell);
                }
                for cell in born {
                    self.prev_live_cells.insert(cell);
                }
            }
        }

        // Nothing changed (paused, camera fixed) - skip the upload entirely.
        if !painted_anything {
            return;
        }

        let image_data =
            ImageData::new_with_u8_clamped_array_and_sh(Clamped(rgba), self.width, self.height)
                .unwrap();
        context.put_image_data(&image_data, 0.0, 0.0).unwrap();
    }

    fn full_redraw(&self, geo: &Geometry, universe: &dyn Universe, rgba: &mut [u8]) {
        let width = geo.width;
        let height = geo.height;

        // 1. Fill background
        for px in rgba.chunks_exact_mut(4) {
            px[0] = BG_COLOR[0];
            px[1] = BG_COLOR[1];
            px[2] = BG_COLOR[2];
            px[3] = 255;
        }

        // Grid lines land at half_w/half_h plus any integer multiple of
        // pixel_size - independent of camera_x/camera_y (see cell_screen_pos).
        let offset_x = ((geo.half_w % geo.ps) + geo.ps) % geo.ps;
        let offset_y = ((geo.half_h % geo.ps) + geo.ps) % geo.ps;

        for y in (offset_y as u32..height).step_by(geo.ps as usize) {
            let row_start = (y * width * 4) as usize;
            let row_end = row_start + (width * 4) as usize;
            for i in (row_start..row_end).step_by(4) {
                rgba[i] = GRID_COLOR[0];
                rgba[i + 1] = GRID_COLOR[1];
                rgba[i + 2] = GRID_COLOR[2];
                rgba[i + 3] = 255;
            }
        }

        for x in (offset_x as u32..width).step_by(geo.ps as usize) {
            for y in 0..height {
                let idx = ((y * width + x) as usize) * 4;
                rgba[idx] = GRID_COLOR[0];
                rgba[idx + 1] = GRID_COLOR[1];
                rgba[idx + 2] = GRID_COLOR[2];
                rgba[idx + 3] = 255;
            }
        }

        // 2. Draw live cells on top
        for &(wx, wy) in universe.live_cells() {
            self.paint_live(geo, rgba, wx, wy);
        }
    }

    /// Paints a single world cell's block as background/grid, exactly
    /// reconstructing what `full_redraw` would have painted there. This is
    /// computed purely from geometry (which pixel is a grid line vs.
    /// background), so it's correct regardless of what was drawn before.
    fn restore_background(&self, geo: &Geometry, rgba: &mut [u8], wx: i32, wy: i32) {
        let (sx, sy) = geo.cell_screen_pos(wx, wy);
        let Some((start_x, start_y, end_x, end_y)) =
            clamp_rect(sx, sy, geo.ps, geo.width, geo.height)
        else {
            return;
        };

        for y in start_y..end_y {
            let is_grid_row = y as i32 == sy;
            for x in start_x..end_x {
                let is_grid_col = x as i32 == sx;
                let color = if is_grid_row || is_grid_col {
                    GRID_COLOR
                } else {
                    BG_COLOR
                };
                let idx = (y * geo.width + x) as usize * 4;
                rgba[idx] = color[0];
                rgba[idx + 1] = color[1];
                rgba[idx + 2] = color[2];
                rgba[idx + 3] = 255;
            }
        }
    }

    /// Fills a single world cell's block entirely with the live-cell color.
    fn paint_live(&self, geo: &Geometry, rgba: &mut [u8], wx: i32, wy: i32) {
        let (sx, sy) = geo.cell_screen_pos(wx, wy);
        let Some((start_x, start_y, end_x, end_y)) =
            clamp_rect(sx, sy, geo.ps, geo.width, geo.height)
        else {
            return;
        };

        for y in start_y..end_y {
            for x in start_x..end_x {
                let idx = (y * geo.width + x) as usize * 4;
                rgba[idx] = CELL_COLOR[0];
                rgba[idx + 1] = CELL_COLOR[1];
                rgba[idx + 2] = CELL_COLOR[2];
                rgba[idx + 3] = 255;
            }
        }
    }
}

/// Clamps a cell's screen-space rect to the viewport, returning `None` if
/// it's entirely off-screen.
fn clamp_rect(sx: i32, sy: i32, ps: i32, width: u32, height: u32) -> Option<(u32, u32, u32, u32)> {
    if sx + ps <= 0 || sx >= width as i32 || sy + ps <= 0 || sy >= height as i32 {
        return None;
    }
    let start_x = sx.max(0) as u32;
    let start_y = sy.max(0) as u32;
    let end_x = (sx + ps).min(width as i32) as u32;
    let end_y = (sy + ps).min(height as i32) as u32;
    Some((start_x, start_y, end_x, end_y))
}
