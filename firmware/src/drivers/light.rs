/// BH1750 ambient light sensor driver.
///
/// Communicates over I2C at address 0x23.
/// Returns light level in lux (1 - 65535).

use crate::config;
use crate::error::{Error, Result, SensorKind};
use embedded_hal::i2c::I2c;

// Commands (instruction set)
const CMD_POWER_ON: u8 = 0x01;
const CMD_POWER_DOWN: u8 = 0x00;
const CMD_RESET: u8 = 0x07;
const CMD_CONT_H_RES: u8 = 0x10; // Continuously high-resolution mode (1 lux)
const CMD_CONT_H_RES2: u8 = 0x11; // High-resolution mode 2 (0.5 lux)
const CMD_ONE_TIME_H_RES: u8 = 0x20;

/// Measurement mode for the BH1750.
#[derive(Debug, Clone, Copy)]
pub enum MeasurementMode {
    /// Continuously measure at 1 lux resolution (~120ms per measurement).
    ContinuousHighRes,
    /// Continuously measure at 0.5 lux resolution (~120ms per measurement).
    ContinuousHighRes2,
    /// Single measurement at 1 lux resolution, then power down.
    OneTimeHighRes,
}

/// BH1750 reading.
#[derive(Debug, Clone, Copy)]
pub struct LightReading {
    /// Ambient light level in lux.
    pub lux: f32,
    /// Raw 16-bit sensor value.
    pub raw: u16,
}

/// BH1750 ambient light sensor driver.
pub struct Bh1750<I2C> {
    i2c: I2C,
    addr: u8,
    mode: MeasurementMode,
}

impl<I2C: I2c> Bh1750<I2C> {
    pub fn new(i2c: I2C) -> Self {
        Self {
            i2c,
            addr: config::BH1750_ADDR,
            mode: MeasurementMode::ContinuousHighRes,
        }
    }

    /// Initialize the BH1750 sensor.
    pub fn init(&mut self) -> Result<()> {
        // Power on
        self.send_command(CMD_POWER_ON)?;

        // Reset
        self.send_command(CMD_RESET)?;

        // Set measurement mode
        let mode_cmd = match self.mode {
            MeasurementMode::ContinuousHighRes => CMD_CONT_H_RES,
            MeasurementMode::ContinuousHighRes2 => CMD_CONT_H_RES2,
            MeasurementMode::OneTimeHighRes => CMD_ONE_TIME_H_RES,
        };
        self.send_command(mode_cmd)?;

        log::info!("BH1750 initialized at 0x{:02X}", self.addr);
        Ok(())
    }

    /// Read the ambient light level.
    pub fn read(&mut self) -> Result<LightReading> {
        let mut buf = [0u8; 2];
        self.i2c
            .read(self.addr, &mut buf)
            .map_err(|_| Error::I2cBusError)?;

        let raw = ((buf[0] as u16) << 8) | buf[1] as u16;

        // Convert to lux: raw / 1.2
        let lux = raw as f32 / 1.2;

        Ok(LightReading { lux, raw })
    }

    /// Set measurement mode.
    pub fn set_mode(&mut self, mode: MeasurementMode) -> Result<()> {
        self.mode = mode;
        let cmd = match mode {
            MeasurementMode::ContinuousHighRes => CMD_CONT_H_RES,
            MeasurementMode::ContinuousHighRes2 => CMD_CONT_H_RES2,
            MeasurementMode::OneTimeHighRes => CMD_ONE_TIME_H_RES,
        };
        self.send_command(cmd)
    }

    /// Power down the sensor to save energy.
    pub fn power_down(&mut self) -> Result<()> {
        self.send_command(CMD_POWER_DOWN)
    }

    pub fn release(self) -> I2C {
        self.i2c
    }

    fn send_command(&mut self, cmd: u8) -> Result<()> {
        self.i2c
            .write(self.addr, &[cmd])
            .map_err(|_| Error::I2cBusError)?;
        Ok(())
    }
}
