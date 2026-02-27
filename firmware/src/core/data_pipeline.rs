/// Data pipeline: calibration, filtering, validation, derived values, and packaging.
///
/// Raw sensor readings flow through:
/// 1. Calibration (apply offsets/gains from NVS)
/// 2. EMA filtering (smooth noise)
/// 3. Validation (range checks, stuck detection)
/// 4. Derived values (heat index, dew point, wind chill)
/// 5. Packaging into WeatherReading struct

use crate::config;
use crate::drivers::SensorStatusMap;
use serde::{Deserialize, Serialize};

/// Complete weather reading after all pipeline stages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherReading {
    pub timestamp_ms: u64,
    pub temperature_c: Option<f32>,
    pub humidity_pct: Option<f32>,
    pub pressure_hpa: Option<f32>,
    pub wind_speed_kmh: Option<f32>,
    pub wind_dir_deg: Option<u16>,
    pub rain_mm: f32,
    pub rain_rate_mm_hr: Option<f32>,
    pub uv_index: Option<f32>,
    pub light_lux: Option<f32>,
    // Derived values
    pub heat_index_c: Option<f32>,
    pub dew_point_c: Option<f32>,
    pub wind_chill_c: Option<f32>,
    // Status
    pub sensor_status: SensorStatusMap,
    // Industry-specific readings
    #[cfg(feature = "agriculture")]
    pub agriculture: Option<crate::industry::agriculture::AgricultureReading>,
    #[cfg(feature = "solar")]
    pub solar: Option<crate::industry::solar::SolarReading>,
}

impl Default for WeatherReading {
    fn default() -> Self {
        Self {
            timestamp_ms: 0,
            temperature_c: None,
            humidity_pct: None,
            pressure_hpa: None,
            wind_speed_kmh: None,
            wind_dir_deg: None,
            rain_mm: 0.0,
            rain_rate_mm_hr: None,
            uv_index: None,
            light_lux: None,
            heat_index_c: None,
            dew_point_c: None,
            wind_chill_c: None,
            sensor_status: SensorStatusMap::default(),
            #[cfg(feature = "agriculture")]
            agriculture: None,
            #[cfg(feature = "solar")]
            solar: None,
        }
    }
}

/// EMA filter state for a single sensor value.
#[derive(Debug, Clone, Copy)]
struct EmaFilter {
    alpha: f32,
    value: Option<f32>,
}

impl EmaFilter {
    fn new(alpha: f32) -> Self {
        Self { alpha, value: None }
    }

    fn update(&mut self, new_value: f32) -> f32 {
        let filtered = match self.value {
            Some(prev) => self.alpha * new_value + (1.0 - self.alpha) * prev,
            None => new_value,
        };
        self.value = Some(filtered);
        filtered
    }

    fn get(&self) -> Option<f32> {
        self.value
    }
}

/// Stuck sensor detector.
#[derive(Debug, Clone, Copy)]
struct StuckDetector {
    last_value: Option<f32>,
    same_count: u32,
    threshold: u32,
}

impl StuckDetector {
    fn new(threshold: u32) -> Self {
        Self {
            last_value: None,
            same_count: 0,
            threshold,
        }
    }

    /// Returns true if the sensor appears stuck.
    fn check(&mut self, value: f32) -> bool {
        match self.last_value {
            Some(prev) if (value - prev).abs() < config::STUCK_SENSOR_EPSILON => {
                self.same_count += 1;
            }
            _ => {
                self.same_count = 0;
            }
        }
        self.last_value = Some(value);
        self.same_count >= self.threshold
    }
}

/// Data pipeline processing engine.
pub struct DataPipeline {
    // EMA filters
    temp_filter: EmaFilter,
    humidity_filter: EmaFilter,
    pressure_filter: EmaFilter,
    wind_speed_filter: EmaFilter,
    wind_dir_filter: EmaFilter,
    uv_filter: EmaFilter,
    light_filter: EmaFilter,

    // Stuck detectors
    temp_stuck: StuckDetector,
    humidity_stuck: StuckDetector,
    pressure_stuck: StuckDetector,

    // Calibration offsets
    cal_temp_offset: f32,
    cal_humidity_offset: f32,
    cal_pressure_offset: f32,
    cal_wind_dir_offset: f32,
}

impl DataPipeline {
    pub fn new() -> Self {
        let threshold = config::STUCK_SENSOR_THRESHOLD;
        Self {
            temp_filter: EmaFilter::new(config::EMA_ALPHA_TEMPERATURE),
            humidity_filter: EmaFilter::new(config::EMA_ALPHA_HUMIDITY),
            pressure_filter: EmaFilter::new(config::EMA_ALPHA_PRESSURE),
            wind_speed_filter: EmaFilter::new(config::EMA_ALPHA_WIND_SPEED),
            wind_dir_filter: EmaFilter::new(config::EMA_ALPHA_WIND_DIR),
            uv_filter: EmaFilter::new(config::EMA_ALPHA_UV),
            light_filter: EmaFilter::new(config::EMA_ALPHA_LIGHT),
            temp_stuck: StuckDetector::new(threshold),
            humidity_stuck: StuckDetector::new(threshold),
            pressure_stuck: StuckDetector::new(threshold),
            cal_temp_offset: 0.0,
            cal_humidity_offset: 0.0,
            cal_pressure_offset: 0.0,
            cal_wind_dir_offset: 0.0,
        }
    }

    /// Set calibration offsets (loaded from NVS).
    pub fn set_calibration(
        &mut self,
        temp_offset: f32,
        humidity_offset: f32,
        pressure_offset: f32,
        wind_dir_offset: f32,
    ) {
        self.cal_temp_offset = temp_offset;
        self.cal_humidity_offset = humidity_offset;
        self.cal_pressure_offset = pressure_offset;
        self.cal_wind_dir_offset = wind_dir_offset;
    }

    /// Process raw sensor data through the full pipeline.
    pub fn process(&mut self, raw: &RawSensorData, timestamp_ms: u64) -> WeatherReading {
        let mut reading = WeatherReading {
            timestamp_ms,
            ..WeatherReading::default()
        };
        let mut status = SensorStatusMap::default();

        // --- Temperature ---
        if let Some(temp_raw) = raw.temperature_c {
            let calibrated = temp_raw + self.cal_temp_offset;
            if validate_range(calibrated, config::TEMP_MIN_C, config::TEMP_MAX_C) {
                let filtered = self.temp_filter.update(calibrated);
                if self.temp_stuck.check(filtered) {
                    status.bme280 = crate::drivers::SensorStatus::Degraded;
                } else {
                    status.bme280 = crate::drivers::SensorStatus::Ok;
                }
                reading.temperature_c = Some(filtered);
            } else {
                status.bme280 = crate::drivers::SensorStatus::Error;
            }
        }

        // --- Humidity ---
        if let Some(hum_raw) = raw.humidity_pct {
            let calibrated = (hum_raw + self.cal_humidity_offset).clamp(0.0, 100.0);
            if validate_range(calibrated, config::HUMIDITY_MIN_PCT, config::HUMIDITY_MAX_PCT) {
                let filtered = self.humidity_filter.update(calibrated);
                if self.humidity_stuck.check(filtered) {
                    // BME280 already set above, just note degradation
                }
                reading.humidity_pct = Some(filtered);
            }
        }

        // --- Pressure ---
        if let Some(press_raw) = raw.pressure_hpa {
            let calibrated = press_raw + self.cal_pressure_offset;
            if validate_range(calibrated, config::PRESSURE_MIN_HPA, config::PRESSURE_MAX_HPA) {
                let filtered = self.pressure_filter.update(calibrated);
                if self.pressure_stuck.check(filtered) {
                    // Note degradation
                }
                reading.pressure_hpa = Some(filtered);
            }
        }

        // --- Wind Speed ---
        if let Some(wind_raw) = raw.wind_speed_kmh {
            if wind_raw >= 0.0 && wind_raw <= config::WIND_SPEED_MAX_KMH {
                reading.wind_speed_kmh = Some(self.wind_speed_filter.update(wind_raw));
                status.wind = crate::drivers::SensorStatus::Ok;
            } else {
                status.wind = crate::drivers::SensorStatus::Error;
            }
        }

        // --- Wind Direction ---
        if let Some(dir_raw) = raw.wind_direction_deg {
            let calibrated = ((dir_raw as f32 + self.cal_wind_dir_offset) % 360.0 + 360.0) % 360.0;
            reading.wind_dir_deg = Some(
                self.wind_dir_filter.update(calibrated) as u16 % 360,
            );
        }

        // --- Rain ---
        reading.rain_mm = raw.rain_total_mm;
        reading.rain_rate_mm_hr = raw.rain_rate_mm_hr;
        status.rain = crate::drivers::SensorStatus::Ok;

        // --- UV Index ---
        if let Some(uv_raw) = raw.uv_index {
            if uv_raw >= 0.0 && uv_raw <= config::UV_INDEX_MAX {
                reading.uv_index = Some(self.uv_filter.update(uv_raw));
                status.uv = crate::drivers::SensorStatus::Ok;
            } else {
                status.uv = crate::drivers::SensorStatus::Error;
            }
        }

        // --- Ambient Light ---
        if let Some(lux_raw) = raw.light_lux {
            if lux_raw >= 0.0 && lux_raw <= config::LIGHT_MAX_LUX {
                reading.light_lux = Some(self.light_filter.update(lux_raw));
                status.light = crate::drivers::SensorStatus::Ok;
            } else {
                status.light = crate::drivers::SensorStatus::Error;
            }
        }

        // --- Derived Values ---
        reading.heat_index_c = compute_heat_index(reading.temperature_c, reading.humidity_pct);
        reading.dew_point_c = compute_dew_point(reading.temperature_c, reading.humidity_pct);
        reading.wind_chill_c = compute_wind_chill(reading.temperature_c, reading.wind_speed_kmh);

        reading.sensor_status = status;
        reading
    }
}

/// Raw sensor data collected from all drivers.
#[derive(Debug, Clone, Default)]
pub struct RawSensorData {
    pub temperature_c: Option<f32>,
    pub humidity_pct: Option<f32>,
    pub pressure_hpa: Option<f32>,
    pub wind_speed_kmh: Option<f32>,
    pub wind_direction_deg: Option<f32>,
    pub rain_total_mm: f32,
    pub rain_rate_mm_hr: Option<f32>,
    pub uv_index: Option<f32>,
    pub light_lux: Option<f32>,
}

/// Validate that a value is within an expected range.
fn validate_range(value: f32, min: f32, max: f32) -> bool {
    value >= min && value <= max && value.is_finite()
}

/// Compute heat index using the Rothfusz regression equation.
/// Only valid when T >= 27°C and H >= 40%.
fn compute_heat_index(temp_c: Option<f32>, humidity: Option<f32>) -> Option<f32> {
    let t = temp_c?;
    let h = humidity?;

    if t < 27.0 || h < 40.0 {
        return None;
    }

    let hi = -8.785
        + 1.611 * t
        + 2.339 * h
        - 0.146 * t * h
        - 0.013 * t * t
        - 0.016 * h * h
        + 0.002 * t * t * h
        + 0.001 * t * h * h
        - 0.000004 * t * t * h * h;

    Some(hi)
}

/// Compute dew point using the Magnus formula.
fn compute_dew_point(temp_c: Option<f32>, humidity: Option<f32>) -> Option<f32> {
    let t = temp_c?;
    let h = humidity?;

    if h <= 0.0 {
        return None;
    }

    let a = 17.67;
    let b = 243.5;
    let gamma = libm::logf(h / 100.0) + (a * t) / (b + t);
    let dew_point = (b * gamma) / (a - gamma);

    Some(dew_point)
}

/// Compute wind chill (only valid when T < 10°C and wind > 4.8 km/h).
fn compute_wind_chill(temp_c: Option<f32>, wind_kmh: Option<f32>) -> Option<f32> {
    let t = temp_c?;
    let v = wind_kmh?;

    if t >= 10.0 || v <= 4.8 {
        return None;
    }

    let v016 = libm::powf(v, 0.16);
    let wc = 13.12 + 0.6215 * t - 11.37 * v016 + 0.3965 * t * v016;

    Some(wc)
}
