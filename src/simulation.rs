use legion::*;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SimulationTime {
    pub time: f32,
    pub last_simulated: i64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SimulationParams {
    pub time_delta: f32,
}

#[system]
pub fn advance_time(
    #[resource] time: &mut SimulationTime,
    #[resource] simulation_params: &SimulationParams,
) {
    time.time += simulation_params.time_delta;
    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    println!("Frame time: {}", current_time - time.last_simulated);
    let ms_to_sleep = std::cmp::max(0, 16 - (current_time - time.last_simulated)) as u64;
    std::thread::sleep(Duration::from_millis(ms_to_sleep));
    time.last_simulated = current_time + (ms_to_sleep as i64);
}
