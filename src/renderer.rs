use crate::universe::Universe;
use wasm_bindgen::Clamped;
use web_sys::CanvasRenderingContext2d;
use web_sys::ImageData;

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

    // 1. Create the pixel buffer
    let bg_color = 30;
    rgba.fill(bg_color);

    let cell_color = 255;
    let camera_x = universe.camera_x();
    let camera_y = universe.camera_y();

    let grid_color = 50; // Slightly lighter than bg, dark enough to be subtle.

    for y in (0..height).step_by(pixel_size) {
        let row_start = (y * width * 4) as usize;
        let row_end = row_start + (width * 4) as usize;
        for i in (row_start..row_end).step_by(4) {
            rgba[i] = grid_color;
            rgba[i + 1] = grid_color;
            rgba[i + 2] = grid_color;
            rgba[i + 3] = 255;
        }
    }

    for x in (0..width).step_by(pixel_size) {
        for y in 0..height {
            let idx = ((y * width + x) as usize) * 4;
            rgba[idx] = grid_color;
            rgba[idx + 1] = grid_color;
            rgba[idx + 2] = grid_color;
            rgba[idx + 3] = 255;
        }
    }

    // 2. Draw live cells (scaled)
    for &(wx, wy) in universe.live_cells() {
        // Convert world coordinate to screen coordinate (top-left of the cell)
        let sx = (wx - camera_x) * pixel_size as i32;
        let sy = (wy - camera_y) * pixel_size as i32;

        // Only draw if the cell is within the viewport
        if sx + pixel_size as i32 > 0
            && sx < width as i32
            && sy + pixel_size as i32 > 0
            && sy < height as i32
        {
            // Clamp the cell to the viewport boundaries
            let start_x = sx.max(0) as u32;
            let start_y = sy.max(0) as u32;
            let end_x = (sx + pixel_size as i32).min(width as i32) as u32;
            let end_y = (sy + pixel_size as i32).min(height as i32) as u32;

            for y in start_y..end_y {
                for x in start_x..end_x {
                    let idx = (y * width + x) as usize * 4;
                    rgba[idx] = cell_color;
                    rgba[idx + 1] = cell_color;
                    rgba[idx + 2] = cell_color;
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
