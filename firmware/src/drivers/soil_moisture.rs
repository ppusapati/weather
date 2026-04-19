/// Capacitive soil moisture sensor driver.
///
/// Uses ADC to read a capacitive probe that outputs a voltage inversely
/// proportional to soil moisture content. Supports dual-depth probes
/// for root-zone profiling.
///
/// # Hardware
///
/// - Capacitive soil moisture sensor v1.2 / v2.0
/// - ADC input with 3.3V reference
/// - Typical range: 1200 (saturated) to 3500 (dry air)

#[cfg(feature = "agriculture")]
use crate::config;

/// Soil moisture reading from a single probe.
#[cfg(feature = "agriculture")]
#[derive(Debug, Clone, Copy)]
pub struct SoilMoistureReading {
    /// Raw ADC value (0–4095).
    pub raw_adc: u16,
    /// Volumetric water content (0–100 %).
    pub moisture_pct: f32,
    /// Probe depth identifier.
    pub depth: ProbeDepth,
}

/// Probe installation depth.
#[cfg(feature = "agriculture")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeDepth {
    /// Shallow probe: 10–15 cm (root zone surface).
    Shallow,
    /// Deep probe: 30–40 cm (root zone base).
    Deep,
}

/// Capacitive soil moisture sensor.
#[cfg(feature = "agriculture")]
pub struct SoilMoistureSensor {
    /// ADC pin for shallow probe.
    adc_pin_shallow: u8,
    /// ADC pin for deep probe (optional).
    adc_pin_deep: Option<u8>,
    /// Calibration: ADC value at 0% moisture.
    cal_dry: u16,
    /// Calibration: ADC value at 100% moisture.
    cal_wet: u16,
    /// Whether the sensor has been initialized.
    initialized: bool,
}

#[cfg(feature = "agriculture")]
impl SoilMoistureSensor {
    pub fn new() -> Self {
        Self {
            adc_pin_shallow: config::SOIL_MOISTURE_ADC_PIN,
            adc_pin_deep: Some(config::SOIL_MOISTURE_DEEP_ADC_PIN),
            cal_dry: config::SOIL_MOISTURE_DRY_ADC,
            cal_wet: config::SOIL_MOISTURE_WET_ADC,
            initialized: false,
        }
    }

    pub fn init(&mut self) -> crate::error::Result<()> {
        log::info!(
            "Soil moisture: init shallow=GPIO{}, deep=GPIO{:?}",
            self.adc_pin_shallow,
            self.adc_pin_deep
        );
        // In real firmware: configure ADC channels
        self.initialized = true;
        Ok(())
    }

    /// Read the shallow probe.
    pub fn read_shallow(&self) -> crate::error::Result<SoilMoistureReading> {
        if !self.initialized {
            return Err(crate::error::Error::SensorNotReady);
        }
        // In real firmware: let raw = adc.read(self.adc_pin_shallow);
        let raw: u16 = 2400; // placeholder
        Ok(self.adc_to_reading(raw, ProbeDepth::Shallow))
    }

    /// Read the deep probe (if installed).
    pub fn read_deep(&self) -> crate::error::Result<SoilMoistureReading> {
        if !self.initialized {
            return Err(crate::error::Error::SensorNotReady);
        }
        if self.adc_pin_deep.is_none() {
            return Err(crate::error::Error::SensorNotFound(
                crate::error::SensorKind::SoilMoisture,
            ));
        }
        // In real firmware: let raw = adc.read(self.adc_pin_deep.unwrap());
        let raw: u16 = 2600; // placeholder
        Ok(self.adc_to_reading(raw, ProbeDepth::Deep))
    }

    /// Convert raw ADC to moisture percentage using linear interpolation.
    fn adc_to_reading(&self, raw: u16, depth: ProbeDepth) -> SoilMoistureReading {
        let range = self.cal_dry as f32 - self.cal_wet as f32;
        let moisture_pct = if range > 0.0 {
            ((self.cal_dry as f32 - raw as f32) / range * 100.0).clamp(0.0, 100.0)
        } else {
            0.0
        };

        SoilMoistureReading {
            raw_adc: raw,
            moisture_pct,
            depth,
        }
    }

    /// Set custom calibration values (from NVS or BLE provisioning).
    pub fn set_calibration(&mut self, dry_adc: u16, wet_adc: u16) {
        self.cal_dry = dry_adc;
        self.cal_wet = wet_adc;
        log::info!("Soil moisture: calibration set dry={} wet={}", dry_adc, wet_adc);
    }
}
