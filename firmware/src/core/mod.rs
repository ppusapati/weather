/// Core firmware modules.
///
/// - data_pipeline: Sensor data calibration, filtering, validation, and packaging
/// - scheduler: Task scheduling and timing coordination
/// - power: Power management and sleep modes
/// - ota: Over-the-air firmware update manager

pub mod data_pipeline;
pub mod ota;
pub mod power;
pub mod scheduler;
