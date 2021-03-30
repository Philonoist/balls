extern crate sdl2;

pub mod advance;
pub mod ball;
pub mod collision;
pub mod render;
pub mod simulation;
pub mod wall;
pub mod world_gen;

use collision::CollisionDetectionData;
use legion::*;
use render::{init_graphics, DisplayConfig};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use simulation::{SimulationParams, SimulationTime};
use std::time::{SystemTime, UNIX_EPOCH};
use world_gen::{init_world, GenerationConfig};

const WIDTH: u32 = 1600;
const HEIGHT: u32 = 800;

pub fn main() {
    // Setup.
    let graphics = init_graphics(DisplayConfig {
        width: WIDTH,
        height: HEIGHT,
    });
    let mut event_pump = graphics.sdl_context.event_pump().unwrap();
    let mut world = World::default();

    // Initialize world.
    init_world(
        &mut world,
        GenerationConfig {
            width: WIDTH,
            height: HEIGHT,
        },
    );
    let mut resources = Resources::default();
    resources.insert(graphics);
    resources.insert(SimulationParams { time_delta: 0.1 });
    resources.insert(SimulationTime {
        time: 0.0,
        last_simulated: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64,
    });
    resources.insert(CollisionDetectionData::default());

    // Initialize scheduler.
    let mut schedule = Schedule::builder()
        .add_system(crate::collision::collision_system())
        .add_system(crate::collision::collision_handle_system())
        .add_system(crate::advance::advance_balls_system())
        .add_system(crate::simulation::advance_time_system())
        .add_thread_local(crate::render::render_balls_system())
        .build();

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                Event::KeyDown {
                    keycode: Some(Keycode::KpPlus),
                    ..
                } => {
                    if let Some(mut sim_params) = resources.get_mut::<SimulationParams>() {
                        sim_params.time_delta *= 1.1;
                    }
                }
                Event::KeyDown {
                    keycode: Some(Keycode::KpMinus),
                    ..
                } => {
                    if let Some(mut sim_params) = resources.get_mut::<SimulationParams>() {
                        sim_params.time_delta /= 1.1;
                    }
                }
                _ => {}
            }
        }

        // The rest of the game loop goes here...
        // run our schedule (you should do this each update)
        schedule.execute(&mut world, &mut resources);
    }
}
