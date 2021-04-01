use nalgebra::Vector2;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ball {
    pub position: Vector2<f64>,
    pub velocity: Vector2<f64>,
    pub radius: f64,
    pub initial_time: f64,
    pub collision_generation: i32,
}
