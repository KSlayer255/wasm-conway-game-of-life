//! Central location for tunable constants, the color palette, DOM element
//! ids, and other hard-coded values shared across the crate. Grouped by
//! concern so values that multiple files need to agree on (e.g. the zoom
//! bounds used by both `universe::sparse` and `renderer`) live in one place
//! instead of drifting apart.

// --- Color palette (R, G, B). Tweak these to change the wallpaper look. ---
pub const BG_COLOR: [u8; 3] = [10, 12, 22]; // deep navy background
pub const GRID_COLOR: [u8; 3] = [26, 30, 46]; // faint grid lines, just above bg

// --- Cell age gradient (HSL) ---
// Interpolated by hue rather than by RGB channel, so the gradient sweeps
// green -> yellow -> orange -> red instead of cutting through a muddy gray
// midpoint the way a straight RGB lerp between two saturated colors would.
/// Hue of a cell the instant it's born.
pub const CELL_HUE_YOUNG_DEG: f32 = 120.0; // green
/// Hue of the oldest cell currently alive, relative to the rest of the live
/// population (see `AGE_COLOR_REFRESH_INTERVAL_MS`) - not an absolute age
/// threshold. Also the fallback for any `Universe` impl that doesn't track
/// age: `age_bounds()` defaulting to `(0, 0)` collapses every cell to the
/// young hue, so this only matters once age tracking is wired up.
pub const CELL_HUE_OLD_DEG: f32 = 0.0; // red
pub const CELL_SATURATION: f32 = 0.65;
pub const CELL_LIGHTNESS: f32 = 0.55;

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

// --- Camera panning ---
/// Camera pan speed in screen pixels per second. Kept constant regardless
/// of zoom level (converted to world cells using the current pixel size)
/// and regardless of refresh rate (converted using measured frame delta
/// time) - see `App::update` in main.rs.
pub const PAN_SPEED_PX_PER_SEC: f64 = 600.0;

// --- HUD ---
/// Minimum time between HUD text rebuilds/writes, in milliseconds. ~10Hz -
/// frequent enough that generation/cell counts still feel live, infrequent
/// enough to avoid a `set_text_content` call every animation frame. State
/// changes that should feel instant (e.g. pause/run) bypass this throttle -
/// see `App::update_hud` in main.rs.
pub const HUD_UPDATE_INTERVAL_MS: f64 = 100.0;

// --- Cell age coloring ---
/// How often (wall-clock ms) the renderer recomputes the live population's
/// (min, max) age and does a full repaint against it. Deliberately
/// decoupled from tick rate (which can run from well under 1/s to
/// thousands/s) and from per-cell change detection, so aging never costs
/// more than a periodic full redraw - the same cost class as a pan/zoom.
pub const AGE_COLOR_REFRESH_INTERVAL_MS: f64 = 400.0;

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
