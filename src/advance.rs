use legion::*;

use crate::{
    ball::{Ball, Trail, Trails},
    simulation::SimulationData,
};

#[system(par_for_each)]
pub fn clear_trails(trails: &mut Trails) {
    trails.trails.clear();
}

#[system(par_for_each)]
pub fn advance_balls(
    ball: &mut Ball,
    trails: &mut Trails,
    #[resource] simulation_data: &SimulationData,
) {
    advance_single_ball(ball, trails, simulation_data.next_time);
}

pub fn advance_single_ball(ball: &mut Ball, trails: &mut Trails, next_time: f64) {
    let new_position = ball.position + ball.velocity * (next_time - ball.initial_time);
    if next_time > ball.initial_time {
        trails.trails.push(Trail {
            position0: ball.position,
            position1: new_position,
            initial_time: ball.initial_time,
            final_time: next_time,
        });
    }
    ball.position = new_position;
    ball.initial_time = next_time;
}
