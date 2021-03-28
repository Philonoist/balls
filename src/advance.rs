use legion::*;

use crate::{
    ball::Ball,
    simulation::{SimulationParams, SimulationTime},
};

#[system(par_for_each)]
pub fn advance_balls(
    ball: &mut Ball,
    #[resource] time: &SimulationTime,
    #[resource] simulation_params: &SimulationParams,
) {
    let next_time = time.time + simulation_params.time_delta;
    advance_single_ball(ball, next_time);
}

pub fn advance_single_ball(ball: &mut Ball, next_t: f32) {
    ball.position += ball.velocity * (next_t - ball.initial_time);
    ball.initial_time = next_t;
}
