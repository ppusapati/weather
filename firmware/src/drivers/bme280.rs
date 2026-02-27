/// BME280 driver — Temperature, Humidity, and Barometric Pressure.
///
/// Communicates over I2C at address 0x76.
/// Implements the Bosch BME280 compensation formulas from the datasheet.

use crate::config;
use crate::error::{Error, Result, SensorKind};
use embedded_hal::i2c::I2c;

// Register addresses
const REG_CHIP_ID: u8 = 0xD0;
const REG_RESET: u8 = 0xE0;
const REG_CTRL_HUM: u8 = 0xF2;
const REG_STATUS: u8 = 0xF3;
const REG_CTRL_MEAS: u8 = 0xF4;
const REG_CONFIG: u8 = 0xF5;
const REG_DATA_START: u8 = 0xF7;

// Calibration register ranges
const REG_CALIB_T_P_START: u8 = 0x88; // T1..P9 (26 bytes)
const REG_CALIB_H_START: u8 = 0xE1; // H2..H6 (7 bytes)
const REG_CALIB_H1: u8 = 0xA1;

const CHIP_ID_BME280: u8 = 0x60;
const SOFT_RESET_CMD: u8 = 0xB6;

/// Calibration data read from BME280 non-volatile memory.
#[derive(Debug, Clone)]
pub struct CalibrationData {
    // Temperature
    pub dig_t1: u16,
    pub dig_t2: i16,
    pub dig_t3: i16,
    // Pressure
    pub dig_p1: u16,
    pub dig_p2: i16,
    pub dig_p3: i16,
    pub dig_p4: i16,
    pub dig_p5: i16,
    pub dig_p6: i16,
    pub dig_p7: i16,
    pub dig_p8: i16,
    pub dig_p9: i16,
    // Humidity
    pub dig_h1: u8,
    pub dig_h2: i16,
    pub dig_h3: u8,
    pub dig_h4: i16,
    pub dig_h5: i16,
    pub dig_h6: i8,
}

/// Raw ADC readings from the BME280.
#[derive(Debug, Clone, Copy)]
pub struct RawReading {
    pub pressure: i32,
    pub temperature: i32,
    pub humidity: i32,
}

/// Compensated BME280 reading.
#[derive(Debug, Clone, Copy)]
pub struct Bme280Reading {
    /// Temperature in degrees Celsius.
    pub temperature_c: f32,
    /// Relative humidity in percent.
    pub humidity_pct: f32,
    /// Barometric pressure in hPa.
    pub pressure_hpa: f32,
}

/// BME280 sensor driver.
pub struct Bme280<I2C> {
    i2c: I2C,
    addr: u8,
    calibration: Option<CalibrationData>,
}

impl<I2C: I2c> Bme280<I2C> {
    /// Create a new BME280 driver instance.
    pub fn new(i2c: I2C) -> Self {
        Self {
            i2c,
            addr: config::BME280_ADDR,
            calibration: None,
        }
    }

    /// Initialize the sensor: verify chip ID, read calibration, configure.
    pub fn init(&mut self) -> Result<()> {
        // Verify chip ID
        let chip_id = self.read_register(REG_CHIP_ID)?;
        if chip_id != CHIP_ID_BME280 {
            return Err(Error::SensorNotFound(SensorKind::Bme280));
        }

        // Soft reset
        self.write_register(REG_RESET, SOFT_RESET_CMD)?;

        // Wait for reset (2ms typical)
        // In real firmware: embassy_time::Timer::after_millis(10).await
        cortex_m_nop_delay(10_000);

        // Read calibration data
        self.calibration = Some(self.read_calibration()?);

        // Configure: humidity oversampling x1
        self.write_register(REG_CTRL_HUM, 0x01)?;

        // Configure: standby 1000ms, filter coeff 4
        self.write_register(REG_CONFIG, 0x90)?;

        // Configure: temp oversampling x2, pressure oversampling x16, normal mode
        self.write_register(REG_CTRL_MEAS, 0x57)?;

        log::info!("BME280 initialized at 0x{:02X}", self.addr);
        Ok(())
    }

    /// Read compensated temperature, humidity, and pressure.
    pub fn read(&mut self) -> Result<Bme280Reading> {
        let cal = self
            .calibration
            .as_ref()
            .ok_or(Error::SensorReadFailed(SensorKind::Bme280))?
            .clone();

        let raw = self.read_raw()?;
        let (temperature, t_fine) = self.compensate_temperature(raw.temperature, &cal);
        let pressure = self.compensate_pressure(raw.pressure, t_fine, &cal);
        let humidity = self.compensate_humidity(raw.humidity, t_fine, &cal);

        Ok(Bme280Reading {
            temperature_c: temperature,
            humidity_pct: humidity,
            pressure_hpa: pressure,
        })
    }

    /// Release the I2C bus.
    pub fn release(self) -> I2C {
        self.i2c
    }

    // --- Private methods ---

    fn read_raw(&mut self) -> Result<RawReading> {
        let mut data = [0u8; 8];
        self.read_registers(REG_DATA_START, &mut data)?;

        let pressure = ((data[0] as i32) << 12) | ((data[1] as i32) << 4) | ((data[2] as i32) >> 4);
        let temperature =
            ((data[3] as i32) << 12) | ((data[4] as i32) << 4) | ((data[5] as i32) >> 4);
        let humidity = ((data[6] as i32) << 8) | (data[7] as i32);

        Ok(RawReading {
            pressure,
            temperature,
            humidity,
        })
    }

    fn read_calibration(&mut self) -> Result<CalibrationData> {
        // Read temperature and pressure calibration (26 bytes starting at 0x88)
        let mut tp_cal = [0u8; 26];
        self.read_registers(REG_CALIB_T_P_START, &mut tp_cal)?;

        // Read H1
        let mut h1_buf = [0u8; 1];
        self.read_registers(REG_CALIB_H1, &mut h1_buf)?;

        // Read H2..H6 (7 bytes starting at 0xE1)
        let mut h_cal = [0u8; 7];
        self.read_registers(REG_CALIB_H_START, &mut h_cal)?;

        Ok(CalibrationData {
            dig_t1: u16::from_le_bytes([tp_cal[0], tp_cal[1]]),
            dig_t2: i16::from_le_bytes([tp_cal[2], tp_cal[3]]),
            dig_t3: i16::from_le_bytes([tp_cal[4], tp_cal[5]]),
            dig_p1: u16::from_le_bytes([tp_cal[6], tp_cal[7]]),
            dig_p2: i16::from_le_bytes([tp_cal[8], tp_cal[9]]),
            dig_p3: i16::from_le_bytes([tp_cal[10], tp_cal[11]]),
            dig_p4: i16::from_le_bytes([tp_cal[12], tp_cal[13]]),
            dig_p5: i16::from_le_bytes([tp_cal[14], tp_cal[15]]),
            dig_p6: i16::from_le_bytes([tp_cal[16], tp_cal[17]]),
            dig_p7: i16::from_le_bytes([tp_cal[18], tp_cal[19]]),
            dig_p8: i16::from_le_bytes([tp_cal[20], tp_cal[21]]),
            dig_p9: i16::from_le_bytes([tp_cal[22], tp_cal[23]]),
            dig_h1: h1_buf[0],
            dig_h2: i16::from_le_bytes([h_cal[0], h_cal[1]]),
            dig_h3: h_cal[2],
            dig_h4: ((h_cal[3] as i16) << 4) | ((h_cal[4] as i16) & 0x0F),
            dig_h5: ((h_cal[5] as i16) << 4) | ((h_cal[4] as i16) >> 4),
            dig_h6: h_cal[6] as i8,
        })
    }

    fn compensate_temperature(&self, adc_t: i32, cal: &CalibrationData) -> (f32, i32) {
        let var1 = ((adc_t >> 3) - ((cal.dig_t1 as i32) << 1)) * (cal.dig_t2 as i32) >> 11;
        let var2 = (((((adc_t >> 4) - (cal.dig_t1 as i32))
            * ((adc_t >> 4) - (cal.dig_t1 as i32)))
            >> 12)
            * (cal.dig_t3 as i32))
            >> 14;
        let t_fine = var1 + var2;
        let temperature = ((t_fine * 5 + 128) >> 8) as f32 / 100.0;
        (temperature, t_fine)
    }

    fn compensate_pressure(&self, adc_p: i32, t_fine: i32, cal: &CalibrationData) -> f32 {
        let mut var1 = (t_fine as i64) - 128000;
        let mut var2 = var1 * var1 * (cal.dig_p6 as i64);
        var2 += (var1 * (cal.dig_p5 as i64)) << 17;
        var2 += (cal.dig_p4 as i64) << 35;
        var1 = ((var1 * var1 * (cal.dig_p3 as i64)) >> 8) + ((var1 * (cal.dig_p2 as i64)) << 12);
        var1 = (((1i64 << 47) + var1) * (cal.dig_p1 as i64)) >> 33;

        if var1 == 0 {
            return 0.0;
        }

        let mut pressure = 1048576i64 - adc_p as i64;
        pressure = (((pressure << 31) - var2) * 3125) / var1;
        var1 = ((cal.dig_p9 as i64) * (pressure >> 13) * (pressure >> 13)) >> 25;
        var2 = ((cal.dig_p8 as i64) * pressure) >> 19;
        pressure = ((pressure + var1 + var2) >> 8) + ((cal.dig_p7 as i64) << 4);

        (pressure as f32) / 25600.0
    }

    fn compensate_humidity(&self, adc_h: i32, t_fine: i32, cal: &CalibrationData) -> f32 {
        let var1 = t_fine - 76800i32;
        let var2 = adc_h - ((cal.dig_h4 as i32) << 4) - (((cal.dig_h5 as i32) * var1) >> 14);
        let var3 = (cal.dig_h2 as i32) * 65536;
        let var4 = ((((var1 * (cal.dig_h6 as i32)) >> 10)
            * (((var1 * (cal.dig_h3 as i32)) >> 11) + 32768))
            >> 10)
            + 2097152;
        let var5 = ((var3 * var4) >> 14) + 16384;
        let humidity = ((var2 * var5) >> 14) as f32 / 1024.0;
        humidity.clamp(0.0, 100.0)
    }

    fn read_register(&mut self, reg: u8) -> Result<u8> {
        let mut buf = [0u8; 1];
        self.i2c
            .write_read(self.addr, &[reg], &mut buf)
            .map_err(|_| Error::I2cBusError)?;
        Ok(buf[0])
    }

    fn read_registers(&mut self, reg: u8, buf: &mut [u8]) -> Result<()> {
        self.i2c
            .write_read(self.addr, &[reg], buf)
            .map_err(|_| Error::I2cBusError)?;
        Ok(())
    }

    fn write_register(&mut self, reg: u8, value: u8) -> Result<()> {
        self.i2c
            .write(self.addr, &[reg, value])
            .map_err(|_| Error::I2cBusError)?;
        Ok(())
    }
}

/// Busy-wait delay (placeholder for actual timer-based delay).
#[inline(always)]
fn cortex_m_nop_delay(cycles: u32) {
    for _ in 0..cycles {
        core::hint::spin_loop();
    }
}
