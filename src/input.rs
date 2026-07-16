use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use web_sys::{KeyboardEvent, window};

#[derive(Clone, Default)]
pub struct InputState {
    pub pressed: HashSet<String>,
    pub just_pressed: HashSet<String>,
}

pub struct InputManager {
    state: Rc<RefCell<InputState>>,
    _keydown: Closure<dyn FnMut(KeyboardEvent)>,
    _keyup: Closure<dyn FnMut(KeyboardEvent)>,
}

impl InputManager {
    pub fn new() -> Self {
        let state = Rc::new(RefCell::new(InputState::default()));

        // --- Keydown ---
        let state_down = state.clone();
        let keydown = Closure::wrap(Box::new(move |ev: KeyboardEvent| {
            let key = ev.key();
            let mut s = state_down.borrow_mut();
            // Only insert into just_pressed if it wasn't already pressed.
            if !s.pressed.contains(&key) {
                s.pressed.insert(key.clone());
                s.just_pressed.insert(key);
            }
        }) as Box<dyn FnMut(KeyboardEvent)>);

        // --- Keyup ---
        let state_up = state.clone();
        let keyup = Closure::wrap(Box::new(move |ev: KeyboardEvent| {
            let key = ev.key();
            let mut s = state_up.borrow_mut();
            s.pressed.remove(&key);
            // just_pressed is NOT removed here; it will be cleared on update.
        }) as Box<dyn FnMut(KeyboardEvent)>);

        // Attach listeners
        let win = window().unwrap();
        win.add_event_listener_with_callback("keydown", keydown.as_ref().unchecked_ref())
            .unwrap();
        win.add_event_listener_with_callback("keyup", keyup.as_ref().unchecked_ref())
            .unwrap();

        InputManager {
            state,
            _keydown: keydown,
            _keyup: keyup,
        }
    }

    /// Call this once per frame to get the current input state and clear the
    /// `just_pressed` set for the next frame.
    pub fn update(&mut self) -> InputState {
        let mut s = self.state.borrow_mut();
        let just_pressed = std::mem::take(&mut s.just_pressed);
        InputState {
            pressed: s.pressed.clone(),
            just_pressed,
        }
    }
}
