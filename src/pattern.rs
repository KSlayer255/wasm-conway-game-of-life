use crate::universe::Cell;
use rustc_hash::FxHashSet;

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, Response, console, window}; // You'll need to add 'rand' crate
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
    let url = format!("/patterns/{}", filename);
    console::log_1(&format!("file location: {}", url).into());
    let win = window().unwrap();
    let opts = RequestInit::new();
    opts.set_method("GET");
    let request = Request::new_with_str_and_init(&url, &opts)?;
    console::log_1(&format!("Request: {:?}", request.text()).into());
    let resp = JsFuture::from(win.fetch_with_request(&request)).await?;
    let resp: Response = resp.dyn_into()?;
    // --- Debug: Check status ---
    let status = resp.status();
    console::log_1(&format!("Response status: {}", status).into());
    if !resp.ok() {
        let err_msg = format!("HTTP error: {}", status);
        console::error_1(&err_msg.clone().into());
        return Err(JsValue::from_str(&err_msg));
    }

    // --- Read the response body as text ---
    let text_promise = resp.text()?;
    let text = JsFuture::from(text_promise).await?;
    let text_str = text.as_string().unwrap_or_default();

    // --- Log a preview of the content ---
    let preview = if text_str.len() > 100 {
        format!("{}...", &text_str[..100])
    } else {
        text_str.clone()
    };
    console::log_1(&format!("Response preview: {}", preview).into());
    console::log_1(&format!("Total length: {} bytes", text_str.len()).into());
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

pub fn centre_cells(
    cells: &FxHashSet<Cell>,
    viewport_width: u32,
    viewport_height: u32,
) -> FxHashSet<Cell> {
    if cells.is_empty() {
        return cells.clone();
    }

    let (min_x, max_x, min_y, max_y) = cells.iter().fold(
        (i32::MAX, i32::MIN, i32::MAX, i32::MIN),
        |(mnx, mxx, mny, mxy), &(x, y)| (mnx.min(x), mxx.max(x), mny.min(y), mxy.max(y)),
    );
    let pattern_w = max_x - min_x + 1;
    let pattern_h = max_y - min_y + 1;

    let shift_x = (viewport_width as i32 - pattern_w) / 2 - min_x;
    let shift_y = (viewport_height as i32 - pattern_h) / 2 - min_y;

    cells
        .iter()
        .map(|&(x, y)| (x + shift_x, y + shift_y))
        .collect()
}
