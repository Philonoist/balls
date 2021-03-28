use crate::{
    advance::advance_single_ball,
    ball::Ball,
    simulation::{SimulationParams, SimulationTime},
    wall::Wall,
};
use fnv::FnvHashMap;
use fnv::FnvHashSet;
use legion::IntoQuery;
use legion::{system, world::SubWorld, Entity, EntityStore};
use maybe_owned::MaybeOwned;
use nalgebra::Vector2;
use ordered_float::OrderedFloat;
use priority_queue::PriorityQueue;
use std::time::{SystemTime, UNIX_EPOCH};
const CELL_SIZE: f32 = 20.;
const EPSILON: f32 = 1e-5;

#[derive(Clone, Debug, PartialEq)]
enum Collidable<'a> {
    Ball(MaybeOwned<'a, Ball>),
    Wall(MaybeOwned<'a, Wall>),
}

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

fn get_cell_range_for_movement(collidable: &Collidable, time_delta: f32) -> (i32, i32, i32, i32) {
    let (min_coords, max_coords) = match collidable {
        Collidable::Ball(ball) => {
            // Compute bounding box.
            let new_position = ball.position + ball.velocity * time_delta;
            (
                ball.position.inf(&new_position).add_scalar(-ball.radius),
                ball.position.sup(&new_position).add_scalar(ball.radius),
            )
        }
        Collidable::Wall(wall) => (wall.p0.inf(&wall.p1), wall.p0.sup(&wall.p1)),
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
        time_delta: f32,
    ) {
        let (i0, i1, j0, j1) = get_cell_range_for_movement(collidable, time_delta);
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

        // TODO: Remove from spatial hash when generation increases.

        // Solve collisions.
        for candidate_entity in results {
            let candidate_collidable = fetch_collidable_copy(world, candidate_entity);
            let collisions_sol = solve_collision(collidable, &candidate_collidable);
            if let Some(collision_time) = collisions_sol {
                if (collision_time >= time - EPSILON)
                    && (collision_time <= time + time_delta + EPSILON)
                {
                    self.collisions_events
                        .push((entity, candidate_entity), OrderedFloat(collision_time));
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

fn solve_collision(collidable: &Collidable, other_collidable: &Collidable) -> Option<f32> {
    match collidable {
        Collidable::Ball(ball) => match other_collidable {
            Collidable::Ball(other_ball) => solve_collision_ball_ball(ball, other_ball),
            Collidable::Wall(wall) => solve_collision_ball_wall(ball, wall),
        },
        Collidable::Wall(wall) => match other_collidable {
            Collidable::Ball(ball) => solve_collision_ball_wall(ball, wall),
            Collidable::Wall(_) => None,
        },
    }
}

fn solve_collision_ball_wall(ball: &Ball, wall: &Wall) -> Option<f32> {
    // TODO: segments;
    let normal = wall.normal();
    // normal*(pb-pw+vt)=r.
    let a = normal.dot(&ball.velocity);
    let d = normal.dot(&(ball.position - wall.p0));
    let b = d - ball.radius;
    if d * a > 0. {
        // If relative position and relative speed are at the same direction, then the ball is moving away.
        // No collision here.
        return None;
    }

    // if a.abs() < EPSILON {
    //     return None;
    // }
    return Some(-b / a + ball.initial_time);
}

fn solve_collision_ball_ball(ball: &Ball, other_ball: &Ball) -> Option<f32> {
    // Shift to start at the same time.
    // d(p0+v0(t-t0), p1+v1(t-t1)) <= r0+r1.
    // || p0-v0t0-p1+v1t1 +t(v0-v1) ||^2 <= (r0+r1)^2.
    let dv = ball.velocity - other_ball.velocity;
    let dx = ball.position - other_ball.position;

    let affine0 = ball.position - ball.velocity * ball.initial_time;
    let affine1 = other_ball.position - other_ball.velocity * other_ball.initial_time;
    let affine = affine0 - affine1;

    let proj = dv.dot(&dx);
    if proj > 0. {
        // Ball are moving away.
        return None;
    }

    let a = dv.dot(&dv);
    let b = dv.dot(&affine) * 2.;
    let c =
        affine.dot(&affine) - (ball.radius + other_ball.radius) * (ball.radius + other_ball.radius);

    // if a <= EPSILON {
    //     return None;
    // }

    let disc = b * b - 4. * a * c;
    if disc < 0. {
        return None;
    }

    let sqrt_disc = disc.sqrt();

    // Entry time is the first root.
    let root0 = (-b - sqrt_disc) / (2. * a);
    return Some(root0);
}

#[system]
#[read_component(Entity)]
#[read_component(Ball)]
#[read_component(Wall)]
pub fn collision(
    world: &mut SubWorld,
    #[resource] time: &SimulationTime,
    #[resource] simulation_params: &SimulationParams,
    #[resource] collision_detection_data: &mut CollisionDetectionData,
) {
    // Clear data.
    // let t0 = SystemTime::now()
    //     .duration_since(UNIX_EPOCH)
    //     .unwrap()
    //     .as_millis();
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
            time.time,
            simulation_params.time_delta,
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
            time.time,
            simulation_params.time_delta,
        );
    }

    // let t1 = SystemTime::now()
    //     .duration_since(UNIX_EPOCH)
    //     .unwrap()
    //     .as_millis();
    // println!("Collision time {}", t1 - t0);
}

#[system]
#[write_component(Ball)]
#[write_component(Wall)]
pub fn collision_handle(
    world: &mut SubWorld,
    #[resource] time: &SimulationTime,
    #[resource] simulation_params: &SimulationParams,
    #[resource] collision_detection_data: &mut CollisionDetectionData,
) {
    // let t0 = SystemTime::now()
    //     .duration_since(UNIX_EPOCH)
    //     .unwrap()
    //     .as_millis();
    // Clear data.
    while !collision_detection_data.collisions_events.is_empty() {
        let ((collision_entity0, collision_entity1), ordered_t) = collision_detection_data
            .collisions_events
            .pop()
            .expect("Impossible");
        let collision_time = ordered_t.0;
        if collision_detection_data.collisions_events.len() > 200 {
            println!(
                "Queue pop t={}, len={}",
                collision_time,
                collision_detection_data.collisions_events.len(),
            );
        }
        // println!(
        //     "Collision {:?} {:?} at {}",
        //     collision_entity0, collision_entity1, collision_time
        // );

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
                    time.time + simulation_params.time_delta - collision_time,
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
                    time.time + simulation_params.time_delta - collision_time,
                );
            }
        }
    }
    // let t1 = SystemTime::now()
    //     .duration_since(UNIX_EPOCH)
    //     .unwrap()
    //     .as_millis();
    // println!("Collision handle took: {}", t1 - t0);
}

fn write_collidable(world: &mut SubWorld, entity: Entity, collidable: &Collidable) -> () {
    // println!("Writing {:?} at {:?}", collidable, entity);
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

fn collide<'a>(
    collidable: Collidable,
    generation: i32,
    other_collidable: Collidable,
    other_generation: i32,
    t: f32,
) -> Option<(Option<Collidable<'a>>, Option<Collidable<'a>>)> {
    match collidable {
        Collidable::Ball(ball) => {
            if ball.collision_generation != generation {
                return None;
            }
            match other_collidable {
                Collidable::Ball(other_ball) => {
                    if other_ball.collision_generation != other_generation {
                        return None;
                    }
                    Some(collide_ball_ball(&ball, &other_ball, t))
                }
                Collidable::Wall(wall) => {
                    // None
                    Some(collide_ball_wall(&ball, &wall, t))
                }
            }
        }
        Collidable::Wall(wall) => match other_collidable {
            Collidable::Ball(ball) => {
                if ball.collision_generation != other_generation {
                    return None;
                }
                // None
                let res = collide_ball_wall(&ball, &wall, t);
                Some((res.1, res.0))
            }
            Collidable::Wall(_) => None,
        },
    }
}

fn collide_ball_wall<'a>(
    ball: &Ball,
    wall: &Wall,
    t: f32,
) -> (Option<Collidable<'a>>, Option<Collidable<'a>>) {
    // Wall does not move.
    // println!(
    let mut new_ball = ball.clone();
    advance_single_ball(&mut new_ball, t);

    let normal = wall.normal();
    new_ball.collision_generation += 1;
    let proj = ball.velocity.dot(&normal);
    if proj < 0. {
        new_ball.velocity -= proj * normal * 2.;
    }
    (Some(Collidable::Ball(MaybeOwned::from(new_ball))), None)
}

fn collide_ball_ball<'a>(
    ball: &Ball,
    other_ball: &Ball,
    t: f32,
) -> (Option<Collidable<'a>>, Option<Collidable<'a>>) {
    let mut ball0 = ball.clone();
    let mut ball1 = other_ball.clone();
    // if (t - ball0.initial_time < EPSILON) && (t - ball1.initial_time < EPSILON) {
    //     // Collision event too close. Make the balls stop.
    //     ball0.velocity *= 0.;
    //     ball1.velocity *= 0.;
    //     return (
    //         Some(Collidable::Ball(MaybeOwned::from(ball0))),
    //         Some(Collidable::Ball(MaybeOwned::from(ball1))),
    //     );
    // }
    advance_single_ball(&mut ball0, t);
    advance_single_ball(&mut ball1, t);
    ball0.collision_generation += 1;
    ball1.collision_generation += 1;

    let mass0 = ball0.radius * ball0.radius;
    let mass1 = ball1.radius * ball1.radius;
    let dx = ball0.position - ball1.position;
    let dv = ball0.velocity - ball1.velocity;
    // Check if they are moving towards each other.
    let proj = dv.dot(&dx);
    if proj < 0. {
        let d2 = (ball0.radius + ball1.radius) * (ball0.radius + ball1.radius);
        let a = 2. / (mass0 + mass1) * dv.dot(&dx) / d2 * dx;
        ball0.velocity -= mass1 * a;
        if ball0.velocity.norm() > 100. {
            ball0.velocity *= 100. / ball0.velocity.norm();
        }
        ball1.velocity += mass0 * a;
        if ball1.velocity.norm() > 100. {
            ball1.velocity *= 100. / ball1.velocity.norm();
        }
    }
    (
        Some(Collidable::Ball(MaybeOwned::from(ball0))),
        Some(Collidable::Ball(MaybeOwned::from(ball1))),
    )
}
