use nalgebra::Vector2;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wall {
    pub p0: Vector2<f32>,
    pub p1: Vector2<f32>,
}
