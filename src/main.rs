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
use wasm_bindgen_futures::spawn_local;
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
    let controls_element = document
        .get_element_by_id("controls")
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap();
    let performance = window().unwrap().performance().unwrap();

    let width = canvas.width();
    let height = canvas.height();

    let mut render_buffer = vec![0u8; (width * height * 4) as usize];
    let mut renderer = renderer::Renderer::new(width, height);

    // Load the embedded pattern and centre it.
    let universe = Rc::new(RefCell::new(None::<universe::SparseUniverse>));
    let current_pattern = Rc::new(RefCell::new(None::<String>));

    let load_random_pattern = {
        let universe = universe.clone();
        let current_pattern = current_pattern.clone();
        move || {
            let universe = universe.clone();
            let current_pattern = current_pattern.clone();
            spawn_local(async move {
                let pattern_name = pattern::random_pattern_name();
                let content = pattern::fetch_pattern(pattern_name).await.unwrap();
                let mut cells = pattern::load_pattern_from_str(&content);
                cells = pattern::centre_cells(&cells);
                let new_universe = universe::SparseUniverse::new(cells);
                *universe.borrow_mut() = Some(new_universe);
                *current_pattern.borrow_mut() = Some(pattern_name.to_string());
            });
        }
    };

    load_random_pattern();

    // --- Keyboard state ---
    let input_manager = InputManager::new();

    let mut paused = false;
    let mut hud_visible = true;

    const DEFAULT_TICKS_PER_SECOND: f64 = 8.0;
    const MIN_TICKS_PER_SECOND: f64 = 1.0 / 32.0; // as slow as 1 tick per 32s
    const MAX_TICKS_PER_SECOND: f64 = 4096.0;
    // Safety cap so a backgrounded tab catching back up doesn't try to
    // replay an enormous backlog of ticks in a single frame.
    const MAX_TICKS_PER_FRAME: u32 = 1024;

    let mut ticks_per_second = DEFAULT_TICKS_PER_SECOND;
    let mut tick_accumulator_ms: f64 = 0.0;
    let mut last_tick_time = performance.now();

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
        if input.is_just_pressed(input::KeyState::Z)
            && let Some(ref mut u) = *universe.borrow_mut()
        {
            u.zoom_in();
        }
        if input.is_just_pressed(input::KeyState::X)
            && let Some(ref mut u) = *universe.borrow_mut()
        {
            u.zoom_out();
        }
        if input.is_just_pressed(input::KeyState::P) {
            paused = !paused;
        }
        if input.is_just_pressed(input::KeyState::O) {
            ticks_per_second = (ticks_per_second * 2.0).min(MAX_TICKS_PER_SECOND);
        }
        if input.is_just_pressed(input::KeyState::I) {
            ticks_per_second = (ticks_per_second / 2.0).max(MIN_TICKS_PER_SECOND);
        }

        if input.is_just_pressed(input::KeyState::T) {
            hud_visible = !hud_visible;
            let display = if hud_visible { "block" } else { "none" };
            hud_element.style().set_property("display", display).ok();
            controls_element
                .style()
                .set_property("display", display)
                .ok();
        }

        if paused
            && input.is_just_pressed(input::KeyState::STEP_FORWARD)
            && let Some(ref mut u) = *universe.borrow_mut()
        {
            u.tick();
        }

        if paused
            && input.is_just_pressed(input::KeyState::STEP_BACK)
            && let Some(ref mut u) = *universe.borrow_mut()
        {
            u.step_back();
        }

        let now = performance.now();
        let delta_ms = now - last_tick_time;
        last_tick_time = now;

        if !paused {
            tick_accumulator_ms += delta_ms;
            let tick_interval_ms = 1000.0 / ticks_per_second;
            let mut ticks_this_frame = 0u32;
            if let Some(ref mut u) = *universe.borrow_mut() {
                while tick_accumulator_ms >= tick_interval_ms
                    && ticks_this_frame < MAX_TICKS_PER_FRAME
                {
                    u.tick();
                    tick_accumulator_ms -= tick_interval_ms;
                    ticks_this_frame += 1
                }
            }
            if tick_accumulator_ms > tick_interval_ms * MAX_TICKS_PER_FRAME as f64 {
                tick_accumulator_ms = 0.0;
            }
        } else {
            tick_accumulator_ms = 0.0;
        }

        if input.is_just_pressed(input::KeyState::R) {
            load_random_pattern();
        }

        if let Some(ref mut u) = *universe.borrow_mut() {
            u.pan(dx, dy);
        }

        // HUD
        if hud_visible {
            let (cam_x, cam_y, cell_count, scale, generation, replaying) =
                if let Some(ref u) = *universe.borrow() {
                    (
                        u.camera_x(),
                        u.camera_y(),
                        u.live_cells().len(),
                        u.scale(),
                        u.generation(),
                        u.is_replaying(),
                    )
                } else {
                    (0, 0, 0, 0, 0, false)
                };
            let pattern_name = current_pattern
                .borrow()
                .clone()
                .unwrap_or_else(|| "None".to_string());
            let paused_text = if paused { "⏸ Paused" } else { "▶ Running" };
            let speed_text = if ticks_per_second >= 1.0 {
                format!("{:.2} ticks/s", ticks_per_second)
            } else {
                format!("1 tick/{:.1}s", 1.0 / ticks_per_second)
            };
            let history_text = if replaying { " rewound" } else { "" };
            let text = format!(
                "Current Pattern = {} | Camera: ({}, {}) | Zoom: {}x | Cells: {} | Gen: {}{} | {} | Speed: {}x",
                pattern_name,
                cam_x,
                cam_y,
                1 << (-scale),
                cell_count,
                generation,
                history_text,
                paused_text,
                speed_text
            );
            hud_element.set_text_content(Some(&text));
        }

        // --- Render ---
        if let Some(ref u) = *universe.borrow() {
            renderer.render(u, &context, &mut render_buffer);
        }

        // --- Next frame ---
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
