/// SI1145 UV Index sensor driver.
///
/// Communicates over I2C at address 0x60.
/// Reads UV index, visible light, and IR light.

use crate::config;
use crate::error::{Error, Result, SensorKind};
use embedded_hal::i2c::I2c;

// Register addresses
const REG_PART_ID: u8 = 0x00;
const REG_SEQ_ID: u8 = 0x02;
const REG_HW_KEY: u8 = 0x07;
const REG_MEAS_RATE0: u8 = 0x08;
const REG_MEAS_RATE1: u8 = 0x09;
const REG_UCOEF0: u8 = 0x13;
const REG_UCOEF1: u8 = 0x14;
const REG_UCOEF2: u8 = 0x15;
const REG_UCOEF3: u8 = 0x16;
const REG_PARAM_WR: u8 = 0x17;
const REG_COMMAND: u8 = 0x18;
const REG_ALS_VIS_DATA0: u8 = 0x22;
const REG_ALS_VIS_DATA1: u8 = 0x23;
const REG_ALS_IR_DATA0: u8 = 0x24;
const REG_ALS_IR_DATA1: u8 = 0x25;
const REG_AUX_DATA0: u8 = 0x2C;
const REG_AUX_DATA1: u8 = 0x2D;

// Commands
const CMD_RESET: u8 = 0x01;
const CMD_ALS_AUTO: u8 = 0x0E;
const CMD_PARAM_SET: u8 = 0xA0;

// Parameters
const PARAM_CH_LIST: u8 = 0x01;
const PARAM_ALS_VIS_ADC_COUNTER: u8 = 0x10;
const PARAM_ALS_VIS_ADC_GAIN: u8 = 0x11;
const PARAM_ALS_VIS_ADC_MISC: u8 = 0x12;

const EXPECTED_PART_ID: u8 = 0x45;
const HW_KEY_VALUE: u8 = 0x17;

/// SI1145 UV index reading.
#[derive(Debug, Clone, Copy)]
pub struct UvReading {
    /// UV index (0.0 - 15.0).
    pub uv_index: f32,
    /// Visible light (raw ADC count).
    pub visible: u16,
    /// Infrared light (raw ADC count).
    pub infrared: u16,
}

/// SI1145 sensor driver.
pub struct Si1145<I2C> {
    i2c: I2C,
    addr: u8,
}

impl<I2C: I2c> Si1145<I2C> {
    pub fn new(i2c: I2C) -> Self {
        Self {
            i2c,
            addr: config::SI1145_ADDR,
        }
    }

    /// Initialize the SI1145 sensor.
    pub fn init(&mut self) -> Result<()> {
        // Verify part ID
        let part_id = self.read_register(REG_PART_ID)?;
        if part_id != EXPECTED_PART_ID {
            return Err(Error::SensorNotFound(SensorKind::UvIndex));
        }

        // Set hardware key
        self.write_register(REG_HW_KEY, HW_KEY_VALUE)?;

        // Set UV coefficients (from application note)
        self.write_register(REG_UCOEF0, 0x7B)?;
        self.write_register(REG_UCOEF1, 0x6B)?;
        self.write_register(REG_UCOEF2, 0x01)?;
        self.write_register(REG_UCOEF3, 0x00)?;

        // Enable UV, visible, and IR channels
        self.set_param(PARAM_CH_LIST, 0x77)?;

        // Set measurement rate (~10 Hz)
        self.write_register(REG_MEAS_RATE0, 0xFF)?;
        self.write_register(REG_MEAS_RATE1, 0x00)?;

        // Start auto measurement mode
        self.write_register(REG_COMMAND, CMD_ALS_AUTO)?;

        log::info!("SI1145 initialized at 0x{:02X}", self.addr);
        Ok(())
    }

    /// Read UV index, visible, and IR light.
    pub fn read(&mut self) -> Result<UvReading> {
        // Read UV index (stored in AUX data as raw value / 100)
        let uv_raw = self.read_u16(REG_AUX_DATA0)?;
        let uv_index = uv_raw as f32 / 100.0;

        // Read visible light
        let visible = self.read_u16(REG_ALS_VIS_DATA0)?;

        // Read IR light
        let infrared = self.read_u16(REG_ALS_IR_DATA0)?;

        Ok(UvReading {
            uv_index,
            visible,
            infrared,
        })
    }

    pub fn release(self) -> I2C {
        self.i2c
    }

    fn set_param(&mut self, param: u8, value: u8) -> Result<()> {
        self.write_register(REG_PARAM_WR, value)?;
        self.write_register(REG_COMMAND, CMD_PARAM_SET | param)?;
        Ok(())
    }

    fn read_u16(&mut self, reg_low: u8) -> Result<u16> {
        let low = self.read_register(reg_low)?;
        let high = self.read_register(reg_low + 1)?;
        Ok((high as u16) << 8 | low as u16)
    }

    fn read_register(&mut self, reg: u8) -> Result<u8> {
        let mut buf = [0u8; 1];
        self.i2c
            .write_read(self.addr, &[reg], &mut buf)
            .map_err(|_| Error::I2cBusError)?;
        Ok(buf[0])
    }

    fn write_register(&mut self, reg: u8, value: u8) -> Result<()> {
        self.i2c
            .write(self.addr, &[reg, value])
            .map_err(|_| Error::I2cBusError)?;
        Ok(())
    }
}
