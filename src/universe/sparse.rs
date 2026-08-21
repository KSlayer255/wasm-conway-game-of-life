use crate::config::MAX_HISTORY;
use crate::universe::Cell;
use crate::universe::Universe;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::VecDeque;

pub struct SparseUniverse {
    history: VecDeque<FxHashSet<Cell>>,
    history_start_generation: u64,
    cursor: usize,
    ages: FxHashMap<Cell, u32>,
}

impl SparseUniverse {
    pub fn new(cells: FxHashSet<Cell>) -> Self {
        let ages = cells.iter().map(|&cell| (cell, 0)).collect();
        let mut history = VecDeque::with_capacity(MAX_HISTORY);
        history.push_back(cells);
        Self {
            history,
            history_start_generation: 0,
            cursor: 0,
            ages,
        }
    }
}

impl Universe for SparseUniverse {
    fn tick(&mut self) {
        if self.cursor + 1 < self.history.len() {
            self.cursor += 1;
            return;
        }

        let (next, next_ages) = step(&self.history[self.cursor], &self.ages);
        self.history.push_back(next);
        self.ages = next_ages;

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

    fn age_of(&self, cell: &Cell) -> u32 {
        self.ages.get(cell).copied().unwrap_or(0)
    }

    fn age_bounds(&self) -> (u32, u32) {
        if self.ages.is_empty() {
            return (0, 0);
        }
        self.ages
            .values()
            .fold((u32::MAX, 0), |(min_age, max_age), &age| {
                (min_age.min(age), max_age.max(age))
            })
    }
}

// ====== Private helper functions (Game of Life rules) ======
fn step(
    live_cells: &FxHashSet<Cell>,
    ages: &FxHashMap<Cell, u32>,
) -> (FxHashSet<Cell>, FxHashMap<Cell, u32>) {
    let mut candidates = FxHashSet::default();
    for &(x, y) in live_cells {
        for dx in -1..=1 {
            for dy in -1..=1 {
                candidates.insert((x + dx, y + dy));
            }
        }
    }

    let mut next = FxHashSet::default();
    let mut next_ages = FxHashMap::default();
    for &(x, y) in &candidates {
        let neighbors = count_neighbors(live_cells, x, y);
        let alive = live_cells.contains(&(x, y));
        if (alive && (neighbors == 2 || neighbors == 3)) || (!alive && neighbors == 3) {
            next.insert((x, y));
            let age = if alive {
                ages.get(&(x, y)).copied().unwrap_or(0) + 1
            } else {
                0
            };
            next_ages.insert((x, y), age);
        }
    }
    (next, next_ages)
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
