use crate::universe::Cell;
use rustc_hash::FxHashSet;

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
