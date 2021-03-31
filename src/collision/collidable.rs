use maybe_owned::MaybeOwned;

use crate::{ball::Ball, wall::Wall};

pub const EPSILON: f32 = 1e-5;

#[derive(Clone, Debug, PartialEq)]
pub enum Collidable<'a> {
    Ball(MaybeOwned<'a, Ball>),
    Wall(MaybeOwned<'a, Wall>),
}
