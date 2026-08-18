//! Central location for tunable constants, the color palette, DOM element
//! ids, and other hard-coded values shared across the crate. Grouped by
//! concern so values that multiple files need to agree on (e.g. the zoom
//! bounds used by both `universe::sparse` and `renderer`) live in one place
//! instead of drifting apart.

// --- Color palette (R, G, B). Tweak these to change the wallpaper look. ---
pub const BG_COLOR: [u8; 3] = [10, 12, 22]; // deep navy background
pub const GRID_COLOR: [u8; 3] = [26, 30, 46]; // faint grid lines, just above bg
pub const CELL_COLOR: [u8; 3] = [175, 220, 255]; // soft cyan-white live cells

// --- Simulation history / zoom ---
/// Number of past generations kept for step-back, capped so memory use
/// stays bounded on long-running (wallpaper) sessions.
pub const MAX_HISTORY: usize = 256;
/// Zoom range, expressed as the `scale` exponent: pixel size per world cell
/// is `1 << -scale`, so `0` is 1:1 and more negative means more zoomed in.
pub const MIN_SCALE: i32 = -8;
pub const MAX_SCALE: i32 = 0;
pub const INITIAL_SCALE: i32 = -4;

// --- Simulation speed ---
pub const DEFAULT_TICKS_PER_SECOND: f64 = 8.0;
pub const MIN_TICKS_PER_SECOND: f64 = 1.0 / 32.0; // as slow as 1 tick per 32s
pub const MAX_TICKS_PER_SECOND: f64 = 4096.0;
/// Safety cap so a backgrounded tab catching back up doesn't try to replay
/// an enormous backlog of ticks in a single animation frame.
pub const MAX_TICKS_PER_FRAME: u32 = 1024;

// --- DOM ---
pub const CANVAS_ELEMENT_ID: &str = "canvas";
pub const HUD_ELEMENT_ID: &str = "hud";
pub const CONTROLS_ELEMENT_ID: &str = "controls";
pub const CANVAS_CONTEXT_ID: &str = "2d";

// --- Pattern loading ---
/// Root the runtime `fetch` requests are made relative to. Must match the
/// `data-target-path` that `index.html`'s `copy-dir` directive copies
/// `patterns/conwaylife/oscillators` into. This can't be unified with
/// `build.rs`'s source-directory path into one shared constant: `build.rs`
/// is compiled and run as a separate program before this crate exists, so
/// it has no access to `crate::config`.
pub const PATTERNS_FETCH_ROOT: &str = "patterns";
