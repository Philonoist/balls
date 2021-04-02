use legion::*;
use log::info;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const FRAME_TIME_CAP: i64 = 16;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SimulationData {
    pub time: f64,
    pub next_time: f64,
    pub last_simulated: i64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SimulationConfig {
    pub time_delta: f64,
}

pub fn init_simulation(resources: &mut Resources, simulation_config: SimulationConfig) {
    resources.insert(SimulationData {
        time: 0.0,
        next_time: simulation_config.time_delta,
        last_simulated: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64,
    });
    resources.insert(simulation_config);
}

#[system]
pub fn advance_time(
    #[resource] simulation_data: &mut SimulationData,
    #[resource] simulation_config: &SimulationConfig,
) {
    simulation_data.time = simulation_data.next_time;
    simulation_data.next_time += simulation_config.time_delta;
    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    info!(
        "Frame time: {}",
        current_time - simulation_data.last_simulated
    );
    let ms_to_sleep = std::cmp::max(
        0,
        FRAME_TIME_CAP - (current_time - simulation_data.last_simulated),
    ) as u64;
    std::thread::sleep(Duration::from_millis(ms_to_sleep));
    simulation_data.last_simulated = current_time + (ms_to_sleep as i64);
}

pub fn adjust_simulation_speed(resources: &mut Resources, factor: f64) {
    let mut simulation_config = resources.get_mut::<SimulationConfig>().unwrap();
    simulation_config.time_delta *= factor;
}
