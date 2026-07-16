use crate::universe::Cell;
use crate::universe::Universe;
use std::collections::HashSet;

pub struct SparseUniverse {
    live_cells: HashSet<Cell>,
    camera_x: i32,
    camera_y: i32,
    scale: i32,
}

impl SparseUniverse {
    pub fn new(cells: HashSet<Cell>) -> Self {
        Self {
            live_cells: cells,
            camera_x: 0,
            camera_y: 0,
            scale: 0,
        }
    }

    fn step(&mut self) {
        self.live_cells = step(&self.live_cells);
    }
}

impl Universe for SparseUniverse {
    fn tick(&mut self) {
        self.step();
    }

    fn live_cells(&self) -> &HashSet<Cell> {
        &self.live_cells
    }

    fn camera_x(&self) -> i32 {
        self.camera_x
    }

    fn camera_y(&self) -> i32 {
        self.camera_y
    }

    fn pan(&mut self, dx: i32, dy: i32) {
        self.camera_x += dx;
        self.camera_y += dy;
    }

    fn scale(&self) -> i32 {
        self.scale
    }

    fn _set_scale(&mut self, scale: i32) {
        self.scale = scale
    }

    fn zoom_in(&mut self) {
        if self.scale > -8 {
            self.scale -= 1
        }
    }

    fn zoom_out(&mut self) {
        if self.scale < 0 {
            self.scale += 1
        }
    }
}

// ====== Private helper functions (Game of Life rules) ======

fn step(live_cells: &HashSet<Cell>) -> HashSet<Cell> {
    let mut candidates = HashSet::new();
    for &(x, y) in live_cells {
        for dx in -1..=1 {
            for dy in -1..=1 {
                candidates.insert((x + dx, y + dy));
            }
        }
    }

    let mut next = HashSet::new();
    for &(x, y) in &candidates {
        let neighbors = count_neighbors(live_cells, x, y);
        let alive = live_cells.contains(&(x, y));
        if (alive && (neighbors == 2 || neighbors == 3)) || (!alive && neighbors == 3) {
            next.insert((x, y));
        }
    }
    next
}

fn count_neighbors(live_cells: &HashSet<Cell>, x: i32, y: i32) -> u8 {
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
