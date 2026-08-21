use crate::config::PATTERNS_FETCH_ROOT;
use crate::universe::Cell;
use rustc_hash::FxHashSet;

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, Response, window}; // You'll need to add 'rand' crate
//
// Include the generated list of pattern filenames
include!(concat!(env!("OUT_DIR"), "/pattern_list.rs"));

pub fn random_pattern_name() -> &'static str {
    let len = PATTERN_FILES.len() as f64;
    // js_sys::Math::random() returns f64 in [0, 1)
    let idx = (js_sys::Math::random() * len) as usize;
    PATTERN_FILES[idx]
}

pub async fn fetch_pattern(filename: &str) -> Result<String, JsValue> {
    let url = format!("{}/{}", PATTERNS_FETCH_ROOT, filename);
    let win = window().unwrap();
    let opts = RequestInit::new();
    opts.set_method("GET");
    let request = Request::new_with_str_and_init(&url, &opts)?;
    let resp = JsFuture::from(win.fetch_with_request(&request)).await?;
    let resp: Response = resp.dyn_into()?;
    // --- Debug: Check status ---
    let status = resp.status();
    if !resp.ok() {
        let err_msg = format!("HTTP error: {}", status);
        return Err(JsValue::from_str(&err_msg));
    }
    // --- Read the response body as text ---
    let text_promise = resp.text()?;
    let text = JsFuture::from(text_promise).await?;
    let text_str = text.as_string().unwrap_or_default();
    Ok(text_str)
}

pub fn load_pattern_from_str(contents: &str) -> FxHashSet<Cell> {
    let mut cells = Vec::new();
    let mut x = 0;
    let mut y = 0;

    if contents.contains("x =") || contents.contains('!') {
        // RLE format
        let mut count = String::new();
        let pattern_data: String = contents
            .lines()
            .filter(|line| !(line.starts_with('#') || line.starts_with("x = ")))
            .map(|line| line.trim())
            .collect::<String>();

        for ch in pattern_data.chars() {
            match ch {
                '0'..='9' => count.push(ch),
                'b' => {
                    let repeat = count.parse().unwrap_or(1);
                    x += repeat;
                    count.clear();
                }
                'o' => {
                    let repeat = count.parse().unwrap_or(1);
                    for _ in 0..repeat {
                        cells.push((x, y));
                        x += 1;
                    }
                    count.clear();
                }
                '$' => {
                    y += count.parse().unwrap_or(1);
                    x = 0;
                    count.clear();
                }
                '!' => break,
                _ => {}
            }
        }
    } else {
        // Plain text grid (e.g., .txt)
        for line in contents.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('!') {
                continue;
            }
            for (x_pos, ch) in line.chars().enumerate() {
                match ch {
                    'O' | '*' => cells.push((x_pos as i32, y)),
                    _ => {}
                }
            }
            y += 1;
        }
    }
    cells.into_iter().collect()
}

pub fn centre_cells(cells: &FxHashSet<Cell>) -> FxHashSet<Cell> {
    if cells.is_empty() {
        return cells.clone();
    }

    let (min_x, max_x, min_y, max_y) = cells.iter().fold(
        (i32::MAX, i32::MIN, i32::MAX, i32::MIN),
        |(mnx, mxx, mny, mxy), &(x, y)| (mnx.min(x), mxx.max(x), mny.min(y), mxy.max(y)),
    );

    let shift_x = -(min_x + max_x) / 2;
    let shift_y = -(min_y + max_y) / 2;

    cells
        .iter()
        .map(|&(x, y)| (x + shift_x, y + shift_y))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(cells: &[(i32, i32)]) -> FxHashSet<Cell> {
        cells.iter().copied().collect()
    }

    // --- load_pattern_from_str: RLE ---

    #[test]
    fn parses_rle_glider() {
        let rle = "x = 3, y = 3, rule = B3/S23\nbob$2bo$3o!";
        let cells = load_pattern_from_str(rle);
        assert_eq!(cells, set(&[(1, 0), (2, 1), (0, 2), (1, 2), (2, 2)]));
    }

    #[test]
    fn rle_header_and_comment_lines_are_ignored() {
        let rle = "#C A comment about the pattern\nx = 2, y = 1, rule = B3/S23\n2o!";
        let cells = load_pattern_from_str(rle);
        assert_eq!(cells, set(&[(0, 0), (1, 0)]));
    }

    #[test]
    fn rle_without_trailing_bang_does_not_panic() {
        let rle = "x = 1, y = 1\n3o";
        let cells = load_pattern_from_str(rle);
        assert_eq!(cells, set(&[(0, 0), (1, 0), (2, 0)]));
    }

    #[test]
    fn rle_unrecognized_characters_are_ignored_not_fatal() {
        let rle = "x = 1, y = 1\nzzz3o!";
        let cells = load_pattern_from_str(rle);
        assert_eq!(cells, set(&[(0, 0), (1, 0), (2, 0)]));
    }

    // --- load_pattern_from_str: plaintext ---

    #[test]
    fn parses_plaintext_blinker_no_comment_line() {
        // No '!' anywhere in this input, so it correctly takes the
        // plaintext branch - see the note on the format-detection heuristic
        // in the PR/chat notes for input that *does* contain '!'.
        let plaintext = ".O.\n.O.\n.O.";
        let cells = load_pattern_from_str(plaintext);
        assert_eq!(cells, set(&[(1, 0), (1, 1), (1, 2)]));
    }

    #[test]
    fn plaintext_accepts_star_as_well_as_o() {
        let plaintext = "*.\n.*";
        let cells = load_pattern_from_str(plaintext);
        assert_eq!(cells, set(&[(0, 0), (1, 1)]));
    }

    #[test]
    fn plaintext_blank_lines_are_skipped_not_counted_as_rows() {
        let plaintext = "O.\n\n.O";
        let cells = load_pattern_from_str(plaintext);
        assert_eq!(cells, set(&[(0, 0), (1, 1)]));
    }

    // --- load_pattern_from_str: edge cases ---

    #[test]
    fn empty_input_produces_no_cells() {
        assert_eq!(load_pattern_from_str(""), FxHashSet::default());
    }

    #[test]
    fn whitespace_only_input_produces_no_cells() {
        assert_eq!(load_pattern_from_str("   \n  \n"), FxHashSet::default());
    }

    // --- centre_cells ---

    #[test]
    fn centre_cells_empty_set_stays_empty() {
        let empty: FxHashSet<Cell> = FxHashSet::default();
        assert_eq!(centre_cells(&empty), empty);
    }

    #[test]
    fn centre_cells_single_cell_moves_to_origin() {
        let cells = set(&[(5, 5)]);
        assert_eq!(centre_cells(&cells), set(&[(0, 0)]));
    }

    #[test]
    fn centre_cells_odd_width_and_height_centers_symmetrically() {
        // x spans 0..=2 (odd width 3), y spans 0..=2 (odd height 3).
        let cells = set(&[(0, 0), (2, 0), (0, 2), (2, 2)]);
        let centred = centre_cells(&cells);
        assert_eq!(centred, set(&[(-1, -1), (1, -1), (-1, 1), (1, 1)]));
    }

    #[test]
    fn centre_cells_even_width_and_height_rounds_toward_zero() {
        // x spans 0..=1 (even width 2), y spans 0..=1 (even height 2).
        // -(0 + 1) / 2 == 0 under Rust's truncating integer division, so
        // this shape doesn't shift at all - perfect symmetry isn't
        // possible for an even span around a single integer center, so
        // this documents the actual (toward-zero) behavior rather than
        // asserting an idealized one.
        let cells = set(&[(0, 0), (1, 0), (0, 1), (1, 1)]);
        assert_eq!(centre_cells(&cells), cells);
    }
}
