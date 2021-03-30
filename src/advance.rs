use legion::*;

use crate::{ball::Ball, simulation::SimulationData};

#[system(par_for_each)]
pub fn advance_balls(ball: &mut Ball, #[resource] simulation_data: &SimulationData) {
    advance_single_ball(ball, simulation_data.next_time);
}

pub fn advance_single_ball(ball: &mut Ball, next_time: f32) {
    ball.position += ball.velocity * (next_time - ball.initial_time);
    ball.initial_time = next_time;
}
