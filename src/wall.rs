use nalgebra::Vector2;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wall {
    pub p0: Vector2<f64>,
    pub p1: Vector2<f64>,
}

impl Wall {
    pub fn normal(&self) -> Vector2<f64> {
        let diff = self.p1 - self.p0;
        return Vector2::new(-diff.y, diff.x).normalize();
    }
}
