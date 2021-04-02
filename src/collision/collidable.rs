pub const EPSILON: f64 = 1e-5;

#[derive(Clone, Copy, Debug, PartialEq, Hash, Eq)]
pub enum CollidableType {
    Ball,
    Wall,
}
#[derive(Clone, Copy, Debug, PartialEq, Hash, Eq)]
pub struct Generation {
    pub generation: i64,
}
