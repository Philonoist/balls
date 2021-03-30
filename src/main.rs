extern crate sdl2;

pub mod advance;
pub mod ball;
pub mod collision;
pub mod simulation;
pub mod wall;

use ball::Ball;
use collision::CollisionDetectionData;
use legion::*;
use nalgebra::Vector2;
use rand::Rng;
use rand_pcg::Pcg64;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::{event::Event, gfx::primitives::DrawRenderer};
use simulation::{SimulationParams, SimulationTime};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use wall::Wall;

const WIDTH: u32 = 1600;
const HEIGHT: u32 = 800;

fn init_canvas() -> (sdl2::Sdl, sdl2::render::Canvas<sdl2::video::Window>) {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem
        .window("aaaa", WIDTH, HEIGHT)
        .position_centered()
        .build()
        .unwrap();

    (sdl_context, window.into_canvas().build().unwrap())
}

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
    let n_balls = 1000;
    let mut balls = std::vec::Vec::<(Ball,)>::new();
    balls.reserve(n_balls);

    while balls.len() < n_balls {
        let angle = rng.gen_range(0.0..(std::f32::consts::TAU));
        let speed = rng.gen_range(3.0..50.0);
        let radius = rng.gen_range(1.0..30.0);
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
    // balls.push((Ball {
    //     position: Vector2::new(20., 100.),
    //     velocity: Vector2::new(1000., 0.),
    //     radius: 10.,
    //     initial_time: 0.,
    //     collision_generation: 0,
    // },));
    // balls.push((Ball {
    //     position: Vector2::new(281., 100.),
    //     velocity: Vector2::new(-100., 0.),
    //     radius: 100.,
    //     initial_time: 0.1,
    //     collision_generation: 0,
    // },));
    world.extend(balls);
}

#[system(for_each)]
fn render_balls(ball: &Ball, #[resource] canvas: &mut sdl2::render::Canvas<sdl2::video::Window>) {
    if ball.position[0] < -1000.0
        || ball.position[1] < -1000.0
        || ball.position[0] > 10000.0
        || ball.position[1] > 10000.0
    {
        println!("Bad ball {:?}", ball);
    }
    canvas
        .filled_circle(
            ball.position[0] as i16,
            ball.position[1] as i16,
            ball.radius as i16,
            Color::RGB(0, 0, 255),
        )
        .expect("ok");
}

pub fn main() {
    // Setup.
    let (sdl_context, canvas) = init_canvas();
    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut world = World::default();

    // Initialize world.
    init_walls(&mut world);
    init_balls(&mut world);
    let mut resources = Resources::default();
    resources.insert(canvas);
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
        .add_thread_local(render_balls_system())
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

        {
            let mut canvas_ref = resources
                .get_mut::<sdl2::render::Canvas<sdl2::video::Window>>()
                .expect("Canvas resource not found");
            canvas_ref.present();
            canvas_ref.set_draw_color(Color::RGB(0, 0, 0));
            canvas_ref.clear();
        }
        // ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }
}
