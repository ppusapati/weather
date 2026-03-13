/// PM2.5 particulate matter sensor driver (Industrial Grade).
///
/// Drives a Sharp GP2Y1014AU0F (industrial replacement for GP2Y1010AU0F)
/// or compatible analog-output dust sensor via ADC.
///
/// Key industrial features:
/// - LED pulse timing per datasheet (0.32 ms pulse, sample at 0.28 ms)
/// - Multi-sample averaging (configurable, default 10 samples) for noise rejection
/// - Sensor warm-up enforcement (minimum 10 seconds after power-on)
/// - NaN/infinity/negative output guards
/// - Configurable calibration curve (baseline + sensitivity)
/// - Out-of-range detection with hysteresis

use crate::config;
use crate::error::{Error, Result, SensorKind};

/// Number of ADC samples to average per reading (noise rejection).
const PM25_SAMPLE_COUNT: usize = 10;

/// Minimum warm-up time after power-on before readings are valid (ms).
const PM25_WARMUP_MS: u64 = 10_000;

/// Maximum valid PM2.5 concentration (µg/m³). Industrial environments
/// can see high dust but >1000 µg/m³ indicates sensor fault.
const PM25_MAX_UGM3: f32 = 1000.0;

/// GP2Y1014AU0F LED pulse width in microseconds (datasheet: 0.32 ms).
const LED_PULSE_US: u32 = 320;

/// Sampling point within LED pulse in microseconds (datasheet: 0.28 ms after LED on).
const SAMPLE_DELAY_US: u32 = 280;

/// PM2.5 reading.
#[derive(Debug, Clone, Copy)]
pub struct Pm25Reading {
    /// PM2.5 concentration in µg/m³.
    pub pm25_ugm3: f32,
    /// Raw ADC voltage (mV) for diagnostics.
    pub raw_voltage_mv: f32,
}

/// PM2.5 sensor driver (ADC-based, industrial grade).
pub struct Pm25Sensor {
    /// ADC pin number.
    adc_pin: u8,
    /// Baseline voltage (mV) at 0 µg/m³ (GP2Y1014AU0F typical: 600 mV).
    baseline_mv: f32,
    /// Sensitivity (mV per µg/m³) (GP2Y1014AU0F typical: 5.0 mV/(µg/m³)).
    sensitivity_mv_per_ugm3: f32,
    /// Whether the sensor has completed warm-up.
    warmed_up: bool,
    /// Power-on timestamp (ms) for warm-up tracking.
    power_on_ms: u64,
    /// EMA-filtered output for trend smoothing.
    ema_value: Option<f32>,
    /// Number of consecutive out-of-range readings (for fault detection).
    out_of_range_count: u8,
}

impl Pm25Sensor {
    pub fn new() -> Self {
        Self {
            adc_pin: config::PM25_SENSOR_ADC_PIN,
            baseline_mv: 600.0,          // GP2Y1014AU0F typical baseline
            sensitivity_mv_per_ugm3: 5.0, // GP2Y1014AU0F typical sensitivity
            warmed_up: false,
            power_on_ms: 0,
            ema_value: None,
            out_of_range_count: 0,
        }
    }

    /// Initialize the sensor and record power-on time.
    pub fn init(&mut self, now_ms: u64) -> Result<()> {
        self.power_on_ms = now_ms;
        self.warmed_up = false;
        self.ema_value = None;
        self.out_of_range_count = 0;
        // In real firmware: configure ADC channel, configure LED drive GPIO as output
        log::info!(
            "PM2.5 sensor (GP2Y1014AU0F) initialized on GPIO {}, warm-up {}ms",
            self.adc_pin,
            PM25_WARMUP_MS
        );
        Ok(())
    }

    /// Check if the sensor has completed its warm-up period.
    pub fn is_ready(&self, now_ms: u64) -> bool {
        self.warmed_up || now_ms.saturating_sub(self.power_on_ms) >= PM25_WARMUP_MS
    }

    /// Read PM2.5 concentration from ADC with multi-sample averaging.
    ///
    /// `adc_samples` should contain `PM25_SAMPLE_COUNT` raw ADC readings
    /// taken with proper LED pulse timing. In real firmware, each sample
    /// involves: LED ON → wait SAMPLE_DELAY_US → ADC read → LED OFF → wait.
    ///
    /// For single-sample fallback, pass a slice with one element.
    pub fn read(&mut self, adc_samples: &[u16], now_ms: u64) -> Result<Pm25Reading> {
        // Enforce warm-up period
        if !self.is_ready(now_ms) {
            return Err(Error::SensorNotReady);
        }
        self.warmed_up = true;

        if adc_samples.is_empty() {
            return Err(Error::SensorReadFailed(SensorKind::Pm25));
        }

        // Multi-sample averaging with outlier rejection
        let voltage_mv = self.average_samples(adc_samples);

        // NaN/infinity guard
        if !voltage_mv.is_finite() {
            return Err(Error::SensorReadFailed(SensorKind::Pm25));
        }

        // Convert voltage to µg/m³ using calibration curve
        let pm25_raw = (voltage_mv - self.baseline_mv) / self.sensitivity_mv_per_ugm3;

        // Clamp negative values (below baseline = clean air = 0)
        let pm25 = pm25_raw.max(0.0);

        // Out-of-range detection with consecutive-count fault trigger
        if pm25 > PM25_MAX_UGM3 {
            self.out_of_range_count = self.out_of_range_count.saturating_add(1);
            if self.out_of_range_count >= 5 {
                log::error!("PM2.5 sensor: {} consecutive out-of-range readings", self.out_of_range_count);
                return Err(Error::SensorOutOfRange(SensorKind::Pm25));
            }
            // Single spike: report clamped value but don't error
            return Ok(Pm25Reading {
                pm25_ugm3: PM25_MAX_UGM3,
                raw_voltage_mv: voltage_mv,
            });
        }
        self.out_of_range_count = 0;

        // Apply EMA filter for trend smoothing
        let filtered = match self.ema_value {
            Some(prev) => {
                let alpha = config::EMA_ALPHA_PM25;
                alpha * pm25 + (1.0 - alpha) * prev
            }
            None => pm25,
        };
        self.ema_value = Some(filtered);

        Ok(Pm25Reading {
            pm25_ugm3: filtered,
            raw_voltage_mv: voltage_mv,
        })
    }

    /// Set calibration parameters (e.g., from NVS or factory calibration).
    pub fn set_calibration(&mut self, baseline_mv: f32, sensitivity_mv_per_ugm3: f32) {
        if baseline_mv.is_finite() && baseline_mv > 0.0 {
            self.baseline_mv = baseline_mv;
        }
        if sensitivity_mv_per_ugm3.is_finite() && sensitivity_mv_per_ugm3 > 0.1 {
            self.sensitivity_mv_per_ugm3 = sensitivity_mv_per_ugm3;
        }
        log::info!(
            "PM2.5 calibration updated: baseline={}mV, sensitivity={}mV/(µg/m³)",
            self.baseline_mv,
            self.sensitivity_mv_per_ugm3
        );
    }

    /// Get LED pulse timing parameters for the acquisition routine.
    pub fn timing() -> (u32, u32) {
        (LED_PULSE_US, SAMPLE_DELAY_US)
    }

    /// Get the recommended number of samples per reading.
    pub fn sample_count() -> usize {
        PM25_SAMPLE_COUNT
    }

    // --- Private methods ---

    /// Average ADC samples with median-of-three outlier rejection.
    /// Converts raw ADC counts to millivolts.
    fn average_samples(&self, samples: &[u16]) -> f32 {
        if samples.len() == 1 {
            return (samples[0] as f32 / config::ADC_RESOLUTION as f32) * config::ADC_VREF * 1000.0;
        }

        // Sort for outlier rejection (use insertion sort for small N)
        let mut sorted = [0u16; PM25_SAMPLE_COUNT];
        let n = samples.len().min(PM25_SAMPLE_COUNT);
        sorted[..n].copy_from_slice(&samples[..n]);
        for i in 1..n {
            let key = sorted[i];
            let mut j = i;
            while j > 0 && sorted[j - 1] > key {
                sorted[j] = sorted[j - 1];
                j -= 1;
            }
            sorted[j] = key;
        }

        // Trim top and bottom 20% for robust mean
        let trim = n / 5;
        let trimmed = &sorted[trim..n - trim];
        if trimmed.is_empty() {
            // Fallback: use median
            let median = sorted[n / 2];
            return (median as f32 / config::ADC_RESOLUTION as f32) * config::ADC_VREF * 1000.0;
        }

        let sum: u32 = trimmed.iter().map(|&s| s as u32).sum();
        let avg = sum as f32 / trimmed.len() as f32;
        (avg / config::ADC_RESOLUTION as f32) * config::ADC_VREF * 1000.0
    }
}
