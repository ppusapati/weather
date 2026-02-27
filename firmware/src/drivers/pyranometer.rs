/// Pyranometer / solar irradiance sensor driver.
///
/// Measures global horizontal irradiance (GHI) using an ML8511 UV/solar
/// sensor or a silicon photodiode-based pyranometer. Output is analog
/// voltage proportional to irradiance.
///
/// # Hardware
///
/// - ML8511 or SP-110 silicon pyranometer
/// - ADC input with known calibration factor
/// - Range: 0 to ~1500 W/m² (full sunlight + reflected)

#[cfg(feature = "solar")]
use crate::config;

/// Solar irradiance reading.
#[cfg(feature = "solar")]
#[derive(Debug, Clone, Copy)]
pub struct IrradianceReading {
    /// Raw ADC value.
    pub raw_adc: u16,
    /// Voltage at sensor output (mV).
    pub voltage_mv: f32,
    /// Global Horizontal Irradiance (W/m²).
    pub irradiance_w_m2: f32,
}

/// Sky condition derived from irradiance vs expected clear-sky.
#[cfg(feature = "solar")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkyCondition {
    /// Clear sky (irradiance > 80% of expected).
    Clear,
    /// Partly cloudy (40–80% of expected).
    PartlyCloudy,
    /// Overcast (< 40% of expected).
    Overcast,
    /// Night / below detection threshold.
    Night,
}

/// Pyranometer sensor.
#[cfg(feature = "solar")]
pub struct Pyranometer {
    adc_pin: u8,
    /// mV per unit irradiance.
    cal_mv_per_unit: f32,
    /// Baseline voltage at zero irradiance.
    baseline_mv: f32,
    /// Conversion factor to W/m².
    to_w_m2: f32,
    initialized: bool,
}

#[cfg(feature = "solar")]
impl Pyranometer {
    pub fn new() -> Self {
        Self {
            adc_pin: config::PYRANOMETER_ADC_PIN,
            cal_mv_per_unit: config::PYRANOMETER_MV_PER_UNIT,
            baseline_mv: config::PYRANOMETER_BASELINE_MV,
            to_w_m2: config::MW_CM2_TO_W_M2,
            initialized: false,
        }
    }

    pub fn init(&mut self) -> crate::error::Result<()> {
        log::info!("Pyranometer: init on GPIO{}", self.adc_pin);
        // In real firmware: configure ADC channel, set attenuation
        self.initialized = true;
        Ok(())
    }

    /// Read solar irradiance.
    pub fn read(&self) -> crate::error::Result<IrradianceReading> {
        if !self.initialized {
            return Err(crate::error::Error::SensorNotReady);
        }

        // In real firmware: let raw = adc.read(self.adc_pin);
        let raw: u16 = 2048; // placeholder

        // Convert ADC to voltage: (raw / 4095) * 3300 mV
        let voltage_mv = (raw as f32 / config::ADC_RESOLUTION as f32) * (config::ADC_VREF * 1000.0);

        // Convert voltage to irradiance
        let delta_mv = (voltage_mv - self.baseline_mv).max(0.0);
        let irradiance_raw = delta_mv / self.cal_mv_per_unit;
        let irradiance_w_m2 = (irradiance_raw * self.to_w_m2).clamp(0.0, config::IRRADIANCE_MAX_W_M2);

        Ok(IrradianceReading {
            raw_adc: raw,
            voltage_mv,
            irradiance_w_m2,
        })
    }

    /// Determine sky condition from current irradiance and expected clear-sky value.
    pub fn sky_condition(&self, irradiance: f32, expected_clear_sky: f32) -> SkyCondition {
        if irradiance < 10.0 {
            return SkyCondition::Night;
        }
        if expected_clear_sky <= 0.0 {
            return SkyCondition::Night;
        }

        let ratio = irradiance / expected_clear_sky;
        match ratio {
            r if r > 0.8 => SkyCondition::Clear,
            r if r > 0.4 => SkyCondition::PartlyCloudy,
            _ => SkyCondition::Overcast,
        }
    }

    /// Set custom calibration.
    pub fn set_calibration(&mut self, mv_per_unit: f32, baseline_mv: f32) {
        self.cal_mv_per_unit = mv_per_unit;
        self.baseline_mv = baseline_mv;
        log::info!(
            "Pyranometer: cal set mv/unit={:.2} baseline={:.0}mV",
            mv_per_unit,
            baseline_mv
        );
    }
}
