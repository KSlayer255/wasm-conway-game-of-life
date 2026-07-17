mod input;
mod pattern;
mod renderer;
mod universe;

use crate::input::InputManager;
use crate::universe::Universe;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use web_sys::window;

fn main() {
    console_error_panic_hook::set_once();

    let document = window().unwrap().document().unwrap();
    let canvas = document
        .get_element_by_id("canvas")
        .unwrap()
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .unwrap();
    let context = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .unwrap();
    let hud_element = document
        .get_element_by_id("hud")
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap();

    let width = canvas.width();
    let height = canvas.height();

    let mut render_buffer = vec![0u8; (width * height * 4) as usize];

    // Load the embedded pattern and centre it.
    let pattern_str = include_str!("../patterns/gun-p165mwss.rle");
    let mut cells = pattern::load_pattern_from_str(pattern_str);
    cells = pattern::centre_cells(&cells, width, height);
    let mut universe = universe::SparseUniverse::new(cells);

    // --- Keyboard state ---
    let input_manager = InputManager::new();

    let mut paused = false;
    let mut steps_per_frame = 1;
    const MAX_STEPS: usize = 1024;

    // --- Animation loop ---
    let f = Rc::new(RefCell::new(None::<Closure<dyn FnMut()>>));
    let g = f.clone();

    *g.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        let input = input_manager.update();

        let mut dx = 0;
        let mut dy = 0;

        //Controls
        if input.is_pressed(input::KeyState::K) || input.is_pressed(input::KeyState::UP) {
            dy -= 1;
        }
        if input.is_pressed(input::KeyState::J) || input.is_pressed(input::KeyState::DOWN) {
            dy += 1;
        }
        if input.is_pressed(input::KeyState::H) || input.is_pressed(input::KeyState::LEFT) {
            dx -= 1;
        }
        if input.is_pressed(input::KeyState::L) || input.is_pressed(input::KeyState::RIGHT) {
            dx += 1;
        }
        if input.is_just_pressed(input::KeyState::Z) {
            universe.zoom_in();
        }
        if input.is_just_pressed(input::KeyState::X) {
            universe.zoom_out();
        }
        if input.is_just_pressed(input::KeyState::P) {
            paused = !paused;
        }
        if input.is_just_pressed(input::KeyState::O) {
            steps_per_frame = (steps_per_frame * 2).min(MAX_STEPS);
        }
        if input.is_just_pressed(input::KeyState::I) {
            steps_per_frame = (steps_per_frame / 2).max(1);
        }

        if !paused {
            for _ in 0..steps_per_frame {
                universe.tick();
            }
        }

        universe.pan(dx, dy);

        // HUD
        let paused_text = if paused { "Paused" } else { "Running" };
        let text = format!(
            "Camera: ({}, {}) | Cells: {} | {} | Speed {}x",
            universe.camera_x(),
            universe.camera_y(),
            universe.live_cells().len(),
            paused_text,
            steps_per_frame
        );
        hud_element.set_text_content(Some(&text));

        // Render
        renderer::render(&universe, &context, width, height, &mut render_buffer);

        // Next frame
        window()
            .unwrap()
            .request_animation_frame(f.borrow().as_ref().unwrap().as_ref().unchecked_ref())
            .unwrap();
    }) as Box<dyn FnMut()>));

    window()
        .unwrap()
        .request_animation_frame(g.borrow().as_ref().unwrap().as_ref().unchecked_ref())
        .unwrap();
}
