mod sparse;

pub use sparse::SparseUniverse;
pub type Cell = (i32, i32);

pub trait Universe {
    fn tick(&mut self);
    fn live_cells(&self) -> &rustc_hash::FxHashSet<Cell>;
    fn camera_x(&self) -> i32;
    fn camera_y(&self) -> i32;
    fn pan(&mut self, dx: i32, dy: i32);
    fn scale(&self) -> i32;
    fn _set_scale(&mut self, scale: i32);
    fn zoom_in(&mut self);
    fn zoom_out(&mut self);
    fn generation(&self) -> u64;
}
