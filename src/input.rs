use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use web_sys::{KeyboardEvent, window};

/// Bit flags for all keys we care about.
#[derive(Clone, Copy, Default)]
pub struct KeyState {
    pub pressed: u32,
    pub just_pressed: u32,
}

impl KeyState {
    pub const K: u32 = 1 << 0;
    pub const H: u32 = 1 << 1;
    pub const J: u32 = 1 << 2;
    pub const L: u32 = 1 << 3;
    pub const UP: u32 = 1 << 4;
    pub const DOWN: u32 = 1 << 5;
    pub const LEFT: u32 = 1 << 6;
    pub const RIGHT: u32 = 1 << 7;
    pub const Z: u32 = 1 << 8;
    pub const X: u32 = 1 << 9;
    pub const P: u32 = 1 << 10; // Pause
    pub const O: u32 = 1 << 11; // Speed up (o for faster)
    pub const I: u32 = 1 << 12; // Speed down (i for slower)
    pub const R: u32 = 1 << 13; // Reload pattern
    pub const T: u32 = 1 << 14; // Toggle HUD
    pub const STEP_BACK: u32 = 1 << 15; // ',' step back one generation while paused
    pub const STEP_FORWARD: u32 = 1 << 16; // '.' step forward one generation while paused

    pub fn is_pressed(&self, key: u32) -> bool {
        self.pressed & key != 0
    }

    pub fn is_just_pressed(&self, key: u32) -> bool {
        self.just_pressed & key != 0
    }
}

/// Manages keyboard input, updates state, and clears `just_pressed` each frame.
pub struct InputManager {
    state: Rc<RefCell<KeyState>>,
    _keydown: Closure<dyn FnMut(KeyboardEvent)>,
    _keyup: Closure<dyn FnMut(KeyboardEvent)>,
}

impl InputManager {
    pub fn new() -> Self {
        // Shared state with interior mutability
        let state = Rc::new(RefCell::new(KeyState::default()));

        // --- Keydown ---
        let state_down = state.clone();
        let keydown = Closure::wrap(Box::new(move |ev: KeyboardEvent| {
            let mut s = state_down.borrow_mut();
            let bit = Self::key_to_bit(&ev.key());
            if bit != 0 && !s.is_pressed(bit) {
                s.pressed |= bit;
                s.just_pressed |= bit;
            }
        }) as Box<dyn FnMut(KeyboardEvent)>);

        // --- Keyup ---
        let state_up = state.clone();
        let keyup = Closure::wrap(Box::new(move |ev: KeyboardEvent| {
            let mut s = state_up.borrow_mut();
            let bit = Self::key_to_bit(&ev.key());
            if bit != 0 {
                s.pressed &= !bit;
                // just_pressed is NOT cleared here; it will be cleared in update()
            }
        }) as Box<dyn FnMut(KeyboardEvent)>);

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

    /// Call once per frame to get the current input and clear `just_pressed`.
    pub fn update(&self) -> KeyState {
        let mut s = self.state.borrow_mut();
        let just = s.just_pressed;
        s.just_pressed = 0; // clear for next frame
        KeyState {
            pressed: s.pressed,
            just_pressed: just,
        }
    }

    fn key_to_bit(key: &str) -> u32 {
        match key {
            "k" | "K" => KeyState::K,
            "h" | "H" => KeyState::H,
            "j" | "J" => KeyState::J,
            "l" | "L" => KeyState::L,
            "ArrowUp" => KeyState::UP,
            "ArrowDown" => KeyState::DOWN,
            "ArrowLeft" => KeyState::LEFT,
            "ArrowRight" => KeyState::RIGHT,
            "z" | "Z" => KeyState::Z,
            "x" | "X" => KeyState::X,
            "p" | "P" => KeyState::P,
            "o" | "O" => KeyState::O,
            "i" | "I" => KeyState::I,
            "r" | "R" => KeyState::R,
            "t" | "T" => KeyState::T,
            "," => KeyState::STEP_BACK,
            "." => KeyState::STEP_FORWARD,
            _ => 0,
        }
    }
}
