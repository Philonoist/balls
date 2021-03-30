use legion::*;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SimulationData {
    pub time: f32,
    pub next_time: f32,
    pub last_simulated: i64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SimulationConfig {
    pub time_delta: f32,
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
    println!(
        "Frame time: {}",
        current_time - simulation_data.last_simulated
    );
    let ms_to_sleep = std::cmp::max(0, 16 - (current_time - simulation_data.last_simulated)) as u64;
    std::thread::sleep(Duration::from_millis(ms_to_sleep));
    simulation_data.last_simulated = current_time + (ms_to_sleep as i64);
}

pub fn adjust_simulation_speed(resources: &mut Resources, factor: f32) {
    if let Some(mut simulation_config) = resources.get_mut::<SimulationConfig>() {
        simulation_config.time_delta *= factor;
    }
}
