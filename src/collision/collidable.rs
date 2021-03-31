use legion::{world::SubWorld, Entity, EntityStore};
use maybe_owned::MaybeOwned;

use crate::{ball::Ball, wall::Wall};

pub const EPSILON: f32 = 1e-5;

#[derive(Clone, Copy, Debug, PartialEq, Hash, Eq)]
pub enum CollidableType {
    Ball,
    Wall,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Collidable<'a> {
    Ball(MaybeOwned<'a, Ball>),
    Wall(MaybeOwned<'a, Wall>),
}

pub fn fetch_collidable_copy<'a, 'b>(
    world: &'b SubWorld,
    collidable_type: CollidableType,
    entity: Entity,
) -> Collidable<'a> {
    return match collidable_type {
        CollidableType::Ball => {
            let entry = world.entry_ref(entity).unwrap();
            // Try to remove this clone.
            let ball = entry.get_component::<Ball>().unwrap().clone();
            Collidable::Ball(MaybeOwned::from(ball))
        }
        CollidableType::Wall => {
            let entry = world.entry_ref(entity).unwrap();
            let wall = entry.get_component::<Wall>().unwrap().clone();
            Collidable::Wall(MaybeOwned::from(wall))
        }
    };
}

pub fn write_collidable(world: &mut SubWorld, entity: Entity, collidable: &Collidable) -> () {
    match collidable {
        Collidable::Ball(ball) => {
            *(world
                .entry_mut(entity)
                .unwrap()
                .get_component_mut::<Ball>()
                .unwrap()) = **ball;
        }
        Collidable::Wall(wall) => {
            *(world
                .entry_mut(entity)
                .unwrap()
                .get_component_mut::<Wall>()
                .unwrap()) = **wall;
        }
    }
}
