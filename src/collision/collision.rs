use super::{
    collidable::{CollidableType, Generation, EPSILON},
    colliders::{collide, EntityAndRef, GenerationalCollisionEntity},
    solvers::{get_movement_bounding_box, solve_collision},
};
use crate::{ball::Ball, ball::Trails, simulation::SimulationData, wall::Wall};
use fnv::FnvHashMap;
use fnv::FnvHashSet;
use legion::{
    query::View,
    system,
    world::{EntryRef, SubWorld},
    Entity,
};
use legion::{EntityStore, IntoQuery};
use log::debug;
use ordered_float::OrderedFloat;
use priority_queue::PriorityQueue;

const CELL_SIZE: f64 = 20.;

// This is ugly.
#[derive(Default)]
pub struct CollisionDetectionData {
    spatial_buckets: FnvHashMap<(i32, i32), FnvHashSet<GenerationalCollisionEntity>>,
    last_box: FnvHashMap<GenerationalCollisionEntity, (i32, i32, i32, i32)>,
    collisions_events: PriorityQueue<
        (GenerationalCollisionEntity, GenerationalCollisionEntity),
        OrderedFloat<f64>,
    >,
    // TODO: Set that remembers?
}

fn get_cell_range_for_movement(
    world: &SubWorld,
    entry: &EntryRef,
    next_time: f64,
) -> (i32, i32, i32, i32) {
    let (min_coords, max_coords) = get_movement_bounding_box(world, &entry, next_time);
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
        time: f64,
        next_time: f64,
    ) {
        let entry = world.entry_ref(entity.entity).unwrap();
        let (i0, i1, j0, j1) = get_cell_range_for_movement(world, &entry, next_time);
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
            let candidate_entry = world.entry_ref(candidate_entity.entity).unwrap();
            let collisions_sol = solve_collision(world, &entry, &candidate_entry);
            if let Some((t0, t1)) = collisions_sol {
                if segments_intersect((t0, t1), (time - EPSILON, next_time)) {
                    self.collisions_events
                        .push((entity, candidate_entity), OrderedFloat(-t0));
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

fn segments_intersect((x0, x1): (f64, f64), (y0, y1): (f64, f64)) -> bool {
    return x1 >= y0 && y1 >= x0;
}

#[system]
#[read_component(Ball)]
#[read_component(CollidableType)]
#[read_component(Entity)]
#[read_component(Generation)]
#[read_component(Wall)]
pub fn collision(
    world: &mut SubWorld,
    #[resource] simulation_data: &SimulationData,
    #[resource] collision_detection_data: &mut CollisionDetectionData,
) {
    // Clear data.
    collision_detection_data.spatial_buckets.clear();
    collision_detection_data.collisions_events.clear();

    // Iterate collidables.
    for (entity, generation, _) in <(Entity, &Generation, &CollidableType)>::query().iter(world) {
        collision_detection_data.add(
            world,
            GenerationalCollisionEntity {
                entity: entity.clone(),
                generation: generation.generation,
            },
            simulation_data.time,
            simulation_data.next_time,
        );
    }
}

#[system]
#[read_component(CollidableType)]
#[read_component(Entity)]
#[read_component(Wall)]
#[write_component(Ball)]
#[write_component(Generation)]
#[write_component(Trails)]
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

        let entry0 = EntityAndRef::get(world, collision_entity0.entity);
        let entry1 = EntityAndRef::get(world, collision_entity1.entity);
        if collision_entity0.generation
            != entry0
                .entry
                .get_component::<Generation>()
                .unwrap()
                .generation
        {
            continue;
        }
        if collision_entity1.generation
            != entry1
                .entry
                .get_component::<Generation>()
                .unwrap()
                .generation
        {
            continue;
        }

        let new_entities = collide(world, &entry0, &entry1, collision_time);
        for entity in new_entities.iter() {
            collision_detection_data.add(world, *entity, collision_time, simulation_data.next_time);
        }
    }
}
