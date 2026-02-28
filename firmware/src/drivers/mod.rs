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

// Agriculture sensors
#[cfg(feature = "agriculture")]
pub mod leaf_wetness;
#[cfg(feature = "agriculture")]
pub mod soil_moisture;
#[cfg(feature = "agriculture")]
pub mod soil_temp;

// Solar sensors
#[cfg(feature = "solar")]
pub mod pyranometer;

// India sensors
#[cfg(feature = "india")]
pub mod pm25;

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
    #[cfg(feature = "agriculture")]
    pub soil_moisture: SensorStatus,
    #[cfg(feature = "agriculture")]
    pub soil_temp: SensorStatus,
    #[cfg(feature = "agriculture")]
    pub leaf_wetness: SensorStatus,
    #[cfg(feature = "solar")]
    pub pyranometer: SensorStatus,
    #[cfg(feature = "solar")]
    pub panel_temp: SensorStatus,
    #[cfg(feature = "india")]
    pub pm25: SensorStatus,
}

impl Default for SensorStatusMap {
    fn default() -> Self {
        Self {
            bme280: SensorStatus::NotFound,
            wind: SensorStatus::NotFound,
            rain: SensorStatus::NotFound,
            uv: SensorStatus::NotFound,
            light: SensorStatus::NotFound,
            #[cfg(feature = "agriculture")]
            soil_moisture: SensorStatus::NotFound,
            #[cfg(feature = "agriculture")]
            soil_temp: SensorStatus::NotFound,
            #[cfg(feature = "agriculture")]
            leaf_wetness: SensorStatus::NotFound,
            #[cfg(feature = "solar")]
            pyranometer: SensorStatus::NotFound,
            #[cfg(feature = "solar")]
            panel_temp: SensorStatus::NotFound,
            #[cfg(feature = "india")]
            pm25: SensorStatus::NotFound,
        }
    }
}
