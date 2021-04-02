use nalgebra::{Vector2, Vector3};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ball {
    pub position: Vector2<f64>,
    pub velocity: Vector2<f64>,
    pub radius: f64,
    pub initial_time: f64,
    pub color: Vector3<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Trail {
    pub position0: Vector2<f64>,
    pub position1: Vector2<f64>,
    pub initial_time: f64,
    pub final_time: f64,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Trails {
    pub trails: Vec<Trail>,
}
