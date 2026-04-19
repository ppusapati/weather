/// DS18B20 soil temperature sensor driver (1-Wire protocol).
///
/// Measures soil temperature at probe depth for agriculture applications.
/// Also used for solar panel back-of-module temperature measurement.
///
/// # Hardware
///
/// - Dallas DS18B20 waterproof probe
/// - 1-Wire bus on a single GPIO with 4.7kΩ external pull-up
/// - Resolution: configurable 9–12 bit (default 12-bit = 0.0625°C)
/// - Range: -55°C to +125°C

/// DS18B20 1-Wire commands.
mod commands {
    pub const SKIP_ROM: u8 = 0xCC;
    pub const CONVERT_T: u8 = 0x44;
    pub const READ_SCRATCHPAD: u8 = 0xBE;
    pub const WRITE_SCRATCHPAD: u8 = 0x4E;
}

/// Temperature resolution setting.
#[derive(Debug, Clone, Copy)]
pub enum Resolution {
    /// 9-bit: 0.5°C, 93.75 ms conversion.
    Bits9 = 0x1F,
    /// 10-bit: 0.25°C, 187.5 ms conversion.
    Bits10 = 0x3F,
    /// 11-bit: 0.125°C, 375 ms conversion.
    Bits11 = 0x5F,
    /// 12-bit: 0.0625°C, 750 ms conversion.
    Bits12 = 0x7F,
}

/// DS18B20 soil/panel temperature sensor.
pub struct Ds18b20 {
    /// GPIO pin for 1-Wire data line.
    pin: u8,
    /// Current resolution setting.
    resolution: Resolution,
    /// Label for logging (e.g., "soil" or "panel").
    label: &'static str,
    initialized: bool,
}

impl Ds18b20 {
    pub fn new(pin: u8, label: &'static str) -> Self {
        Self {
            pin,
            resolution: Resolution::Bits12,
            label,
            initialized: false,
        }
    }

    pub fn init(&mut self) -> crate::error::Result<()> {
        log::info!("DS18B20 ({}): init on GPIO{}", self.label, self.pin);

        // In real firmware:
        // 1. Send reset pulse on 1-Wire bus
        // 2. Check for presence pulse from DS18B20
        // 3. Set resolution via WRITE_SCRATCHPAD
        self.initialized = true;
        Ok(())
    }

    /// Start a temperature conversion (non-blocking).
    pub fn start_conversion(&self) -> crate::error::Result<()> {
        if !self.initialized {
            return Err(crate::error::Error::SensorNotReady);
        }
        // In real firmware:
        // 1. Reset + presence
        // 2. Send SKIP_ROM (single device on bus)
        // 3. Send CONVERT_T
        // 4. Wait for conversion (or poll busy bit)
        let _ = commands::SKIP_ROM;
        let _ = commands::CONVERT_T;
        Ok(())
    }

    /// Read the temperature result after conversion completes.
    pub fn read_temperature(&self) -> crate::error::Result<f32> {
        if !self.initialized {
            return Err(crate::error::Error::SensorNotReady);
        }
        // In real firmware:
        // 1. Reset + presence
        // 2. Send SKIP_ROM
        // 3. Send READ_SCRATCHPAD
        // 4. Read 9 bytes, verify CRC
        // 5. Combine byte 0 (LSB) and byte 1 (MSB)
        let _ = commands::READ_SCRATCHPAD;

        // Temperature conversion from raw 16-bit value:
        // For 12-bit: temp_c = raw / 16.0
        let raw: i16 = 400; // placeholder: 25.0°C
        let temp_c = raw as f32 / 16.0;

        if temp_c < -55.0 || temp_c > 125.0 {
            return Err(crate::error::Error::SensorReadFailed(
                crate::error::SensorKind::Temperature,
            ));
        }

        Ok(temp_c)
    }

    pub fn set_resolution(&mut self, res: Resolution) {
        self.resolution = res;
        log::info!("DS18B20 ({}): resolution set to {:?}", self.label, self.resolution);
        // In real firmware: send WRITE_SCRATCHPAD with new config byte
    }
}
