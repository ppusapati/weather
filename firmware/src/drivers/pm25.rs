/// PM2.5 particulate matter sensor driver.
///
/// Reads PM2.5 concentration via ADC (analog output sensor such as
/// Sharp GP2Y1010AU0F or Plantower PMS5003 analog output).
///
/// ADC voltage is converted to µg/m³ using a linear calibration curve.

use crate::config;
use crate::error::{Error, Result, SensorKind};

/// PM2.5 reading.
#[derive(Debug, Clone, Copy)]
pub struct Pm25Reading {
    /// PM2.5 concentration in µg/m³.
    pub pm25_ugm3: f32,
}

/// PM2.5 sensor driver (ADC-based).
pub struct Pm25Sensor {
    /// ADC pin number.
    adc_pin: u8,
    /// Baseline voltage (mV) at 0 µg/m³.
    baseline_mv: f32,
    /// Sensitivity (mV per µg/m³).
    sensitivity_mv: f32,
}

impl Pm25Sensor {
    pub fn new() -> Self {
        Self {
            adc_pin: config::PM25_SENSOR_ADC_PIN,
            baseline_mv: 900.0,   // typical for GP2Y1010AU0F
            sensitivity_mv: 5.0,  // mV per µg/m³
        }
    }

    pub fn init(&mut self) -> Result<()> {
        // In real firmware: configure ADC channel for PM2.5 sensor
        log::info!("PM2.5 sensor initialized on GPIO {}", self.adc_pin);
        Ok(())
    }

    /// Read PM2.5 concentration from ADC.
    pub fn read(&mut self, _adc_raw: u16) -> Result<Pm25Reading> {
        // Convert ADC to voltage
        let voltage_mv = (_adc_raw as f32 / config::ADC_RESOLUTION as f32) * config::ADC_VREF * 1000.0;

        // Convert voltage to µg/m³
        let pm25 = ((voltage_mv - self.baseline_mv) / self.sensitivity_mv).max(0.0);

        if pm25 > 1000.0 {
            return Err(Error::SensorOutOfRange(SensorKind::Pm25));
        }

        Ok(Pm25Reading { pm25_ugm3: pm25 })
    }
}
