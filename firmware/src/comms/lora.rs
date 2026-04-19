/// LoRa driver for SX1276 (RFM95W) via SPI.
///
/// Implements LoRa modulation for long-range low-power telemetry.
/// Transmits compact 27-byte weather packets.

use crate::config;
use crate::core::data_pipeline::WeatherReading;
use crate::error::{Error, Result};
use crate::utils::crc::crc16;
use embedded_hal::digital::OutputPin;
use embedded_hal::spi::SpiDevice;

// SX1276 Register addresses
const REG_FIFO: u8 = 0x00;
const REG_OP_MODE: u8 = 0x01;
const REG_FRF_MSB: u8 = 0x06;
const REG_FRF_MID: u8 = 0x07;
const REG_FRF_LSB: u8 = 0x08;
const REG_PA_CONFIG: u8 = 0x09;
const REG_OCP: u8 = 0x0B;
const REG_LNA: u8 = 0x0C;
const REG_FIFO_ADDR_PTR: u8 = 0x0D;
const REG_FIFO_TX_BASE: u8 = 0x0E;
const REG_FIFO_RX_BASE: u8 = 0x0F;
const REG_FIFO_RX_CURRENT: u8 = 0x10;
const REG_IRQ_FLAGS: u8 = 0x12;
const REG_RX_NB_BYTES: u8 = 0x13;
const REG_PKT_RSSI: u8 = 0x1A;
const REG_PKT_SNR: u8 = 0x19;
const REG_MODEM_CONFIG_1: u8 = 0x1D;
const REG_MODEM_CONFIG_2: u8 = 0x1E;
const REG_PREAMBLE_MSB: u8 = 0x20;
const REG_PREAMBLE_LSB: u8 = 0x21;
const REG_PAYLOAD_LENGTH: u8 = 0x22;
const REG_MODEM_CONFIG_3: u8 = 0x26;
const REG_DETECTION_OPTIMIZE: u8 = 0x31;
const REG_DETECTION_THRESHOLD: u8 = 0x37;
const REG_SYNC_WORD: u8 = 0x39;
const REG_DIO_MAPPING_1: u8 = 0x40;
const REG_VERSION: u8 = 0x42;
const REG_PA_DAC: u8 = 0x4D;

// Operating modes
const MODE_SLEEP: u8 = 0x00;
const MODE_STANDBY: u8 = 0x01;
const MODE_TX: u8 = 0x03;
const MODE_RX_CONTINUOUS: u8 = 0x05;
const MODE_LORA: u8 = 0x80;

// IRQ flags
const IRQ_TX_DONE: u8 = 0x08;
const IRQ_RX_DONE: u8 = 0x40;

// Expected chip version
const SX1276_VERSION: u8 = 0x12;

/// Telemetry packet type.
const PKT_TYPE_TELEMETRY: u8 = 0x01;
/// Telemetry packet size.
const PKT_SIZE: usize = 27;

/// LoRa radio state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoraState {
    Uninitialized,
    Standby,
    Transmitting,
    Receiving,
    Sleep,
}

/// LoRa transceiver driver.
pub struct LoraRadio<SPI, RST> {
    spi: SPI,
    rst: RST,
    state: LoraState,
    device_id: u16,
    frequency_hz: u32,
    spreading_factor: u8,
    tx_power_dbm: i8,
}

impl<SPI: SpiDevice, RST: OutputPin> LoraRadio<SPI, RST> {
    pub fn new(spi: SPI, rst: RST, device_id: u16) -> Self {
        Self {
            spi,
            rst,
            state: LoraState::Uninitialized,
            device_id,
            frequency_hz: config::LORA_FREQUENCY_HZ,
            spreading_factor: config::LORA_SPREADING_FACTOR,
            tx_power_dbm: config::LORA_TX_POWER_DBM,
        }
    }

    /// Initialize the SX1276 radio.
    pub fn init(&mut self) -> Result<()> {
        // Hardware reset
        self.rst.set_low().map_err(|_| Error::LoraInitFailed)?;
        spin_delay(10_000);
        self.rst.set_high().map_err(|_| Error::LoraInitFailed)?;
        spin_delay(50_000);

        // Verify chip version
        let version = self.read_reg(REG_VERSION)?;
        if version != SX1276_VERSION {
            log::error!("LoRa: unexpected chip version 0x{:02X}", version);
            return Err(Error::LoraInitFailed);
        }

        // Set LoRa mode + sleep
        self.write_reg(REG_OP_MODE, MODE_LORA | MODE_SLEEP)?;
        spin_delay(10_000);

        // Set frequency
        self.set_frequency(self.frequency_hz)?;

        // Set TX power
        self.set_tx_power(self.tx_power_dbm)?;

        // Set spreading factor
        self.set_spreading_factor(self.spreading_factor)?;

        // Set bandwidth (125 kHz) + coding rate (4/5) + explicit header
        self.write_reg(REG_MODEM_CONFIG_1, 0x72)?;

        // Set LNA boost
        self.write_reg(REG_LNA, 0x23)?;

        // Auto AGC
        self.write_reg(REG_MODEM_CONFIG_3, 0x04)?;

        // Preamble length = 8
        self.write_reg(REG_PREAMBLE_MSB, 0x00)?;
        self.write_reg(REG_PREAMBLE_LSB, 0x08)?;

        // LoRaWAN sync word
        self.write_reg(REG_SYNC_WORD, 0x34)?;

        // DIO0 = TX Done / RX Done
        self.write_reg(REG_DIO_MAPPING_1, 0x00)?;

        // Go to standby
        self.write_reg(REG_OP_MODE, MODE_LORA | MODE_STANDBY)?;
        self.state = LoraState::Standby;

        log::info!(
            "LoRa: SX1276 initialized, freq={}Hz, SF={}, TX={}dBm",
            self.frequency_hz,
            self.spreading_factor,
            self.tx_power_dbm
        );

        Ok(())
    }

    /// Transmit a weather reading as a compact 27-byte LoRa packet.
    pub fn send_reading(&mut self, reading: &WeatherReading, timestamp_epoch: u32) -> Result<()> {
        let packet = self.encode_packet(reading, timestamp_epoch);
        self.transmit(&packet)
    }

    /// Encode a weather reading into a 27-byte packet.
    fn encode_packet(&self, reading: &WeatherReading, timestamp: u32) -> [u8; PKT_SIZE] {
        let mut pkt = [0u8; PKT_SIZE];

        pkt[0] = PKT_TYPE_TELEMETRY;
        pkt[1..3].copy_from_slice(&self.device_id.to_be_bytes());
        pkt[3..7].copy_from_slice(&timestamp.to_be_bytes());

        // Temperature: int16, x100 °C
        let temp = reading.temperature_c.map(|t| (t * 100.0) as i16).unwrap_or(i16::MIN);
        pkt[7..9].copy_from_slice(&temp.to_be_bytes());

        // Humidity: uint16, x100 %
        let hum = reading.humidity_pct.map(|h| (h * 100.0) as u16).unwrap_or(0);
        pkt[9..11].copy_from_slice(&hum.to_be_bytes());

        // Pressure: uint32, x100 Pa (hPa * 100)
        let press = reading.pressure_hpa.map(|p| (p * 100.0) as u32).unwrap_or(0);
        pkt[11..15].copy_from_slice(&press.to_be_bytes());

        // Wind speed: uint16, x10 km/h
        let wind = reading.wind_speed_kmh.map(|w| (w * 10.0) as u16).unwrap_or(0);
        pkt[15..17].copy_from_slice(&wind.to_be_bytes());

        // Wind direction: uint16, degrees
        let wdir = reading.wind_dir_deg.unwrap_or(0);
        pkt[17..19].copy_from_slice(&wdir.to_be_bytes());

        // Rain total: uint16, x10 mm
        let rain = (reading.rain_mm * 10.0) as u16;
        pkt[19..21].copy_from_slice(&rain.to_be_bytes());

        // UV index: uint8, x10
        let uv = reading.uv_index.map(|u| (u * 10.0) as u8).unwrap_or(0);
        pkt[21] = uv;

        // Light: uint16, lux (capped at 65535)
        let light = reading.light_lux.map(|l| l.min(65535.0) as u16).unwrap_or(0);
        pkt[22..24].copy_from_slice(&light.to_be_bytes());

        // Battery %
        pkt[24] = 0; // filled by caller if available

        // CRC16 over bytes 0..25
        let crc = crc16(&pkt[0..25]);
        pkt[25..27].copy_from_slice(&crc.to_be_bytes());

        pkt
    }

    /// Transmit raw bytes over LoRa.
    pub fn transmit(&mut self, data: &[u8]) -> Result<()> {
        if self.state == LoraState::Uninitialized {
            return Err(Error::LoraInitFailed);
        }

        // Go to standby
        self.write_reg(REG_OP_MODE, MODE_LORA | MODE_STANDBY)?;

        // Set FIFO TX base address
        self.write_reg(REG_FIFO_TX_BASE, 0x00)?;
        self.write_reg(REG_FIFO_ADDR_PTR, 0x00)?;

        // Write data to FIFO
        for &byte in data {
            self.write_reg(REG_FIFO, byte)?;
        }

        // Set payload length
        self.write_reg(REG_PAYLOAD_LENGTH, data.len() as u8)?;

        // Clear IRQ flags
        self.write_reg(REG_IRQ_FLAGS, 0xFF)?;

        // Start TX
        self.write_reg(REG_OP_MODE, MODE_LORA | MODE_TX)?;
        self.state = LoraState::Transmitting;

        log::debug!("LoRa: transmitting {} bytes", data.len());

        // In real firmware, wait for DIO0 interrupt (TX done).
        // For now, poll the IRQ flag.
        for _ in 0..1000 {
            let flags = self.read_reg(REG_IRQ_FLAGS)?;
            if flags & IRQ_TX_DONE != 0 {
                self.write_reg(REG_IRQ_FLAGS, IRQ_TX_DONE)?;
                self.state = LoraState::Standby;
                self.write_reg(REG_OP_MODE, MODE_LORA | MODE_STANDBY)?;
                log::debug!("LoRa: TX complete");
                return Ok(());
            }
            spin_delay(1000);
        }

        log::error!("LoRa: TX timeout");
        self.state = LoraState::Standby;
        self.write_reg(REG_OP_MODE, MODE_LORA | MODE_STANDBY)?;
        Err(Error::LoraTxFailed)
    }

    /// Enter sleep mode to save power.
    pub fn sleep(&mut self) -> Result<()> {
        self.write_reg(REG_OP_MODE, MODE_LORA | MODE_SLEEP)?;
        self.state = LoraState::Sleep;
        Ok(())
    }

    /// Return to standby from sleep.
    pub fn wake(&mut self) -> Result<()> {
        self.write_reg(REG_OP_MODE, MODE_LORA | MODE_STANDBY)?;
        self.state = LoraState::Standby;
        Ok(())
    }

    pub fn state(&self) -> LoraState {
        self.state
    }

    // ---- Private register access ----

    fn set_frequency(&mut self, freq_hz: u32) -> Result<()> {
        let frf = ((freq_hz as u64) << 19) / 32_000_000;
        self.write_reg(REG_FRF_MSB, ((frf >> 16) & 0xFF) as u8)?;
        self.write_reg(REG_FRF_MID, ((frf >> 8) & 0xFF) as u8)?;
        self.write_reg(REG_FRF_LSB, (frf & 0xFF) as u8)?;
        Ok(())
    }

    fn set_tx_power(&mut self, power_dbm: i8) -> Result<()> {
        let power = power_dbm.clamp(2, 17) as u8;
        // PA_BOOST, max output power, output power level
        self.write_reg(REG_PA_CONFIG, 0x80 | (power - 2))?;
        self.write_reg(REG_OCP, 0x2B)?; // OCP 100mA
        Ok(())
    }

    fn set_spreading_factor(&mut self, sf: u8) -> Result<()> {
        let sf = sf.clamp(6, 12);
        let current = self.read_reg(REG_MODEM_CONFIG_2)?;
        self.write_reg(REG_MODEM_CONFIG_2, (current & 0x0F) | ((sf & 0x0F) << 4))?;

        // Detection optimization for SF6
        if sf == 6 {
            self.write_reg(REG_DETECTION_OPTIMIZE, 0x05)?;
            self.write_reg(REG_DETECTION_THRESHOLD, 0x0C)?;
        } else {
            self.write_reg(REG_DETECTION_OPTIMIZE, 0x03)?;
            self.write_reg(REG_DETECTION_THRESHOLD, 0x0A)?;
        }

        Ok(())
    }

    fn read_reg(&mut self, addr: u8) -> Result<u8> {
        let mut tx = [addr & 0x7F, 0x00];
        let mut rx = [0u8; 2];
        self.spi
            .transfer(&mut rx, &tx)
            .map_err(|_| Error::SpiBusError)?;
        Ok(rx[1])
    }

    fn write_reg(&mut self, addr: u8, value: u8) -> Result<()> {
        let tx = [addr | 0x80, value];
        self.spi.write(&tx).map_err(|_| Error::SpiBusError)?;
        Ok(())
    }
}

fn spin_delay(cycles: u32) {
    for _ in 0..cycles {
        core::hint::spin_loop();
    }
}
