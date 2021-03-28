use nalgebra::Vector2;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ball {
    pub position: Vector2<f32>,
    pub velocity: Vector2<f32>,
    pub radius: f32,
    pub initial_time: f32,
    pub collision_generation: i32,
}
