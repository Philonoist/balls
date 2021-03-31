use crate::{ball::Ball, wall::Wall};

use super::collidable::Collidable;
use super::collidable::EPSILON;

pub fn solve_collision(
    collidable: &Collidable,
    other_collidable: &Collidable,
) -> Option<(f32, f32)> {
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

fn solve_collision_ball_wall(ball: &Ball, wall: &Wall) -> Option<(f32, f32)> {
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

fn solve_collision_ball_ball(ball: &Ball, other_ball: &Ball) -> Option<(f32, f32)> {
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
    let b = dv.dot(&affine) * 2.;
    let c =
        affine.dot(&affine) - (ball.radius + other_ball.radius) * (ball.radius + other_ball.radius);

    let disc = b * b - 4. * a * c;
    if disc < -EPSILON {
        return None;
    }

    let sqrt_disc = disc.max(0.).sqrt();

    // Entry time is the first root.
    let root0 = (-b - sqrt_disc) / (2. * a);
    let mid = -b / (2. * a);
    return Some((root0, mid));
}
