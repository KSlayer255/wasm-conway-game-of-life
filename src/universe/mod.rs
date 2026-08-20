mod sparse;

pub use sparse::SparseUniverse;
pub type Cell = (i32, i32);

pub trait Universe {
    fn tick(&mut self);
    fn live_cells(&self) -> &rustc_hash::FxHashSet<Cell>;
    fn step_back(&mut self);
    fn is_replaying(&self) -> bool;
    fn generation(&self) -> u64;
}
