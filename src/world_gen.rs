use crate::wall::Wall;
use crate::{
    ball::{Ball, Trails},
    collision::collidable::{CollidableType, Generation},
};
use legion::World;
use nalgebra::{Vector2, Vector3};
use rand::Rng;
use rand_pcg::Pcg64;

pub struct GenerationConfig {
    pub width: u32,
    pub height: u32,
}

pub fn init_world(world: &mut World, config: GenerationConfig) {
    init_walls(world, &config);
    init_balls(world, &config);
}

fn init_walls(world: &mut World, config: &GenerationConfig) {
    let points = [
        Vector2::new(0., 0.),
        Vector2::new(config.width as f64, 0.),
        Vector2::new(config.width as f64, config.height as f64),
        Vector2::new(0., config.height as f64),
    ];
    let mut walls = std::vec::Vec::<(Wall, CollidableType, Generation)>::new();
    walls.reserve(4);
    walls.extend(
        [
            (
                Wall {
                    p0: points[0],
                    p1: points[1],
                },
                CollidableType::Wall,
                Generation { generation: 0 },
            ),
            (
                Wall {
                    p0: points[1],
                    p1: points[2],
                },
                CollidableType::Wall,
                Generation { generation: 0 },
            ),
            (
                Wall {
                    p0: points[2],
                    p1: points[3],
                },
                CollidableType::Wall,
                Generation { generation: 0 },
            ),
            (
                Wall {
                    p0: points[3],
                    p1: points[0],
                },
                CollidableType::Wall,
                Generation { generation: 0 },
            ),
        ]
        .iter(),
    );
    world.extend(walls);
}

fn init_balls(world: &mut World, config: &GenerationConfig) {
    // let mut rng = rand::thread_rng();
    let mut rng = Pcg64::new(0xcafef00dd15ea5e5, 0xa02bdbf7bb3c0a7ac28fa16a64abf96);
    let n_balls = 1000;
    let mut balls = std::vec::Vec::<(Ball, Trails, CollidableType, Generation)>::new();
    balls.reserve(n_balls);

    let colors = vec![
        Vector3::new(0.9, 0.8, 0.7),
        Vector3::new(0.7, 0.9, 0.8),
        Vector3::new(0.8, 0.7, 0.9),
    ];

    while balls.len() < n_balls {
        let angle = rng.gen_range(0.0..(std::f64::consts::TAU));
        let speed = rng.gen_range(3.0..50.0);
        let radius = rng.gen_range(2.0..30.0);
        let ball = Ball {
            position: Vector2::new(
                rng.gen_range(radius..(config.width as f64 - radius)),
                rng.gen_range(radius..(config.height as f64 - radius)),
            ),
            velocity: Vector2::new(speed * angle.cos(), speed * angle.sin()),
            radius: radius,
            initial_time: 0.,
            color: colors[rng.gen_range(0..colors.len())],
        };

        // Check it doesn't overlap with an existing ball.
        let mut found = false;
        for (other_ball, _, _, _) in &balls {
            if (other_ball.position - ball.position).norm() <= other_ball.radius + ball.radius {
                found = true;
                break;
            }
        }
        if found {
            continue;
        }
        balls.push((
            ball,
            Trails::default(),
            CollidableType::Ball,
            Generation { generation: 0 },
        ));
    }
    world.extend(balls);
}
