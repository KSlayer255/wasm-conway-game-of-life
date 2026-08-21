use crate::camera::Camera;
use crate::config::{
    AGE_COLOR_REFRESH_INTERVAL_MS, BG_COLOR, CELL_HUE_OLD_DEG, CELL_HUE_YOUNG_DEG, CELL_LIGHTNESS,
    CELL_SATURATION, GRID_COLOR,
};
use crate::universe::{Cell, Universe};
use rustc_hash::FxHashSet;
use wasm_bindgen::Clamped;
use web_sys::CanvasRenderingContext2d;
use web_sys::ImageData;

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> [u8; 3] {
    if s <= 0.0 {
        let v = (l * 255.0).round() as u8;
        return [v, v, v];
    }
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let h_prime = h.rem_euclid(360.0) / 60.0;
    let x = c * (1.0 - (h_prime % 2.0 - 1.0).abs());
    let (r1, g1, b1) = match h_prime as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    [
        ((r1 + m) * 255.0).round() as u8,
        ((g1 + m) * 255.0).round() as u8,
        ((b1 + m) * 255.0).round() as u8,
    ]
}
fn color_for_age(age: u32, bounds: (u32, u32)) -> [u8; 3] {
    let (min_age, max_age) = bounds;
    let t = if max_age <= min_age {
        0.0
    } else {
        (age.saturating_sub(min_age) as f32 / (max_age - min_age) as f32).clamp(0.0, 1.0)
    };
    let hue = CELL_HUE_YOUNG_DEG + (CELL_HUE_OLD_DEG - CELL_HUE_YOUNG_DEG) * t;
    hsl_to_rgb(hue, CELL_SATURATION, CELL_LIGHTNESS)
}

struct Geometry {
    width: u32,
    height: u32,
    half_w: i32,
    half_h: i32,
    ps: i32,
    camera_x: i32,
    camera_y: i32,
}

pub fn pixel_size_for_scale(scale: i32) -> i32 {
    if scale < 0 {
        1i32 << (-scale) as u32 // Zoomed in
    } else {
        1 // Zoomed out (1:1)
    }
}

impl Geometry {
    fn new(camera: &Camera, width: u32, height: u32) -> Self {
        let scale = camera.scale();
        Self {
            width,
            height,
            half_w: (width / 2) as i32,
            half_h: (height / 2) as i32,
            ps: pixel_size_for_scale(scale),
            camera_x: camera.x(),
            camera_y: camera.y(),
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
    age_bounds: (u32, u32),
    last_age_refresh_time: f64,
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
            age_bounds: (0, 0),
            last_age_refresh_time: f64::NEG_INFINITY,
        }
    }

    pub fn render(
        &mut self,
        universe: &dyn Universe,
        camera: &Camera,
        context: &CanvasRenderingContext2d,
        rgba: &mut [u8],
        now: f64,
    ) {
        let geo = Geometry::new(camera, self.width, self.height);
        let camera_or_zoom_changed = !self.initialized
            || geo.camera_x != self.prev_camera.0
            || geo.camera_y != self.prev_camera.1
            || camera.scale() != self.prev_scale;

        if camera_or_zoom_changed {
            self.age_bounds = universe.age_bounds();
            self.last_age_refresh_time = now;

            self.full_redraw(&geo, universe, self.age_bounds, rgba);
            self.prev_live_cells.clear();
            self.prev_live_cells
                .extend(universe.live_cells().iter().copied());
            self.prev_camera = (geo.camera_x, geo.camera_y);
            self.prev_scale = camera.scale();
            self.initialized = true;

            let image_data =
                ImageData::new_with_u8_clamped_array_and_sh(Clamped(rgba), self.width, self.height)
                    .unwrap();
            context.put_image_data(&image_data, 0.0, 0.0).unwrap();
        }
        let age_refresh_due = now - self.last_age_refresh_time >= AGE_COLOR_REFRESH_INTERVAL_MS;
        if age_refresh_due {
            self.age_bounds = universe.age_bounds();
            self.last_age_refresh_time = now;
        }

        let current = universe.live_cells();
        let to_paint: Vec<Cell> = if age_refresh_due {
            current.iter().copied().collect()
        } else {
            current.difference(&self.prev_live_cells).copied().collect()
        };
        let died: Vec<Cell> = self.prev_live_cells.difference(current).copied().collect();

        if died.is_empty() && to_paint.is_empty() {
            return;
        }

        let mut dirty: Option<(u32, u32, u32, u32)> = None;
        for &(wx, wy) in &died {
            if let Some(r) = self.restore_background(&geo, rgba, wx, wy) {
                dirty = Some(union_rect(dirty, r))
            }
        }

        for &(wx, wy) in &to_paint {
            let color = color_for_age(universe.age_of(&(wx, wy)), self.age_bounds);
            if let Some(r) = self.paint_live(&geo, rgba, wx, wy, color) {
                dirty = Some(union_rect(dirty, r))
            }
        }

        for cell in died {
            self.prev_live_cells.remove(&cell);
        }

        self.prev_live_cells.extend(to_paint);

        let Some((start_x, start_y, end_x, end_y)) = dirty else {
            return;
        };

        let image_data =
            ImageData::new_with_u8_clamped_array_and_sh(Clamped(rgba), self.width, self.height)
                .unwrap();
        context
            .put_image_data_with_dirty_x_and_dirty_y_and_dirty_width_and_dirty_height(
                &image_data,
                0.0,
                0.0,
                start_x as f64,
                start_y as f64,
                (end_x - start_x) as f64,
                (end_y - start_y) as f64,
            )
            .unwrap();
    }

    fn full_redraw(
        &self,
        geo: &Geometry,
        universe: &dyn Universe,
        bounds: (u32, u32),
        rgba: &mut [u8],
    ) {
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
        if geo.ps > 5 {
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
        }

        // 2. Draw live cells on top
        for &(wx, wy) in universe.live_cells() {
            let color = color_for_age(universe.age_of(&(wx, wy)), bounds);
            self.paint_live(geo, rgba, wx, wy, color);
        }
    }

    /// Paints a single world cell's block as background/grid, exactly
    /// reconstructing what `full_redraw` would have painted there. This is
    /// computed purely from geometry (which pixel is a grid line vs.
    /// background), so it's correct regardless of what was drawn before.
    fn restore_background(
        &self,
        geo: &Geometry,
        rgba: &mut [u8],
        wx: i32,
        wy: i32,
    ) -> Option<(u32, u32, u32, u32)> {
        let (sx, sy) = geo.cell_screen_pos(wx, wy);
        let (start_x, start_y, end_x, end_y) = clamp_rect(sx, sy, geo.ps, geo.width, geo.height)?;

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
        Some((start_x, start_y, end_x, end_y))
    }

    /// Fills a single world cell's block entirely with the live-cell color.
    fn paint_live(
        &self,
        geo: &Geometry,
        rgba: &mut [u8],
        wx: i32,
        wy: i32,
        color: [u8; 3],
    ) -> Option<(u32, u32, u32, u32)> {
        let (sx, sy) = geo.cell_screen_pos(wx, wy);
        let (start_x, start_y, end_x, end_y) = clamp_rect(sx, sy, geo.ps, geo.width, geo.height)?;

        for y in start_y..end_y {
            for x in start_x..end_x {
                let idx = (y * geo.width + x) as usize * 4;
                rgba[idx] = color[0];
                rgba[idx + 1] = color[1];
                rgba[idx + 2] = color[2];
                rgba[idx + 3] = 255;
            }
        }
        Some((start_x, start_y, end_x, end_y))
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

fn union_rect(acc: Option<(u32, u32, u32, u32)>, r: (u32, u32, u32, u32)) -> (u32, u32, u32, u32) {
    match acc {
        None => r,
        Some((ax0, ay0, ax1, ay1)) => (ax0.min(r.0), ay0.min(r.1), ax1.max(r.2), ay1.max(r.3)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::Camera;

    // --- clamp_rect ---

    #[test]
    fn clamp_rect_fully_on_screen() {
        assert_eq!(clamp_rect(10, 10, 8, 100, 100), Some((10, 10, 18, 18)));
    }

    #[test]
    fn clamp_rect_clips_to_viewport_edges() {
        // Straddles the bottom-right edge of a 100x100 viewport.
        assert_eq!(clamp_rect(95, 95, 8, 100, 100), Some((95, 95, 100, 100)));
        // Straddles the top-left edge (negative sx/sy).
        assert_eq!(clamp_rect(-3, -3, 8, 100, 100), Some((0, 0, 5, 5)));
    }

    #[test]
    fn clamp_rect_entirely_off_screen_returns_none() {
        assert_eq!(clamp_rect(200, 10, 8, 100, 100), None); // past right
        assert_eq!(clamp_rect(10, 200, 8, 100, 100), None); // past bottom
        assert_eq!(clamp_rect(-20, 10, 8, 100, 100), None); // past left
        assert_eq!(clamp_rect(10, -20, 8, 100, 100), None); // past top
    }

    #[test]
    fn clamp_rect_touching_far_edge_exactly_is_off_screen() {
        // A block starting exactly at the viewport's far edge covers zero
        // visible pixels.
        assert_eq!(clamp_rect(100, 10, 8, 100, 100), None);
    }

    // --- union_rect (dirty-rect accumulation, item 1) ---

    #[test]
    fn union_rect_seeds_from_none() {
        assert_eq!(union_rect(None, (5, 5, 10, 10)), (5, 5, 10, 10));
    }

    #[test]
    fn union_rect_grows_to_cover_disjoint_rects() {
        let acc = Some((5, 5, 10, 10));
        assert_eq!(union_rect(acc, (20, 2, 25, 8)), (5, 2, 25, 10));
    }

    #[test]
    fn union_rect_handles_overlapping_rects() {
        let acc = Some((0, 0, 10, 10));
        assert_eq!(union_rect(acc, (5, 5, 15, 15)), (0, 0, 15, 15));
    }

    // --- pixel_size_for_scale ---

    #[test]
    fn pixel_size_doubles_per_zoom_level() {
        assert_eq!(pixel_size_for_scale(0), 1);
        assert_eq!(pixel_size_for_scale(-1), 2);
        assert_eq!(pixel_size_for_scale(-4), 16);
        assert_eq!(pixel_size_for_scale(-8), 256);
    }

    // --- Geometry::cell_screen_pos ---

    /// Builds a `Camera` at a specific position/zoom using only its public
    /// API (pan/zoom_in/zoom_out), since its fields are private by design.
    fn camera_at(x: i32, y: i32, scale: i32) -> Camera {
        let mut camera = Camera::new();
        camera.pan(x, y);
        while camera.scale() > scale {
            camera.zoom_in();
        }
        while camera.scale() < scale {
            camera.zoom_out();
        }
        camera
    }

    #[test]
    fn cell_screen_pos_camera_cell_is_screen_centered() {
        let camera = camera_at(5, 5, -2); // ps = 4
        let geo = Geometry::new(&camera, 200, 100);
        assert_eq!(geo.cell_screen_pos(5, 5), (100, 50));
    }

    #[test]
    fn cell_screen_pos_scales_with_zoom() {
        let camera = camera_at(0, 0, -3); // ps = 8
        let geo = Geometry::new(&camera, 200, 100);
        assert_eq!(geo.cell_screen_pos(1, 0), (108, 50));
        assert_eq!(geo.cell_screen_pos(0, 1), (100, 58));
    }

    // --- dirty-rect calculation, combining clamp_rect + union_rect the
    // way Renderer::render does for an incremental frame ---

    #[test]
    fn dirty_rect_covers_all_changed_cells() {
        let camera = camera_at(0, 0, -3); // ps = 8
        let geo = Geometry::new(&camera, 200, 100);
        let changed = [(0, 0), (2, 0)]; // two cells, 16px apart on screen

        let mut dirty: Option<(u32, u32, u32, u32)> = None;
        for &(wx, wy) in &changed {
            let (sx, sy) = geo.cell_screen_pos(wx, wy);
            if let Some(r) = clamp_rect(sx, sy, geo.ps, geo.width, geo.height) {
                dirty = Some(union_rect(dirty, r));
            }
        }

        // (0,0) covers screen x [100,108); (2,0) covers [116,124).
        assert_eq!(dirty, Some((100, 50, 124, 58)));
    }

    // --- color_for_age (relative age-color gradient) ---

    #[test]
    fn color_for_age_no_spread_is_youngest_color() {
        let youngest = hsl_to_rgb(CELL_HUE_YOUNG_DEG, CELL_SATURATION, CELL_LIGHTNESS);
        assert_eq!(color_for_age(50, (0, 0)), youngest); // untracked default
        assert_eq!(color_for_age(7, (7, 7)), youngest); // uniform population
    }

    #[test]
    fn color_for_age_endpoints_match_palette() {
        assert_eq!(
            color_for_age(0, (0, 100)),
            hsl_to_rgb(CELL_HUE_YOUNG_DEG, CELL_SATURATION, CELL_LIGHTNESS)
        );
        assert_eq!(
            color_for_age(100, (0, 100)),
            hsl_to_rgb(CELL_HUE_OLD_DEG, CELL_SATURATION, CELL_LIGHTNESS)
        );
    }

    #[test]
    fn color_for_age_is_relative_not_absolute() {
        // Same absolute age (50), different population spreads - a young
        // population and an old-skewing population should not necessarily
        // agree on how "old" age 50 counts as.
        let in_young_population = color_for_age(50, (0, 100));
        let in_old_population = color_for_age(50, (40, 90));
        assert_ne!(in_young_population, in_old_population);
    }
}
