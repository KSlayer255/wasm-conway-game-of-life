use crate::config::MAX_HISTORY;
use crate::universe::Cell;
use crate::universe::Universe;
use rustc_hash::FxHashSet;
use std::collections::VecDeque;

pub struct SparseUniverse {
    history: VecDeque<FxHashSet<Cell>>,
    history_start_generation: u64,
    cursor: usize,
}

impl SparseUniverse {
    pub fn new(cells: FxHashSet<Cell>) -> Self {
        let mut history = VecDeque::with_capacity(MAX_HISTORY);
        history.push_back(cells);
        Self {
            history,
            history_start_generation: 0,
            cursor: 0,
        }
    }
}

impl Universe for SparseUniverse {
    fn tick(&mut self) {
        if self.cursor + 1 < self.history.len() {
            self.cursor += 1;
            return;
        }

        let next = step(&self.history[self.cursor]);
        self.history.push_back(next);

        if self.history.len() > MAX_HISTORY {
            self.history.pop_front();
            self.history_start_generation += 1;
        } else {
            self.cursor += 1;
        }
    }

    fn live_cells(&self) -> &FxHashSet<Cell> {
        &self.history[self.cursor]
    }

    fn step_back(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    fn is_replaying(&self) -> bool {
        self.cursor + 1 < self.history.len()
    }

    fn generation(&self) -> u64 {
        self.history_start_generation + self.cursor as u64
    }
}

// ====== Private helper functions (Game of Life rules) ======
fn step(live_cells: &FxHashSet<Cell>) -> FxHashSet<Cell> {
    let mut candidates = FxHashSet::default();
    for &(x, y) in live_cells {
        for dx in -1..=1 {
            for dy in -1..=1 {
                candidates.insert((x + dx, y + dy));
            }
        }
    }

    let mut next = FxHashSet::default();
    for &(x, y) in &candidates {
        let neighbors = count_neighbors(live_cells, x, y);
        let alive = live_cells.contains(&(x, y));
        if (alive && (neighbors == 2 || neighbors == 3)) || (!alive && neighbors == 3) {
            next.insert((x, y));
        }
    }
    next
}

fn count_neighbors(live_cells: &FxHashSet<Cell>, x: i32, y: i32) -> u8 {
    let mut count = 0;
    for dx in -1..=1 {
        for dy in -1..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            if live_cells.contains(&(x + dx, y + dy)) {
                count += 1;
            }
        }
    }
    count
}
