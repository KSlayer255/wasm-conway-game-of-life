use crate::universe::Universe;
use wasm_bindgen::Clamped;
use web_sys::CanvasRenderingContext2d;
use web_sys::ImageData;

pub fn render(
    universe: &dyn Universe,
    context: &CanvasRenderingContext2d,
    width: u32,
    height: u32,
) {
    let scale = universe.scale();
    let pixel_size = if scale < 0 {
        1 << (-scale) as u32 // Zoomed in
    } else {
        1 // Zoomed out (1:1) - we'll keep it simple for now
    };

    // 1. Create the pixel buffer
    let mut rgba = vec![0u8; (width * height * 4) as usize];
    let bg_color = 30;
    rgba.fill(bg_color);

    let cell_color = 255;
    let camera_x = universe.camera_x();
    let camera_y = universe.camera_y();

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
        ImageData::new_with_u8_clamped_array_and_sh(Clamped(&rgba), width, height).unwrap();
    context.put_image_data(&image_data, 0.0, 0.0).unwrap();

    // 4. Draw grid lines (only when zoomed in)
    if scale < 0 {
        draw_grid_lines(context, width, height, pixel_size);
    }
}

fn draw_grid_lines(context: &CanvasRenderingContext2d, width: u32, height: u32, pixel_size: u32) {
    // Set up line style
    context.set_stroke_style_str("rgba(140, 170, 200, 0.4)");
    context.set_line_width(0.5);
    context.begin_path();

    // Vertical lines at every `pixel_size` pixels
    let mut x = 0;
    while x <= width {
        let screen_x = x as f64;
        context.move_to(screen_x, 0.0);
        context.line_to(screen_x, height as f64);
        x += pixel_size;
    }

    // Horizontal lines at every `pixel_size` pixels
    let mut y = 0;
    while y <= height {
        let screen_y = y as f64;
        context.move_to(0.0, screen_y);
        context.line_to(width as f64, screen_y);
        y += pixel_size;
    }

    context.stroke();
}
