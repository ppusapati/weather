/// Core firmware modules.
///
/// - data_pipeline: Sensor data calibration, filtering, validation, and packaging
/// - scheduler: Task scheduling and timing coordination
/// - power: Power management and sleep modes
/// - ota: Over-the-air firmware update manager
/// - mode_manager: SCADA/Cloud/Hybrid operating mode control

pub mod data_pipeline;
pub mod ota;
pub mod power;
pub mod scheduler;

// Mode manager for dual-MCU SCADA/Cloud/Hybrid operation
#[cfg(feature = "stm32")]
pub mod mode_manager;
