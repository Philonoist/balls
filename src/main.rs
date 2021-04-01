use winit::event::{Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowBuilder};
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
use simulation::{adjust_simulation_speed, init_simulation, SimulationConfig};
use world_gen::{init_world, GenerationConfig};

const WIDTH: u32 = 1600;
const HEIGHT: u32 = 800;

pub fn main() {
    // Logging.
    log4rs::init_file("config/log4rs.yaml", Default::default())
        .expect("Logging configuration file 'log4rs.yaml' not found.");

    // Setup.
    let (graphics, event_loop) = init_graphics(DisplayConfig {
        width: WIDTH,
        height: HEIGHT,
        max_vertices: 60000,
    });
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
    init_simulation(&mut resources, SimulationConfig { time_delta: 0.1 });
    resources.insert(CollisionDetectionData::default());

    // Initialize scheduler.
    let mut schedule = Schedule::builder()
        .add_system(crate::collision::collision_system())
        .add_system(crate::collision::collision_handle_system())
        .add_system(crate::advance::advance_balls_system())
        .add_system(crate::simulation::advance_time_system())
        .add_thread_local(crate::render::render_balls_system())
        .build();

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        }
        | Event::WindowEvent {
            event:
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(VirtualKeyCode::Escape),
                            ..
                        },
                    ..
                },
            ..
        } => {
            *control_flow = ControlFlow::Exit;
        }
        Event::WindowEvent {
            event:
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(VirtualKeyCode::NumpadAdd),
                            ..
                        },
                    ..
                },
            ..
        } => {
            adjust_simulation_speed(&mut resources, 1.1);
        }
        Event::WindowEvent {
            event:
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(VirtualKeyCode::NumpadSubtract),
                            ..
                        },
                    ..
                },
            ..
        } => {
            adjust_simulation_speed(&mut resources, 1. / 1.1);
        }
        Event::RedrawEventsCleared => {
            schedule.execute(&mut world, &mut resources);
        }
        _ => (),
    });
}
