use maybe_owned::MaybeOwned;

use crate::{advance::advance_single_ball, ball::Ball, wall::Wall};

use super::collidable::Collidable;

pub fn collide<'a>(
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
    // static mut it: i64 = 0;
    // unsafe {
    //     it += 1;
    //     if it > 1000000 {
    //         println!("proj: {}, v0: {}", proj, ball0.velocity);
    //     }
    // }
    if proj < 0. {
        let d2 = dx.dot(&dx);
        let a = 2. / (mass0 + mass1) * proj / d2 * dx;
        ball0.velocity -= mass1 * a;
        // println!("v1: {}", ball0.velocity);
        if ball0.velocity.norm() > 1000. {
            ball0.velocity *= 1000. / ball0.velocity.norm();
        }
        ball1.velocity += mass0 * a;
        if ball1.velocity.norm() > 1000. {
            ball1.velocity *= 1000. / ball1.velocity.norm();
        }
    }
    (
        Some(Collidable::Ball(MaybeOwned::from(ball0))),
        Some(Collidable::Ball(MaybeOwned::from(ball1))),
    )
}
