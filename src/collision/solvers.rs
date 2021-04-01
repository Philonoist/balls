use nalgebra::Vector2;

use crate::{ball::Ball, wall::Wall};

use super::collidable::Collidable;
use super::collidable::EPSILON;

pub fn get_movement_bounding_box(
    collidable: &Collidable,
    next_time: f64,
) -> (Vector2<f64>, Vector2<f64>) {
    match collidable {
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
    }
}

pub fn solve_collision(
    collidable: &Collidable,
    other_collidable: &Collidable,
) -> Option<(f64, f64)> {
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

fn solve_collision_ball_wall(ball: &Ball, wall: &Wall) -> Option<(f64, f64)> {
    // TODO: segments;
    let normal = wall.normal();
    // normal*(pb-pw+vt)=r.
    let a = normal.dot(&ball.velocity);
    let d = normal.dot(&(ball.position - wall.p0));
    if d * a >= 0. {
        // If relative position and relative speed are at the same direction, then the ball is moving away.
        // No collision here.
        return None;
    }

    let b0 = d - ball.radius;
    let b1 = d;
    return Some((-b0 / a + ball.initial_time, -b1 / a + ball.initial_time));
}

fn solve_collision_ball_ball(ball: &Ball, other_ball: &Ball) -> Option<(f64, f64)> {
    // Shift to start at the same time.
    // d(p0+v0(t-t0), p1+v1(t-t1)) <= r0+r1.
    // || p0-v0t0-p1+v1t1 +t(v0-v1) ||^2 <= (r0+r1)^2.
    let dv = ball.velocity - other_ball.velocity;
    let dx = ball.position - other_ball.position;

    let affine0 = ball.position - ball.velocity * ball.initial_time;
    let affine1 = other_ball.position - other_ball.velocity * other_ball.initial_time;
    let affine = affine0 - affine1;

    let proj = dv.dot(&dx);
    if proj > -EPSILON {
        // Balls are moving away.
        return None;
    }

    let a = dv.dot(&dv);
    let b = (dv.dot(&affine) * 2.);
    let c = (affine.dot(&affine)
        - (ball.radius + other_ball.radius) * (ball.radius + other_ball.radius));

    let disc = b * b - 4. * a * c;
    if disc < 0.0 {
        return None;
    }

    let sqrt_disc = disc.sqrt();

    // Entry time is the first root.
    let root0 = ((-b - sqrt_disc) / (2. * a)) as f64;
    let mid = (-b / (2. * a)) as f64;

    let delta = (ball.position + (root0 - ball.initial_time) * ball.velocity
        - other_ball.position
        - (root0 - other_ball.initial_time) * other_ball.velocity)
        .norm()
        - ball.radius
        - other_ball.radius;
    if delta > 0.1 {
        println!(
            "delta2: {}, a: {}, b:{}, c:{}, disc:{}",
            delta, a, b, c, disc
        );
        println!(
            "res: {}",
            (a as f64) * root0 * root0 + (b as f64) * root0 + (c as f64)
        );
    }

    return Some((root0, mid));
}
