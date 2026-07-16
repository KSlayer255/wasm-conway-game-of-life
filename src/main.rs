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

    // Load the embedded pattern and centre it.
    let pattern_str = include_str!("../patterns/gun-p165mwss.rle");
    let mut cells = pattern::load_pattern_from_str(pattern_str);
    cells = pattern::centre_cells(&cells, width, height);
    let mut universe = universe::SparseUniverse::new(cells);

    // --- Keyboard state ---
    let mut input_manager = InputManager::new();

    // --- Animation loop ---
    let f = Rc::new(RefCell::new(None::<Closure<dyn FnMut()>>));
    let g = f.clone();

    *g.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        let input = input_manager.update();

        let mut dx = 0;
        let mut dy = 0;
        if input.pressed.contains("k")
            || input.pressed.contains("w")
            || input.pressed.contains("ArrowUp")
        {
            dy -= 1;
        }
        if input.pressed.contains("j")
            || input.pressed.contains("s")
            || input.pressed.contains("ArrowDown")
        {
            dy += 1;
        }
        if input.pressed.contains("h")
            || input.pressed.contains("a")
            || input.pressed.contains("ArrowLeft")
        {
            dx -= 1;
        }
        if input.pressed.contains("l")
            || input.pressed.contains("d")
            || input.pressed.contains("ArrowRight")
        {
            dx += 1;
        }
        if input.just_pressed.contains("z") {
            universe.zoom_in();
        }
        if input.just_pressed.contains("x") {
            universe.zoom_out();
        }
        universe.pan(dx, dy);

        // Simulation step
        universe.tick();

        // HUD
        let text = format!(
            "Camera: ({}, {}) | Cells: {}",
            universe.camera_x(),
            universe.camera_y(),
            universe.live_cells().len()
        );
        hud_element.set_text_content(Some(&text));

        // Render
        renderer::render(&universe, &context, width, height);

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
