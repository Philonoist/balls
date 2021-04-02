use super::collidable::{CollidableType, Generation};
use legion::{
    world::{EntryRef, SubWorld},
    Entity, EntityStore,
};

use crate::{
    advance::advance_single_ball,
    ball::{Ball, Trails},
    wall::Wall,
};

#[derive(Clone, Copy, Debug, PartialEq, Hash, Eq)]
pub struct GenerationalCollisionEntity {
    pub entity: Entity,
    pub generation: i64,
}

pub struct EntityAndRef<'a> {
    pub entity: Entity,
    pub entry: EntryRef<'a>,
}

impl EntityAndRef<'_> {
    pub fn get<'a>(world: &'a SubWorld<'a>, entity: Entity) -> EntityAndRef<'a> {
        EntityAndRef {
            entity,
            entry: world.entry_ref(entity).unwrap(),
        }
    }
}

//get_component_unchecked
pub fn collide<'a>(
    world: &SubWorld,
    entry0: &EntityAndRef,
    entry1: &EntityAndRef,
    t: f64,
) -> Vec<GenerationalCollisionEntity> {
    let collidable_type0 = entry0.entry.get_component::<CollidableType>().unwrap();
    let collidable_type1 = entry1.entry.get_component::<CollidableType>().unwrap();
    match collidable_type0 {
        CollidableType::Ball => match collidable_type1 {
            CollidableType::Ball => collide_ball_ball(world, entry0, entry1, t),
            CollidableType::Wall => collide_ball_wall(world, entry0, entry1, t),
        },
        CollidableType::Wall => match collidable_type1 {
            CollidableType::Ball => collide_ball_wall(world, entry1, entry0, t),
            CollidableType::Wall => vec![],
        },
    }
}

fn collide_ball_wall<'a>(
    world: &SubWorld,
    entry0: &EntityAndRef,
    entry1: &EntityAndRef,
    t: f64,
) -> Vec<GenerationalCollisionEntity> {
    unsafe {
        let mut ball = entry0.entry.get_component_unchecked::<Ball>().unwrap();
        let wall = entry1.entry.get_component::<Wall>().unwrap();
        // Wall does not move.
        let mut trails = entry0.entry.get_component_unchecked::<Trails>().unwrap();
        advance_single_ball(&mut ball, &mut trails, t);

        let normal = wall.normal();
        let proj = ball.velocity.dot(&normal);
        if proj < 0. {
            ball.velocity -= proj * normal * 2.;
            let mut generation = entry0
                .entry
                .get_component_unchecked::<Generation>()
                .unwrap();
            generation.generation += 1;
            return vec![GenerationalCollisionEntity {
                entity: entry0.entity.clone(),
                generation: generation.generation,
            }];
        }
        vec![]
    }
}

fn collide_ball_ball<'a>(
    world: &SubWorld,
    entry0: &EntityAndRef,
    entry1: &EntityAndRef,
    t: f64,
) -> Vec<GenerationalCollisionEntity> {
    unsafe {
        let mut ball0 = entry0.entry.get_component_unchecked::<Ball>().unwrap();
        let mut ball1 = entry1.entry.get_component_unchecked::<Ball>().unwrap();
        let mut trails0 = entry0.entry.get_component_unchecked::<Trails>().unwrap();
        let mut trails1 = entry1.entry.get_component_unchecked::<Trails>().unwrap();
        let mut generation0 = entry0
            .entry
            .get_component_unchecked::<Generation>()
            .unwrap();
        let mut generation1 = entry1
            .entry
            .get_component_unchecked::<Generation>()
            .unwrap();

        advance_single_ball(&mut ball0, &mut trails0, t);
        advance_single_ball(&mut ball1, &mut trails1, t);

        let mass0 = ball0.radius * ball0.radius;
        let mass1 = ball1.radius * ball1.radius;
        let dx = ball0.position - ball1.position;
        let dv = ball0.velocity - ball1.velocity;
        // Check if they are moving towards each other.
        let proj = dv.dot(&dx);
        if proj < 0. {
            let d2 = dx.dot(&dx);
            let a = 2. / (mass0 + mass1) * proj / d2 * dx;
            ball0.velocity -= mass1 * a;
            if ball0.velocity.norm() > 1000. {
                ball0.velocity *= 1000. / ball0.velocity.norm();
            }
            ball1.velocity += mass0 * a;
            if ball1.velocity.norm() > 1000. {
                ball1.velocity *= 1000. / ball1.velocity.norm();
            }
            generation0.generation += 1;
            generation1.generation += 1;

            return vec![
                GenerationalCollisionEntity {
                    entity: entry0.entity.clone(),
                    generation: generation0.generation,
                },
                GenerationalCollisionEntity {
                    entity: entry1.entity.clone(),
                    generation: generation1.generation,
                },
            ];
        }
        vec![]
    }
}
