/// Leaf wetness sensor driver.
///
/// Resistive leaf wetness sensor that detects surface moisture on plant leaves.
/// Critical for disease prediction models (fungal infection risk).
///
/// # Hardware
///
/// - Resistive leaf wetness sensor (printed circuit grid)
/// - ADC input: high impedance when dry, low when wet
/// - Typical range: 0 (submerged) to 4095 (dry)

#[cfg(feature = "agriculture")]
use crate::config;

/// Leaf wetness state.
#[cfg(feature = "agriculture")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WetnessState {
    /// Leaf surface is dry.
    Dry,
    /// Leaf surface has dew/light moisture.
    Dew,
    /// Leaf surface is wet (rain/heavy dew).
    Wet,
    /// Sensor submerged or error.
    Saturated,
}

/// Leaf wetness reading.
#[cfg(feature = "agriculture")]
#[derive(Debug, Clone, Copy)]
pub struct LeafWetnessReading {
    /// Raw ADC value (0–4095).
    pub raw_adc: u16,
    /// Wetness percentage (0 = dry, 100 = saturated).
    pub wetness_pct: f32,
    /// Discrete wetness state.
    pub state: WetnessState,
    /// Duration of continuous wetness (minutes).
    pub wet_duration_min: u32,
}

/// Leaf wetness sensor.
#[cfg(feature = "agriculture")]
pub struct LeafWetnessSensor {
    adc_pin: u8,
    threshold: u16,
    /// Timestamp when wetness was first detected (ms since boot).
    wet_start_ms: Option<u64>,
    /// Accumulated wet duration in the current period (minutes).
    wet_duration_min: u32,
    initialized: bool,
}

#[cfg(feature = "agriculture")]
impl LeafWetnessSensor {
    pub fn new() -> Self {
        Self {
            adc_pin: config::LEAF_WETNESS_ADC_PIN,
            threshold: config::LEAF_WETNESS_THRESHOLD,
            wet_start_ms: None,
            wet_duration_min: 0,
            initialized: false,
        }
    }

    pub fn init(&mut self) -> crate::error::Result<()> {
        log::info!("Leaf wetness: init on GPIO{}", self.adc_pin);
        self.initialized = true;
        Ok(())
    }

    /// Read leaf wetness and update duration tracking.
    pub fn read(&mut self, now_ms: u64) -> crate::error::Result<LeafWetnessReading> {
        if !self.initialized {
            return Err(crate::error::Error::SensorNotReady);
        }

        // In real firmware: let raw = adc.read(self.adc_pin);
        let raw: u16 = 3000; // placeholder

        let wetness_pct = ((4095u16.saturating_sub(raw)) as f32 / 4095.0 * 100.0).clamp(0.0, 100.0);

        let state = match raw {
            0..=500 => WetnessState::Saturated,
            501..=1500 => WetnessState::Wet,
            1501..=2500 => WetnessState::Dew,
            _ => WetnessState::Dry,
        };

        // Track wetness duration
        match state {
            WetnessState::Dry => {
                if self.wet_start_ms.is_some() {
                    self.wet_start_ms = None;
                }
            }
            _ => {
                if self.wet_start_ms.is_none() {
                    self.wet_start_ms = Some(now_ms);
                }
                if let Some(start) = self.wet_start_ms {
                    self.wet_duration_min = ((now_ms - start) / 60_000) as u32;
                }
            }
        }

        Ok(LeafWetnessReading {
            raw_adc: raw,
            wetness_pct,
            state,
            wet_duration_min: self.wet_duration_min,
        })
    }

    /// Reset the wetness duration counter (e.g., at midnight).
    pub fn reset_duration(&mut self) {
        self.wet_start_ms = None;
        self.wet_duration_min = 0;
    }
}
