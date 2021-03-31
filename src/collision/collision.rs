use super::{collidable::EPSILON, colliders::collide, solvers::solve_collision};
use crate::{advance::advance_single_ball, ball::Ball, simulation::SimulationData, wall::Wall};
use fnv::FnvHashMap;
use fnv::FnvHashSet;
use legion::IntoQuery;
use legion::{system, world::SubWorld, Entity, EntityStore};
use log::debug;
use maybe_owned::MaybeOwned;
use nalgebra::Vector2;
use ordered_float::OrderedFloat;
use priority_queue::PriorityQueue;
use std::time::{SystemTime, UNIX_EPOCH};

use super::collidable::Collidable;
const CELL_SIZE: f32 = 20.;

#[derive(Clone, Copy, Debug, PartialEq, Hash, Eq)]
enum CollisionType {
    Ball,
    Wall,
}

#[derive(Clone, Copy, Debug, PartialEq, Hash, Eq)]
struct GenerationalCollisionEntity {
    entity: Entity,
    collision_type: CollisionType,
    generation: i32,
}

// This is ugly.
impl GenerationalCollisionEntity {
    fn next(self) -> GenerationalCollisionEntity {
        match self.collision_type {
            CollisionType::Ball => GenerationalCollisionEntity {
                generation: self.generation + 1,
                ..self
            },
            CollisionType::Wall => self,
        }
    }
}
#[derive(Default)]
pub struct CollisionDetectionData {
    spatial_buckets: FnvHashMap<(i32, i32), FnvHashSet<GenerationalCollisionEntity>>,
    last_box: FnvHashMap<GenerationalCollisionEntity, (i32, i32, i32, i32)>,
    collisions_events: PriorityQueue<
        (GenerationalCollisionEntity, GenerationalCollisionEntity),
        OrderedFloat<f32>,
    >,
    // TODO: Set that remembers?
}

fn get_cell_range_for_movement(collidable: &Collidable, next_time: f32) -> (i32, i32, i32, i32) {
    let (min_coords, max_coords) = match collidable {
        Collidable::Ball(ball) => {
            // Compute bounding box.
            let time_delta = next_time - ball.initial_time;
            let new_position = ball.position + ball.velocity * time_delta;
            (
                ball.position
                    .inf(&new_position)
                    .add_scalar(-ball.radius - EPSILON),
                ball.position
                    .sup(&new_position)
                    .add_scalar(ball.radius + EPSILON),
            )
        }
        Collidable::Wall(wall) => (
            wall.p0.inf(&wall.p1).add_scalar(-EPSILON),
            wall.p0.sup(&wall.p1.add_scalar(EPSILON)),
        ),
    };
    return (
        std::cmp::max(0, (min_coords.x / CELL_SIZE).floor() as i32),
        std::cmp::min(100, (max_coords.x / CELL_SIZE).ceil() as i32) + 1,
        std::cmp::max(0, (min_coords.y / CELL_SIZE).floor() as i32),
        std::cmp::min(100, (max_coords.y / CELL_SIZE).ceil() as i32) + 1,
    );
}

impl CollisionDetectionData {
    fn add(
        &mut self,
        world: &SubWorld,
        entity: GenerationalCollisionEntity,
        collidable: &Collidable,
        time: f32,
        next_time: f32,
    ) {
        let (i0, i1, j0, j1) = get_cell_range_for_movement(collidable, next_time);
        self.last_box.insert(entity, (i0, i1, j0, j1));
        // Find candidates using spatial hash mapping.
        let mut results = FnvHashSet::<GenerationalCollisionEntity>::default();

        for i in i0..i1 {
            for j in j0..j1 {
                if let Some(cell_set) = self.spatial_buckets.get_mut(&(i, j)) {
                    results.extend(cell_set.iter());
                    cell_set.insert(entity);
                } else {
                    self.spatial_buckets
                        .insert((i, j), [entity].iter().cloned().collect());
                }
            }
        }

        // Solve collisions.
        for candidate_entity in results {
            let candidate_collidable = fetch_collidable_copy(world, candidate_entity);
            let collisions_sol = solve_collision(collidable, &candidate_collidable);
            if let Some((t0, t1)) = collisions_sol {
                if segments_intersect((t0, t1), (time, next_time)) {
                    self.collisions_events.push(
                        (entity, candidate_entity),
                        OrderedFloat(-t0.clamp(time, next_time)),
                    );
                }
            }
        }
    }

    fn remove(&mut self, entity: GenerationalCollisionEntity) {
        if let Some((i0, i1, j0, j1)) = self.last_box.remove(&entity) {
            for i in i0..i1 {
                for j in j0..j1 {
                    if let Some(cell_set) = self.spatial_buckets.get_mut(&(i, j)) {
                        cell_set.remove(&entity);
                    }
                }
            }
        }
    }
}

fn segments_intersect((x0, x1): (f32, f32), (y0, y1): (f32, f32)) -> bool {
    return x1 >= y0 && y1 >= x0;
}

fn fetch_collidable_copy<'a, 'b>(
    world: &'b SubWorld,
    candidate_entity: GenerationalCollisionEntity,
) -> Collidable<'a> {
    return match candidate_entity.collision_type {
        CollisionType::Ball => {
            let entry = world.entry_ref(candidate_entity.entity).unwrap();
            // Try to remove this clone.
            let ball = entry.get_component::<Ball>().unwrap().clone();
            Collidable::Ball(MaybeOwned::from(ball))
        }
        CollisionType::Wall => {
            let entry = world.entry_ref(candidate_entity.entity).unwrap();
            let wall = entry.get_component::<Wall>().unwrap().clone();
            Collidable::Wall(MaybeOwned::from(wall))
        }
    };
}

#[system]
#[read_component(Entity)]
#[read_component(Ball)]
#[read_component(Wall)]
pub fn collision(
    world: &mut SubWorld,
    #[resource] simulation_data: &SimulationData,
    #[resource] collision_detection_data: &mut CollisionDetectionData,
) {
    // Clear data.
    collision_detection_data.spatial_buckets.clear();
    collision_detection_data.collisions_events.clear();

    // Iterate balls.
    for (entity, ball) in <(Entity, &Ball)>::query().iter(world) {
        collision_detection_data.add(
            world,
            GenerationalCollisionEntity {
                collision_type: CollisionType::Ball,
                entity: entity.clone(),
                generation: ball.collision_generation,
            },
            &Collidable::Ball(MaybeOwned::from(ball)),
            simulation_data.time,
            simulation_data.next_time,
        );
    }

    // Iterate walls.
    for (entity, wall) in <(Entity, &Wall)>::query().iter(world) {
        collision_detection_data.add(
            world,
            GenerationalCollisionEntity {
                collision_type: CollisionType::Wall,
                entity: entity.clone(),
                generation: 0,
            },
            &Collidable::Wall(MaybeOwned::from(wall)),
            simulation_data.time,
            simulation_data.next_time,
        );
    }
}

#[system]
#[write_component(Ball)]
#[write_component(Wall)]
pub fn collision_handle(
    world: &mut SubWorld,
    #[resource] simulation_data: &SimulationData,
    #[resource] collision_detection_data: &mut CollisionDetectionData,
) {
    // Clear data.
    while !collision_detection_data.collisions_events.is_empty() {
        let ((collision_entity0, collision_entity1), ordered_t) = collision_detection_data
            .collisions_events
            .pop()
            .expect("Impossible");
        let collision_time = -ordered_t.0;
        debug!(
            "Collision {:?} {:?} at {}",
            collision_entity0, collision_entity1, collision_time
        );

        // TODO: Consider separating collision_generation to its own (optional?) component.
        let collidable0 = fetch_collidable_copy(world, collision_entity0);
        let collidable1 = fetch_collidable_copy(world, collision_entity1);

        let result = collide(
            collidable0,
            collision_entity0.generation,
            collidable1,
            collision_entity1.generation,
            collision_time,
        );
        if let Some((new_collidable0, new_collidable1)) = result {
            if let Some(c) = new_collidable0 {
                collision_detection_data.remove(collision_entity0);
                write_collidable(world, collision_entity0.entity, &c);
                collision_detection_data.add(
                    world,
                    collision_entity0.next(),
                    &c,
                    collision_time,
                    simulation_data.next_time,
                );
            }
            if let Some(c) = new_collidable1 {
                collision_detection_data.remove(collision_entity1);
                write_collidable(world, collision_entity1.entity, &c);
                collision_detection_data.add(
                    world,
                    collision_entity1.next(),
                    &c,
                    collision_time,
                    simulation_data.next_time,
                );
            }
        }
    }
}

fn write_collidable(world: &mut SubWorld, entity: Entity, collidable: &Collidable) -> () {
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
