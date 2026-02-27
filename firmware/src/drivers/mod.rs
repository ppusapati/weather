/// Sensor drivers for the weather station.
///
/// Each driver implements a consistent interface:
/// - `new()` — construct with I2C/GPIO handle
/// - `init()` — probe and configure the sensor
/// - `read()` — take a measurement

pub mod bme280;
pub mod light;
pub mod rain;
pub mod uv;
pub mod wind;

use serde::{Deserialize, Serialize};

/// Status of an individual sensor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SensorStatus {
    /// Sensor is operating normally.
    Ok,
    /// Sensor is responding but readings may be unreliable.
    Degraded,
    /// Sensor is not responding.
    Error,
    /// Sensor is not present / not initialized.
    NotFound,
}

impl SensorStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            SensorStatus::Ok => "ok",
            SensorStatus::Degraded => "degraded",
            SensorStatus::Error => "error",
            SensorStatus::NotFound => "not_found",
        }
    }
}

/// Aggregate status of all sensors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorStatusMap {
    pub bme280: SensorStatus,
    pub wind: SensorStatus,
    pub rain: SensorStatus,
    pub uv: SensorStatus,
    pub light: SensorStatus,
}

impl Default for SensorStatusMap {
    fn default() -> Self {
        Self {
            bme280: SensorStatus::NotFound,
            wind: SensorStatus::NotFound,
            rain: SensorStatus::NotFound,
            uv: SensorStatus::NotFound,
            light: SensorStatus::NotFound,
        }
    }
}
