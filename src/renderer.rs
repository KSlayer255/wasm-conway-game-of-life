use crate::universe::Universe;
use wasm_bindgen::Clamped;
use web_sys::CanvasRenderingContext2d;
use web_sys::ImageData;

// Color palette (R, G, B). Tweak these to change the wallpaper look.
const BG_COLOR: [u8; 3] = [10, 12, 22]; // deep navy background
const GRID_COLOR: [u8; 3] = [26, 30, 46]; // faint grid lines, just above bg
const CELL_COLOR: [u8; 3] = [175, 220, 255]; // soft cyan-white live cells

pub fn render(
    universe: &dyn Universe,
    context: &CanvasRenderingContext2d,
    width: u32,
    height: u32,
    rgba: &mut [u8],
) {
    let scale = universe.scale();
    let pixel_size = if scale < 0 {
        1 << (-scale) as u32 // Zoomed in
    } else {
        1 // Zoomed out (1:1) - we'll keep it simple for now
    };

    let ps = pixel_size as i32;

    // 1. Create the pixel buffer
    for px in rgba.chunks_exact_mut(4) {
        px[0] = BG_COLOR[0];
        px[1] = BG_COLOR[1];
        px[2] = BG_COLOR[2];
        px[3] = 255;
    }

    let camera_x = universe.camera_x();
    let camera_y = universe.camera_y();

    let half_w = (width / 2) as i32;
    let half_h = (height / 2) as i32;

    // Grid lines land at half_w/half_h plus any integer multiple of
    // pixel_size. That multiple is (wx - camera_x), which is always a whole
    // number of cells, so the *phase* of the grid relative to the screen only
    // depends on half_w/half_h and pixel_size - not on where the camera is.
    let offset_x = ((half_w % ps) + ps) % ps;
    let offset_y = ((half_h % ps) + ps) % ps;

    for y in (offset_y as u32..height).step_by(pixel_size) {
        let row_start = (y * width * 4) as usize;
        let row_end = row_start + (width * 4) as usize;
        for i in (row_start..row_end).step_by(4) {
            rgba[i] = GRID_COLOR[0];
            rgba[i + 1] = GRID_COLOR[1];
            rgba[i + 2] = GRID_COLOR[2];
            rgba[i + 3] = 255;
        }
    }

    for x in (offset_x as u32..width).step_by(pixel_size) {
        for y in 0..height {
            let idx = ((y * width + x) as usize) * 4;
            rgba[idx] = GRID_COLOR[0];
            rgba[idx + 1] = GRID_COLOR[1];
            rgba[idx + 2] = GRID_COLOR[2];
            rgba[idx + 3] = 255;
        }
    }

    // 2. Draw live cells (scaled)
    let offset_x = camera_x - width as i32 / 2;
    let offset_y = camera_y - height as i32 / 2;
    for &(wx, wy) in universe.live_cells() {
        // Convert world coordinate to screen coordinate (top-left of the cell)
        let sx = half_w + (wx - camera_x) * ps;
        let sy = half_h + (wy - camera_y) * ps;

        // Only draw if the cell is within the viewport
        if sx + ps > 0 && sx < width as i32 && sy + ps > 0 && sy < height as i32 {
            // Clamp the cell to the viewport boundaries
            let start_x = sx.max(0) as u32;
            let start_y = sy.max(0) as u32;
            let end_x = (sx + ps).min(width as i32) as u32;
            let end_y = (sy + ps).min(height as i32) as u32;

            for y in start_y..end_y {
                for x in start_x..end_x {
                    let idx = (y * width + x) as usize * 4;
                    rgba[idx] = CELL_COLOR[0];
                    rgba[idx + 1] = CELL_COLOR[1];
                    rgba[idx + 2] = CELL_COLOR[2];
                    rgba[idx + 3] = 255;
                }
            }
        }
    }

    // 3. Push pixel buffer to canvas
    let image_data =
        ImageData::new_with_u8_clamped_array_and_sh(Clamped(rgba), width, height).unwrap();
    context.put_image_data(&image_data, 0.0, 0.0).unwrap();
}
