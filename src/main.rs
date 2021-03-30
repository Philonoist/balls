extern crate sdl2;

pub mod advance;
pub mod ball;
pub mod collision;
pub mod render;
pub mod simulation;
pub mod wall;

use ball::Ball;
use collision::CollisionDetectionData;
use legion::*;
use nalgebra::Vector2;
use rand::Rng;
use rand_pcg::Pcg64;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use simulation::{SimulationParams, SimulationTime};
use std::time::{SystemTime, UNIX_EPOCH};
use wall::Wall;

const WIDTH: u32 = 1600;
const HEIGHT: u32 = 800;

fn init_walls(world: &mut World) {
    let points = [
        Vector2::new(0., 0.),
        Vector2::new(WIDTH as f32, 0.),
        Vector2::new(WIDTH as f32, HEIGHT as f32),
        Vector2::new(0., HEIGHT as f32),
    ];
    let mut walls = std::vec::Vec::<(Wall,)>::new();
    walls.reserve(4);
    walls.extend(
        [
            (Wall {
                p0: points[0],
                p1: points[1],
            },),
            (Wall {
                p0: points[1],
                p1: points[2],
            },),
            (Wall {
                p0: points[2],
                p1: points[3],
            },),
            (Wall {
                p0: points[3],
                p1: points[0],
            },),
        ]
        .iter(),
    );
    world.extend(walls);
}

fn init_balls(world: &mut World) {
    // let mut rng = rand::thread_rng();
    let mut rng = Pcg64::new(0xcafef00dd15ea5e5, 0xa02bdbf7bb3c0a7ac28fa16a64abf96);
    let n_balls = 1500;
    let mut balls = std::vec::Vec::<(Ball,)>::new();
    balls.reserve(n_balls);

    while balls.len() < n_balls {
        let angle = rng.gen_range(0.0..(std::f32::consts::TAU));
        let speed = rng.gen_range(3.0..50.0);
        let radius = rng.gen_range(2.0..8.0);
        let ball = Ball {
            position: Vector2::new(
                rng.gen_range(radius..(WIDTH as f32 - radius)),
                rng.gen_range(radius..(HEIGHT as f32 - radius)),
            ),
            velocity: Vector2::new(speed * angle.cos(), speed * angle.sin()),
            radius: radius,
            initial_time: 0.,
            collision_generation: 0,
        };

        // Check it doesn't overlap with an existing ball.
        let mut found = false;
        for (other_ball,) in &balls {
            if (other_ball.position - ball.position).norm() <= other_ball.radius + ball.radius {
                found = true;
                break;
            }
        }
        if found {
            continue;
        }
        balls.push((ball,));
    }
    world.extend(balls);
}

pub fn main() {
    // Setup.
    let graphics = crate::render::init_graphics(crate::render::DisplayConfig {
        width: WIDTH,
        height: HEIGHT,
    });
    let mut event_pump = graphics.sdl_context.event_pump().unwrap();
    let mut world = World::default();

    // Initialize world.
    init_walls(&mut world);
    init_balls(&mut world);
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
