/// Wind sensor drivers:
/// - Anemometer (reed switch, pulse counting via GPIO interrupt)
/// - Wind vane (AS5600 magnetic rotary encoder via I2C)

use crate::config;
use crate::error::{Error, Result, SensorKind};
use core::sync::atomic::{AtomicU32, Ordering};
use embedded_hal::i2c::I2c;

// AS5600 registers
const REG_RAW_ANGLE_H: u8 = 0x0C;
const REG_STATUS: u8 = 0x0B;
const REG_MAGNITUDE_H: u8 = 0x1B;

/// Global pulse counter incremented by GPIO ISR.
/// Safe to access from ISR context using atomic operations.
static WIND_PULSE_COUNT: AtomicU32 = AtomicU32::new(0);

/// Timestamp of last pulse read (milliseconds since boot).
static LAST_WIND_READ_MS: AtomicU32 = AtomicU32::new(0);

/// Wind speed reading.
#[derive(Debug, Clone, Copy)]
pub struct WindSpeedReading {
    /// Wind speed in km/h.
    pub speed_kmh: f32,
    /// Raw pulse count since last read.
    pub pulses: u32,
    /// Sample period in milliseconds.
    pub period_ms: u32,
}

/// Wind direction reading.
#[derive(Debug, Clone, Copy)]
pub struct WindDirectionReading {
    /// Wind direction in degrees (0-359).
    pub direction_deg: u16,
    /// Raw 12-bit angle from AS5600.
    pub raw_angle: u16,
    /// Magnet status (detected, too weak, too strong).
    pub magnet_status: MagnetStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MagnetStatus {
    Detected,
    TooWeak,
    TooStrong,
    NotDetected,
}

// ---- Anemometer (pulse counting) ----

/// Call this from the GPIO ISR for the anemometer pin.
/// Increments the global pulse counter atomically.
pub fn wind_isr_handler() {
    WIND_PULSE_COUNT.fetch_add(1, Ordering::Relaxed);
}

/// Anemometer driver using pulse counting.
pub struct Anemometer {
    last_count: u32,
    last_time_ms: u32,
}

impl Anemometer {
    pub fn new() -> Self {
        Self {
            last_count: 0,
            last_time_ms: 0,
        }
    }

    /// Initialize the anemometer. Resets the pulse counter.
    pub fn init(&mut self) {
        WIND_PULSE_COUNT.store(0, Ordering::Relaxed);
        self.last_count = 0;
        log::info!("Anemometer initialized on GPIO {}", config::WIND_SPEED_PIN);
    }

    /// Read wind speed. Call this periodically (e.g. every 5 seconds).
    /// `current_time_ms` is the current system time in milliseconds.
    pub fn read(&mut self, current_time_ms: u32) -> WindSpeedReading {
        let current_count = WIND_PULSE_COUNT.load(Ordering::Relaxed);
        let pulses = current_count.wrapping_sub(self.last_count);
        let period_ms = current_time_ms.wrapping_sub(self.last_time_ms);

        self.last_count = current_count;
        self.last_time_ms = current_time_ms;

        let speed_kmh = if period_ms > 0 {
            (pulses as f32 * config::WIND_SPEED_FACTOR * 1000.0) / period_ms as f32
        } else {
            0.0
        };

        WindSpeedReading {
            speed_kmh,
            pulses,
            period_ms,
        }
    }

    /// Get the total pulse count since init.
    pub fn total_pulses(&self) -> u32 {
        WIND_PULSE_COUNT.load(Ordering::Relaxed)
    }
}

// ---- Wind Vane (AS5600 magnetic encoder) ----

/// AS5600 wind direction sensor driver.
pub struct WindVane<I2C> {
    i2c: I2C,
    addr: u8,
    offset_deg: f32,
}

impl<I2C: I2c> WindVane<I2C> {
    /// Create a new wind vane driver.
    pub fn new(i2c: I2C) -> Self {
        Self {
            i2c,
            addr: config::AS5600_ADDR,
            offset_deg: 0.0,
        }
    }

    /// Initialize the AS5600. Verifies magnet presence.
    pub fn init(&mut self) -> Result<()> {
        let status = self.read_status()?;
        if status == MagnetStatus::NotDetected {
            log::warn!("AS5600: no magnet detected");
            return Err(Error::SensorNotFound(SensorKind::WindDirection));
        }

        log::info!(
            "AS5600 initialized at 0x{:02X}, magnet: {:?}",
            self.addr,
            status
        );
        Ok(())
    }

    /// Set the zero-point offset for wind direction.
    pub fn set_offset(&mut self, offset_deg: f32) {
        self.offset_deg = offset_deg;
    }

    /// Read the current wind direction.
    pub fn read(&mut self) -> Result<WindDirectionReading> {
        let raw_angle = self.read_raw_angle()?;
        let magnet_status = self.read_status()?;

        // Convert 12-bit value to degrees, apply offset
        let mut direction = (raw_angle as f32 * 360.0 / 4096.0) + self.offset_deg;
        if direction < 0.0 {
            direction += 360.0;
        }
        if direction >= 360.0 {
            direction -= 360.0;
        }

        Ok(WindDirectionReading {
            direction_deg: direction as u16,
            raw_angle,
            magnet_status,
        })
    }

    /// Release the I2C bus.
    pub fn release(self) -> I2C {
        self.i2c
    }

    fn read_raw_angle(&mut self) -> Result<u16> {
        let mut buf = [0u8; 2];
        self.i2c
            .write_read(self.addr, &[REG_RAW_ANGLE_H], &mut buf)
            .map_err(|_| Error::I2cBusError)?;
        Ok(((buf[0] as u16 & 0x0F) << 8) | buf[1] as u16)
    }

    fn read_status(&mut self) -> Result<MagnetStatus> {
        let mut buf = [0u8; 1];
        self.i2c
            .write_read(self.addr, &[REG_STATUS], &mut buf)
            .map_err(|_| Error::I2cBusError)?;

        let status = buf[0];
        if status & 0x20 != 0 {
            Ok(MagnetStatus::Detected)
        } else if status & 0x10 != 0 {
            Ok(MagnetStatus::TooWeak)
        } else if status & 0x08 != 0 {
            Ok(MagnetStatus::TooStrong)
        } else {
            Ok(MagnetStatus::NotDetected)
        }
    }
}
