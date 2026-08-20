mod camera;
mod config;
mod input;
mod pattern;
mod renderer;
mod universe;

use crate::camera::Camera;
use crate::input::InputManager;
use crate::universe::{SparseUniverse, Universe};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::window;

/// Owns every piece of per-frame state: the simulation, renderer, input
/// tracking, HUD/pause/speed state, and the currently-loading pattern. The
/// `requestAnimationFrame` closure in `main` only calls `App::frame`; all
/// the actual per-frame logic lives in the methods below.
struct App {
    universe: Rc<RefCell<Option<SparseUniverse>>>,
    camera: Camera,
    current_pattern: Rc<RefCell<Option<String>>>,
    renderer: renderer::Renderer,
    render_buffer: Vec<u8>,
    context: web_sys::CanvasRenderingContext2d,
    hud_element: web_sys::HtmlElement,
    controls_element: web_sys::HtmlElement,
    performance: web_sys::Performance,
    input_manager: InputManager,
    paused: bool,
    hud_visible: bool,
    ticks_per_second: f64,
    tick_accumulator_ms: f64,
    last_tick_time: f64,
    pan_accum_x_px: f64,
    pan_accum_y_px: f64,
    hud_last_text: String,
    hud_last_update_time: f64,
    hud_last_paused: bool,
}

impl App {
    fn new(
        context: web_sys::CanvasRenderingContext2d,
        hud_element: web_sys::HtmlElement,
        controls_element: web_sys::HtmlElement,
        performance: web_sys::Performance,
        width: u32,
        height: u32,
    ) -> Self {
        let last_tick_time = performance.now();
        Self {
            universe: Rc::new(RefCell::new(None)),
            camera: Camera::new(),
            current_pattern: Rc::new(RefCell::new(None)),
            renderer: renderer::Renderer::new(width, height),
            render_buffer: vec![0u8; (width * height * 4) as usize],
            context,
            hud_element,
            controls_element,
            performance,
            input_manager: InputManager::new(),
            paused: false,
            hud_visible: true,
            tick_accumulator_ms: 0.0,
            last_tick_time,
            ticks_per_second: config::DEFAULT_TICKS_PER_SECOND,
            pan_accum_x_px: 0.0,
            pan_accum_y_px: 0.0,
            hud_last_text: String::new(),
            hud_last_update_time: f64::NEG_INFINITY,
            hud_last_paused: false,
        }
    }

    /// Reads keyboard state and applies every input that isn't simulation
    /// ticking or panning (those need frame delta-time, handled in
    /// `update`). Returns the requested pan direction plus whether a
    /// pattern reload was requested this frame.
    fn handle_input(&mut self) -> (i32, i32, bool) {
        let input = self.input_manager.update();

        let mut dx = 0;
        let mut dy = 0;
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
            self.camera.zoom_in();
        }
        if input.is_just_pressed(input::KeyState::X) {
            self.camera.zoom_out();
        }
        if input.is_just_pressed(input::KeyState::P) {
            self.paused = !self.paused;
        }
        if input.is_just_pressed(input::KeyState::O) {
            self.ticks_per_second = (self.ticks_per_second * 2.0).min(config::MAX_TICKS_PER_SECOND);
        }
        if input.is_just_pressed(input::KeyState::I) {
            self.ticks_per_second = (self.ticks_per_second / 2.0).max(config::MIN_TICKS_PER_SECOND);
        }
        if input.is_just_pressed(input::KeyState::T) {
            self.hud_visible = !self.hud_visible;
            let display = if self.hud_visible { "block" } else { "none" };
            self.hud_element
                .style()
                .set_property("display", display)
                .ok();
            self.controls_element
                .style()
                .set_property("display", display)
                .ok();
        }
        if self.paused
            && input.is_just_pressed(input::KeyState::STEP_FORWARD)
            && let Some(ref mut u) = *self.universe.borrow_mut()
        {
            u.tick();
        }
        if self.paused
            && input.is_just_pressed(input::KeyState::STEP_BACK)
            && let Some(ref mut u) = *self.universe.borrow_mut()
        {
            u.step_back();
        }

        let reload_requested = input.is_just_pressed(input::KeyState::R);
        (dx, dy, reload_requested)
    }

    /// Advances the simulation by however many ticks are due this frame,
    /// applies panning, and kicks off a pattern reload if requested.
    fn update(&mut self, dx: i32, dy: i32, reload_requested: bool) {
        let now = self.performance.now();
        let delta_ms = now - self.last_tick_time;
        self.last_tick_time = now;

        if !self.paused {
            self.tick_accumulator_ms += delta_ms;
            let tick_interval_ms = 1000.0 / self.ticks_per_second;
            let mut ticks_this_frame = 0u32;
            if let Some(ref mut u) = *self.universe.borrow_mut() {
                while self.tick_accumulator_ms >= tick_interval_ms
                    && ticks_this_frame < config::MAX_TICKS_PER_FRAME
                {
                    u.tick();
                    self.tick_accumulator_ms -= tick_interval_ms;
                    ticks_this_frame += 1;
                }
            }
            if self.tick_accumulator_ms > tick_interval_ms * config::MAX_TICKS_PER_FRAME as f64 {
                self.tick_accumulator_ms = 0.0;
            }
        } else {
            self.tick_accumulator_ms = 0.0;
        }

        if reload_requested {
            self.load_random_pattern();
        }

        let ps = renderer::pixel_size_for_scale(self.camera.scale()) as f64;
        let px_to_move = config::PAN_SPEED_PX_PER_SEC * (delta_ms / 1000.0);

        if dx != 0 {
            self.pan_accum_x_px += px_to_move * dx as f64;
        } else {
            self.pan_accum_x_px = 0.0;
        }
        if dy != 0 {
            self.pan_accum_y_px += px_to_move * dy as f64;
        } else {
            self.pan_accum_y_px = 0.0;
        }

        let cells_x = (self.pan_accum_x_px / ps).trunc() as i32;
        let cells_y = (self.pan_accum_y_px / ps).trunc() as i32;

        self.pan_accum_x_px -= cells_x as f64 * ps;
        self.pan_accum_y_px -= cells_y as f64 * ps;

        if cells_x != 0 || cells_y != 0 {
            self.camera.pan(cells_x, cells_y);
        }
    }

    /// Rebuilds and writes the HUD text, if visible. (Still rebuilds every
    /// frame regardless of whether anything changed — see roadmap item 5.)
    fn update_hud(&mut self) {
        if !self.hud_visible {
            return;
        }

        let now = self.performance.now();
        let pause_changed = self.paused != self.hud_last_paused;
        let due = now - self.hud_last_update_time >= config::HUD_UPDATE_INTERVAL_MS;

        if !pause_changed && !due {
            return;
        }

        let (cell_count, generation, replaying) = if let Some(ref u) = *self.universe.borrow() {
            (u.live_cells().len(), u.generation(), u.is_replaying())
        } else {
            (0, 0, false)
        };
        let cam_x = self.camera.x();
        let cam_y = self.camera.y();
        let scale = self.camera.scale();

        let pattern_name = self
            .current_pattern
            .borrow()
            .clone()
            .unwrap_or_else(|| "None".to_string());
        let paused_text = if self.paused {
            "⏸ Paused"
        } else {
            "▶ Running"
        };
        let speed_text = if self.ticks_per_second >= 1.0 {
            format!("{:.2} ticks/s", self.ticks_per_second)
        } else {
            format!("1 tick/{:.1}s", 1.0 / self.ticks_per_second)
        };
        let history_text = if replaying { " rewound" } else { "" };
        let text = format!(
            "Current Pattern = {} | Camera: ({}, {}) | Zoom: {}x | Cells: {} | Gen: {}{} | {} | Speed: {}",
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

        if text != self.hud_last_text {
            self.hud_element.set_text_content(Some(&text));
            self.hud_last_text = text;
        }
        self.hud_last_update_time = now;
        self.hud_last_paused = self.paused;
    }

    /// Paints the current simulation state, if a universe has finished loading.
    fn render(&mut self) {
        if let Some(ref u) = *self.universe.borrow() {
            self.renderer
                .render(u, &self.camera, &self.context, &mut self.render_buffer);
        }
    }

    /// Kicks off an async fetch + parse of a random pattern from the
    /// library, swapping it in once it's ready.
    fn load_random_pattern(&mut self) {
        self.camera = Camera::new();

        let universe = self.universe.clone();
        let current_pattern = self.current_pattern.clone();
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

    /// Runs one full frame: input -> simulation update -> HUD -> render.
    fn frame(&mut self) {
        let (dx, dy, reload_requested) = self.handle_input();
        self.update(dx, dy, reload_requested);
        self.update_hud();
        self.render();
    }
}

fn main() {
    console_error_panic_hook::set_once();

    let document = window().unwrap().document().unwrap();
    let canvas = document
        .get_element_by_id(config::CANVAS_ELEMENT_ID)
        .unwrap()
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .unwrap();
    let context_options = js_sys::Object::new();
    js_sys::Reflect::set(&context_options, &"alpha".into(), &false.into()).unwrap();
    let context = canvas
        .get_context_with_context_options(config::CANVAS_CONTEXT_ID, &context_options)
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .unwrap();
    let hud_element = document
        .get_element_by_id(config::HUD_ELEMENT_ID)
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap();
    let controls_element = document
        .get_element_by_id(config::CONTROLS_ELEMENT_ID)
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap();
    let performance = window().unwrap().performance().unwrap();

    let width = canvas.width();
    let height = canvas.height();

    let app = Rc::new(RefCell::new(App::new(
        context,
        hud_element,
        controls_element,
        performance,
        width,
        height,
    )));

    app.borrow_mut().load_random_pattern();

    // --- Animation loop ---
    let f = Rc::new(RefCell::new(None::<Closure<dyn FnMut()>>));
    let g = f.clone();

    let app_for_closure = app.clone();
    *g.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        app_for_closure.borrow_mut().frame();

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
