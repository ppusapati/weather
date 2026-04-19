/// Modbus RTU slave driver for SCADA integration.
///
/// Implements a Modbus RTU slave over RS485 (half-duplex) using
/// the MAX3485 transceiver on USART2. Supports function codes
/// 0x03 (Read Holding Registers) and 0x04 (Read Input Registers)
/// for exposing weather data, and 0x06/0x10 (Write Single/Multiple
/// Holding Registers) for remote configuration.
///
/// # Register Map
///
/// ## Input Registers (Read-Only, FC 0x04) — Live Sensor Data
///
/// | Address | Description | Unit | Scale |
/// |---------|-------------|------|-------|
/// | 30001 | Temperature | °C × 100 | i16 |
/// | 30002 | Humidity | % × 100 | u16 |
/// | 30003 | Pressure (high word) | hPa × 100 | u32 |
/// | 30004 | Pressure (low word) | | |
/// | 30005 | Wind Speed | km/h × 100 | u16 |
/// | 30006 | Wind Direction | degrees | u16 |
/// | 30007 | Rain Rate | mm/h × 100 | u16 |
/// | 30008 | Rain Accumulation | mm × 100 | u16 |
/// | 30009 | UV Index | index × 100 | u16 |
/// | 30010 | Light (high word) | lux | u32 |
/// | 30011 | Light (low word) | | |
/// | 30012 | Heat Index | °C × 100 | i16 |
/// | 30013 | Dew Point | °C × 100 | i16 |
/// | 30014 | Wind Chill | °C × 100 | i16 |
/// | 30015 | Battery Voltage | mV | u16 |
/// | 30016 | Battery Percentage | % | u16 |
/// | 30017 | PM2.5 (India) | µg/m³ × 10 | u16 |
/// | 30018 | AQI NAQI (India) | index | u16 |
/// | 30019 | Soil Moisture Shallow | % × 100 | u16 |
/// | 30020 | Soil Moisture Deep | % × 100 | u16 |
/// | 30021 | Soil Temperature | °C × 100 | i16 |
/// | 30022 | Solar Irradiance | W/m² × 10 | u16 |
/// | 30023 | Panel Temperature | °C × 100 | i16 |
/// | 30024 | Est. Solar Power | W × 10 | u16 |
/// | 30025–30030 | Reserved | | |
///
/// ## Holding Registers (R/W, FC 0x03/0x06/0x10) — Configuration
///
/// | Address | Description | Unit | Default |
/// |---------|-------------|------|---------|
/// | 40001 | Slave Address | 1–247 | 1 |
/// | 40002 | Baud Rate Code | 0–4 | 1 (9600) |
/// | 40003 | Sensor Read Interval | seconds | 10 |
/// | 40004 | Operating Mode | 0–2 | 2 (Hybrid) |
/// | 40005 | Temp Cal Offset | °C × 100 | 0 |
/// | 40006 | Humidity Cal Offset | % × 100 | 0 |
/// | 40007 | Pressure Cal Offset | hPa × 100 | 0 |
/// | 40008 | GDD Base Temp | °C × 100 | 1000 |
/// | 40009 | India Region | 0–2 | 0 (Plains) |
/// | 40010 | Device ID | 0–65535 | 1 |
/// | 40011–40020 | Reserved | | |
///
/// ## Discrete Inputs (Read-Only, FC 0x02) — Alert Flags
///
/// | Address | Description |
/// |---------|-------------|
/// | 10001 | Heat Wave Active |
/// | 10002 | Severe Heat Wave |
/// | 10003 | Cyclone Watch |
/// | 10004 | Cyclone Warning |
/// | 10005 | Cyclone Severe |
/// | 10006 | AQI Severe |
/// | 10007 | Frost Warning |
/// | 10008 | Sensor Fault |

use crate::drivers::stm32f407::modbus;
use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};

// ──────────────────────────────────────────────────
// Modbus Function Codes
// ──────────────────────────────────────────────────

/// Supported Modbus function codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FunctionCode {
    /// Read Discrete Inputs (FC 0x02).
    ReadDiscreteInputs = 0x02,
    /// Read Holding Registers (FC 0x03).
    ReadHoldingRegisters = 0x03,
    /// Read Input Registers (FC 0x04).
    ReadInputRegisters = 0x04,
    /// Write Single Holding Register (FC 0x06).
    WriteSingleRegister = 0x06,
    /// Write Multiple Holding Registers (FC 0x10).
    WriteMultipleRegisters = 0x10,
}

impl FunctionCode {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0x02 => Some(FunctionCode::ReadDiscreteInputs),
            0x03 => Some(FunctionCode::ReadHoldingRegisters),
            0x04 => Some(FunctionCode::ReadInputRegisters),
            0x06 => Some(FunctionCode::WriteSingleRegister),
            0x10 => Some(FunctionCode::WriteMultipleRegisters),
            _ => None,
        }
    }
}

/// Modbus exception codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ExceptionCode {
    IllegalFunction = 0x01,
    IllegalDataAddress = 0x02,
    IllegalDataValue = 0x03,
    SlaveDeviceFailure = 0x04,
}

// ──────────────────────────────────────────────────
// Register Banks
// ──────────────────────────────────────────────────

/// Number of input registers (live sensor data).
pub const INPUT_REGISTER_COUNT: usize = 30;
/// Number of holding registers (configuration).
pub const HOLDING_REGISTER_COUNT: usize = 20;
/// Number of discrete inputs (alert flags).
pub const DISCRETE_INPUT_COUNT: usize = 8;

/// Input register base address (Modbus convention: 30001 → index 0).
pub const INPUT_REG_BASE: u16 = 0;
/// Holding register base address (Modbus convention: 40001 → index 0).
pub const HOLDING_REG_BASE: u16 = 0;

// Input register indices
pub const IR_TEMPERATURE: usize = 0;
pub const IR_HUMIDITY: usize = 1;
pub const IR_PRESSURE_HI: usize = 2;
pub const IR_PRESSURE_LO: usize = 3;
pub const IR_WIND_SPEED: usize = 4;
pub const IR_WIND_DIR: usize = 5;
pub const IR_RAIN_RATE: usize = 6;
pub const IR_RAIN_ACCUM: usize = 7;
pub const IR_UV_INDEX: usize = 8;
pub const IR_LIGHT_HI: usize = 9;
pub const IR_LIGHT_LO: usize = 10;
pub const IR_HEAT_INDEX: usize = 11;
pub const IR_DEW_POINT: usize = 12;
pub const IR_WIND_CHILL: usize = 13;
pub const IR_BATTERY_MV: usize = 14;
pub const IR_BATTERY_PCT: usize = 15;
pub const IR_PM25: usize = 16;
pub const IR_AQI_NAQI: usize = 17;
pub const IR_SOIL_SHALLOW: usize = 18;
pub const IR_SOIL_DEEP: usize = 19;
pub const IR_SOIL_TEMP: usize = 20;
pub const IR_IRRADIANCE: usize = 21;
pub const IR_PANEL_TEMP: usize = 22;
pub const IR_SOLAR_POWER: usize = 23;

// Holding register indices
pub const HR_SLAVE_ADDR: usize = 0;
pub const HR_BAUD_CODE: usize = 1;
pub const HR_READ_INTERVAL: usize = 2;
pub const HR_OP_MODE: usize = 3;
pub const HR_TEMP_CAL: usize = 4;
pub const HR_HUM_CAL: usize = 5;
pub const HR_PRESS_CAL: usize = 6;
pub const HR_GDD_BASE: usize = 7;
pub const HR_INDIA_REGION: usize = 8;
pub const HR_DEVICE_ID: usize = 9;

// Discrete input indices
pub const DI_HEAT_WAVE: usize = 0;
pub const DI_SEVERE_HEAT: usize = 1;
pub const DI_CYCLONE_WATCH: usize = 2;
pub const DI_CYCLONE_WARNING: usize = 3;
pub const DI_CYCLONE_SEVERE: usize = 4;
pub const DI_AQI_SEVERE: usize = 5;
pub const DI_FROST_WARNING: usize = 6;
pub const DI_SENSOR_FAULT: usize = 7;

// ──────────────────────────────────────────────────
// Modbus RTU Slave
// ──────────────────────────────────────────────────

/// Modbus RTU slave state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModbusState {
    /// Waiting for a new frame.
    Idle,
    /// Receiving frame bytes.
    Receiving,
    /// Processing a complete frame.
    Processing,
    /// Transmitting response.
    Transmitting,
    /// Error state — will auto-recover to Idle.
    Error,
}

/// Statistics for SCADA monitoring.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModbusStats {
    /// Total valid requests processed.
    pub requests_ok: u32,
    /// CRC errors detected.
    pub crc_errors: u32,
    /// Exception responses sent.
    pub exceptions: u32,
    /// Framing/timeout errors.
    pub frame_errors: u32,
    /// Total bytes received.
    pub bytes_rx: u32,
    /// Total bytes transmitted.
    pub bytes_tx: u32,
}

/// Modbus RTU slave driver.
///
/// Manages the Modbus protocol state machine, register banks,
/// CRC validation, and RS485 transceiver direction control.
pub struct ModbusRtuSlave {
    /// Configured slave address (1–247).
    slave_addr: u8,
    /// Current state machine state.
    state: ModbusState,
    /// Receive buffer.
    rx_buf: [u8; modbus::MAX_FRAME_SIZE],
    /// Receive buffer write position.
    rx_pos: usize,
    /// Transmit buffer.
    tx_buf: [u8; modbus::MAX_FRAME_SIZE],
    /// Transmit buffer length.
    tx_len: usize,
    /// Input registers (read-only sensor data).
    input_regs: [u16; INPUT_REGISTER_COUNT],
    /// Holding registers (read/write configuration).
    holding_regs: [u16; HOLDING_REGISTER_COUNT],
    /// Discrete inputs (alert flags).
    discrete_inputs: [bool; DISCRETE_INPUT_COUNT],
    /// Protocol statistics.
    pub stats: ModbusStats,
    /// Timestamp of last byte received (for inter-frame gap detection).
    last_byte_ms: u64,
    /// Whether the RS485 driver is in transmit mode.
    tx_active: bool,
}

impl ModbusRtuSlave {
    pub fn new(slave_addr: u8) -> Self {
        let addr = if slave_addr == 0 || slave_addr > 247 {
            modbus::DEFAULT_SLAVE_ADDR
        } else {
            slave_addr
        };

        let mut holding = [0u16; HOLDING_REGISTER_COUNT];
        holding[HR_SLAVE_ADDR] = addr as u16;
        holding[HR_BAUD_CODE] = 1; // 9600
        holding[HR_READ_INTERVAL] = 10; // 10 seconds
        holding[HR_OP_MODE] = 2; // Hybrid
        holding[HR_GDD_BASE] = 1000; // 10.0°C × 100

        Self {
            slave_addr: addr,
            state: ModbusState::Idle,
            rx_buf: [0u8; modbus::MAX_FRAME_SIZE],
            rx_pos: 0,
            tx_buf: [0u8; modbus::MAX_FRAME_SIZE],
            tx_len: 0,
            input_regs: [0u16; INPUT_REGISTER_COUNT],
            holding_regs: holding,
            discrete_inputs: [false; DISCRETE_INPUT_COUNT],
            stats: ModbusStats::default(),
            last_byte_ms: 0,
            tx_active: false,
        }
    }

    /// Initialize the Modbus slave (configure USART2, GPIO for DE/RE).
    pub fn init(&mut self) -> Result<()> {
        // In real firmware:
        // 1. Configure USART2 at modbus::BAUD_RATE, 8N1
        // 2. Configure PA1 as output for MAX3485 DE/RE
        // 3. Set DE/RE low (receive mode)
        // 4. Enable USART2 RX interrupt
        // 5. Configure TIM7 for T3.5 inter-frame gap detection
        self.tx_active = false;
        log::info!(
            "Modbus RTU slave initialized: addr={}, baud={}",
            self.slave_addr,
            modbus::BAUD_RATE
        );
        Ok(())
    }

    /// Feed a received byte into the state machine.
    /// Call this from the USART2 RX interrupt handler.
    pub fn on_byte_received(&mut self, byte: u8, now_ms: u64) {
        // Detect inter-frame gap (T3.5 timeout = new frame)
        if self.state != ModbusState::Idle {
            let elapsed_us = (now_ms.saturating_sub(self.last_byte_ms)) * 1000;
            if elapsed_us > modbus::T35_US as u64 {
                // T3.5 gap detected — previous frame complete or aborted
                if self.rx_pos >= 4 {
                    self.state = ModbusState::Processing;
                    self.process_frame();
                } else {
                    self.stats.frame_errors += 1;
                    self.reset_rx();
                }
            }
        }

        self.last_byte_ms = now_ms;
        self.stats.bytes_rx += 1;

        if self.rx_pos >= modbus::MAX_FRAME_SIZE {
            self.stats.frame_errors += 1;
            self.reset_rx();
            return;
        }

        self.rx_buf[self.rx_pos] = byte;
        self.rx_pos += 1;
        self.state = ModbusState::Receiving;
    }

    /// Check for inter-frame gap timeout (call from main loop).
    /// If T3.5 has elapsed since last byte, process the accumulated frame.
    pub fn poll(&mut self, now_ms: u64) {
        if self.state == ModbusState::Receiving {
            let elapsed_us = (now_ms.saturating_sub(self.last_byte_ms)) * 1000;
            if elapsed_us > modbus::T35_US as u64 && self.rx_pos >= 4 {
                self.state = ModbusState::Processing;
                self.process_frame();
            }
        }
    }

    /// Update input registers from weather reading data.
    pub fn update_input_registers(
        &mut self,
        temp_c: Option<f32>,
        humidity_pct: Option<f32>,
        pressure_hpa: Option<f32>,
        wind_speed_kmh: Option<f32>,
        wind_dir_deg: Option<f32>,
        rain_rate_mm_hr: Option<f32>,
        rain_mm: Option<f32>,
        uv_index: Option<f32>,
        light_lux: Option<f32>,
        heat_index_c: Option<f32>,
        dew_point_c: Option<f32>,
        wind_chill_c: Option<f32>,
        battery_mv: u16,
        battery_pct: u16,
    ) {
        self.input_regs[IR_TEMPERATURE] = temp_c.map_or(0x8000, |v| (v * 100.0) as i16 as u16);
        self.input_regs[IR_HUMIDITY] = humidity_pct.map_or(0, |v| (v * 100.0) as u16);

        if let Some(p) = pressure_hpa {
            let p_scaled = (p * 100.0) as u32;
            self.input_regs[IR_PRESSURE_HI] = (p_scaled >> 16) as u16;
            self.input_regs[IR_PRESSURE_LO] = p_scaled as u16;
        }

        self.input_regs[IR_WIND_SPEED] = wind_speed_kmh.map_or(0, |v| (v * 100.0) as u16);
        self.input_regs[IR_WIND_DIR] = wind_dir_deg.map_or(0, |v| v as u16);
        self.input_regs[IR_RAIN_RATE] = rain_rate_mm_hr.map_or(0, |v| (v * 100.0) as u16);
        self.input_regs[IR_RAIN_ACCUM] = rain_mm.map_or(0, |v| (v * 100.0) as u16);
        self.input_regs[IR_UV_INDEX] = uv_index.map_or(0, |v| (v * 100.0) as u16);

        if let Some(l) = light_lux {
            let l_u32 = l as u32;
            self.input_regs[IR_LIGHT_HI] = (l_u32 >> 16) as u16;
            self.input_regs[IR_LIGHT_LO] = l_u32 as u16;
        }

        self.input_regs[IR_HEAT_INDEX] = heat_index_c.map_or(0x8000, |v| (v * 100.0) as i16 as u16);
        self.input_regs[IR_DEW_POINT] = dew_point_c.map_or(0x8000, |v| (v * 100.0) as i16 as u16);
        self.input_regs[IR_WIND_CHILL] = wind_chill_c.map_or(0x8000, |v| (v * 100.0) as i16 as u16);
        self.input_regs[IR_BATTERY_MV] = battery_mv;
        self.input_regs[IR_BATTERY_PCT] = battery_pct;
    }

    /// Update India-specific input registers.
    pub fn update_india_registers(&mut self, pm25_ugm3: Option<f32>, aqi_naqi: Option<u16>) {
        self.input_regs[IR_PM25] = pm25_ugm3.map_or(0, |v| (v * 10.0) as u16);
        self.input_regs[IR_AQI_NAQI] = aqi_naqi.unwrap_or(0);
    }

    /// Update agriculture-specific input registers.
    pub fn update_agriculture_registers(
        &mut self,
        soil_shallow_pct: Option<f32>,
        soil_deep_pct: Option<f32>,
        soil_temp_c: Option<f32>,
    ) {
        self.input_regs[IR_SOIL_SHALLOW] = soil_shallow_pct.map_or(0, |v| (v * 100.0) as u16);
        self.input_regs[IR_SOIL_DEEP] = soil_deep_pct.map_or(0, |v| (v * 100.0) as u16);
        self.input_regs[IR_SOIL_TEMP] = soil_temp_c.map_or(0x8000, |v| (v * 100.0) as i16 as u16);
    }

    /// Update solar-specific input registers.
    pub fn update_solar_registers(
        &mut self,
        irradiance_w_m2: Option<f32>,
        panel_temp_c: Option<f32>,
        power_w: Option<f32>,
    ) {
        self.input_regs[IR_IRRADIANCE] = irradiance_w_m2.map_or(0, |v| (v * 10.0) as u16);
        self.input_regs[IR_PANEL_TEMP] = panel_temp_c.map_or(0x8000, |v| (v * 100.0) as i16 as u16);
        self.input_regs[IR_SOLAR_POWER] = power_w.map_or(0, |v| (v * 10.0) as u16);
    }

    /// Update discrete input (alert) flags.
    pub fn update_discrete_inputs(
        &mut self,
        heat_wave: bool,
        severe_heat: bool,
        cyclone_watch: bool,
        cyclone_warning: bool,
        cyclone_severe: bool,
        aqi_severe: bool,
        frost_warning: bool,
        sensor_fault: bool,
    ) {
        self.discrete_inputs[DI_HEAT_WAVE] = heat_wave;
        self.discrete_inputs[DI_SEVERE_HEAT] = severe_heat;
        self.discrete_inputs[DI_CYCLONE_WATCH] = cyclone_watch;
        self.discrete_inputs[DI_CYCLONE_WARNING] = cyclone_warning;
        self.discrete_inputs[DI_CYCLONE_SEVERE] = cyclone_severe;
        self.discrete_inputs[DI_AQI_SEVERE] = aqi_severe;
        self.discrete_inputs[DI_FROST_WARNING] = frost_warning;
        self.discrete_inputs[DI_SENSOR_FAULT] = sensor_fault;
    }

    /// Get the response buffer to transmit (if any).
    /// Returns None if no response is pending.
    pub fn take_response(&mut self) -> Option<&[u8]> {
        if self.tx_len > 0 {
            let len = self.tx_len;
            self.tx_len = 0;
            self.state = ModbusState::Idle;
            Some(&self.tx_buf[..len])
        } else {
            None
        }
    }

    /// Get current slave address.
    pub fn slave_addr(&self) -> u8 {
        self.slave_addr
    }

    /// Get current state.
    pub fn state(&self) -> ModbusState {
        self.state
    }

    // ── Private methods ──────────────────────────────

    fn reset_rx(&mut self) {
        self.rx_pos = 0;
        self.state = ModbusState::Idle;
    }

    fn process_frame(&mut self) {
        let len = self.rx_pos;
        self.rx_pos = 0;

        if len < 4 {
            self.stats.frame_errors += 1;
            self.state = ModbusState::Idle;
            return;
        }

        // Verify CRC
        let received_crc = u16::from_le_bytes([self.rx_buf[len - 2], self.rx_buf[len - 1]]);
        let computed_crc = Self::crc16(&self.rx_buf[..len - 2]);
        if received_crc != computed_crc {
            self.stats.crc_errors += 1;
            self.state = ModbusState::Idle;
            return;
        }

        // Check slave address (0 = broadcast)
        let addr = self.rx_buf[0];
        if addr != self.slave_addr && addr != 0 {
            // Not for us — ignore silently
            self.state = ModbusState::Idle;
            return;
        }

        let fc_byte = self.rx_buf[1];
        let fc = match FunctionCode::from_u8(fc_byte) {
            Some(fc) => fc,
            None => {
                self.send_exception(fc_byte, ExceptionCode::IllegalFunction);
                return;
            }
        };

        match fc {
            FunctionCode::ReadDiscreteInputs => self.handle_read_discrete_inputs(),
            FunctionCode::ReadInputRegisters => self.handle_read_input_registers(),
            FunctionCode::ReadHoldingRegisters => self.handle_read_holding_registers(),
            FunctionCode::WriteSingleRegister => self.handle_write_single_register(),
            FunctionCode::WriteMultipleRegisters => self.handle_write_multiple_registers(),
        }
    }

    fn handle_read_discrete_inputs(&mut self) {
        let start = u16::from_be_bytes([self.rx_buf[2], self.rx_buf[3]]) as usize;
        let count = u16::from_be_bytes([self.rx_buf[4], self.rx_buf[5]]) as usize;

        if start + count > DISCRETE_INPUT_COUNT || count == 0 {
            self.send_exception(0x02, ExceptionCode::IllegalDataAddress);
            return;
        }

        let byte_count = (count + 7) / 8;
        self.tx_buf[0] = self.slave_addr;
        self.tx_buf[1] = 0x02;
        self.tx_buf[2] = byte_count as u8;

        for i in 0..byte_count {
            let mut byte_val = 0u8;
            for bit in 0..8 {
                let idx = start + i * 8 + bit;
                if idx < start + count && idx < DISCRETE_INPUT_COUNT {
                    if self.discrete_inputs[idx] {
                        byte_val |= 1 << bit;
                    }
                }
            }
            self.tx_buf[3 + i] = byte_val;
        }

        let payload_len = 3 + byte_count;
        let crc = Self::crc16(&self.tx_buf[..payload_len]);
        self.tx_buf[payload_len] = crc as u8;
        self.tx_buf[payload_len + 1] = (crc >> 8) as u8;
        self.tx_len = payload_len + 2;
        self.stats.requests_ok += 1;
        self.state = ModbusState::Transmitting;
    }

    fn handle_read_input_registers(&mut self) {
        let start = u16::from_be_bytes([self.rx_buf[2], self.rx_buf[3]]) as usize;
        let count = u16::from_be_bytes([self.rx_buf[4], self.rx_buf[5]]) as usize;

        if start + count > INPUT_REGISTER_COUNT
            || count == 0
            || count > modbus::MAX_REGISTERS_PER_READ as usize
        {
            self.send_exception(0x04, ExceptionCode::IllegalDataAddress);
            return;
        }

        self.tx_buf[0] = self.slave_addr;
        self.tx_buf[1] = 0x04;
        self.tx_buf[2] = (count * 2) as u8;

        for i in 0..count {
            let val = self.input_regs[start + i];
            self.tx_buf[3 + i * 2] = (val >> 8) as u8;
            self.tx_buf[4 + i * 2] = val as u8;
        }

        let payload_len = 3 + count * 2;
        let crc = Self::crc16(&self.tx_buf[..payload_len]);
        self.tx_buf[payload_len] = crc as u8;
        self.tx_buf[payload_len + 1] = (crc >> 8) as u8;
        self.tx_len = payload_len + 2;
        self.stats.requests_ok += 1;
        self.state = ModbusState::Transmitting;
    }

    fn handle_read_holding_registers(&mut self) {
        let start = u16::from_be_bytes([self.rx_buf[2], self.rx_buf[3]]) as usize;
        let count = u16::from_be_bytes([self.rx_buf[4], self.rx_buf[5]]) as usize;

        if start + count > HOLDING_REGISTER_COUNT
            || count == 0
            || count > modbus::MAX_REGISTERS_PER_READ as usize
        {
            self.send_exception(0x03, ExceptionCode::IllegalDataAddress);
            return;
        }

        self.tx_buf[0] = self.slave_addr;
        self.tx_buf[1] = 0x03;
        self.tx_buf[2] = (count * 2) as u8;

        for i in 0..count {
            let val = self.holding_regs[start + i];
            self.tx_buf[3 + i * 2] = (val >> 8) as u8;
            self.tx_buf[4 + i * 2] = val as u8;
        }

        let payload_len = 3 + count * 2;
        let crc = Self::crc16(&self.tx_buf[..payload_len]);
        self.tx_buf[payload_len] = crc as u8;
        self.tx_buf[payload_len + 1] = (crc >> 8) as u8;
        self.tx_len = payload_len + 2;
        self.stats.requests_ok += 1;
        self.state = ModbusState::Transmitting;
    }

    fn handle_write_single_register(&mut self) {
        let reg_addr = u16::from_be_bytes([self.rx_buf[2], self.rx_buf[3]]) as usize;
        let value = u16::from_be_bytes([self.rx_buf[4], self.rx_buf[5]]);

        if reg_addr >= HOLDING_REGISTER_COUNT {
            self.send_exception(0x06, ExceptionCode::IllegalDataAddress);
            return;
        }

        if !self.validate_holding_write(reg_addr, value) {
            self.send_exception(0x06, ExceptionCode::IllegalDataValue);
            return;
        }

        self.holding_regs[reg_addr] = value;

        // Apply side effects
        if reg_addr == HR_SLAVE_ADDR {
            let new_addr = value as u8;
            if new_addr >= 1 && new_addr <= 247 {
                self.slave_addr = new_addr;
            }
        }

        // Echo request as response (Modbus FC 0x06 convention)
        self.tx_buf[..6].copy_from_slice(&self.rx_buf[..6]);
        self.tx_buf[0] = self.slave_addr;
        let crc = Self::crc16(&self.tx_buf[..6]);
        self.tx_buf[6] = crc as u8;
        self.tx_buf[7] = (crc >> 8) as u8;
        self.tx_len = 8;
        self.stats.requests_ok += 1;
        self.state = ModbusState::Transmitting;

        log::info!("Modbus: wrote HR[{}] = {}", reg_addr, value);
    }

    fn handle_write_multiple_registers(&mut self) {
        let start = u16::from_be_bytes([self.rx_buf[2], self.rx_buf[3]]) as usize;
        let count = u16::from_be_bytes([self.rx_buf[4], self.rx_buf[5]]) as usize;
        let byte_count = self.rx_buf[6] as usize;

        if start + count > HOLDING_REGISTER_COUNT
            || count == 0
            || byte_count != count * 2
        {
            self.send_exception(0x10, ExceptionCode::IllegalDataAddress);
            return;
        }

        // Validate all values before writing
        for i in 0..count {
            let val = u16::from_be_bytes([self.rx_buf[7 + i * 2], self.rx_buf[8 + i * 2]]);
            if !self.validate_holding_write(start + i, val) {
                self.send_exception(0x10, ExceptionCode::IllegalDataValue);
                return;
            }
        }

        // Apply writes
        for i in 0..count {
            let val = u16::from_be_bytes([self.rx_buf[7 + i * 2], self.rx_buf[8 + i * 2]]);
            self.holding_regs[start + i] = val;
        }

        // Response: addr + FC + start + count + CRC
        self.tx_buf[0] = self.slave_addr;
        self.tx_buf[1] = 0x10;
        self.tx_buf[2..6].copy_from_slice(&self.rx_buf[2..6]);
        let crc = Self::crc16(&self.tx_buf[..6]);
        self.tx_buf[6] = crc as u8;
        self.tx_buf[7] = (crc >> 8) as u8;
        self.tx_len = 8;
        self.stats.requests_ok += 1;
        self.state = ModbusState::Transmitting;

        log::info!("Modbus: wrote {} registers starting at HR[{}]", count, start);
    }

    fn validate_holding_write(&self, reg_addr: usize, value: u16) -> bool {
        match reg_addr {
            HR_SLAVE_ADDR => value >= 1 && value <= 247,
            HR_BAUD_CODE => value <= 4,
            HR_READ_INTERVAL => value >= 1 && value <= 3600,
            HR_OP_MODE => value <= 2,
            HR_INDIA_REGION => value <= 2,
            _ => true,
        }
    }

    fn send_exception(&mut self, fc: u8, code: ExceptionCode) {
        self.tx_buf[0] = self.slave_addr;
        self.tx_buf[1] = fc | 0x80; // Exception: set MSB
        self.tx_buf[2] = code as u8;
        let crc = Self::crc16(&self.tx_buf[..3]);
        self.tx_buf[3] = crc as u8;
        self.tx_buf[4] = (crc >> 8) as u8;
        self.tx_len = 5;
        self.stats.exceptions += 1;
        self.state = ModbusState::Transmitting;
    }

    /// Compute Modbus CRC-16 (polynomial 0xA001).
    fn crc16(data: &[u8]) -> u16 {
        let mut crc: u16 = 0xFFFF;
        for &byte in data {
            crc ^= byte as u16;
            for _ in 0..8 {
                if crc & 0x0001 != 0 {
                    crc = (crc >> 1) ^ 0xA001;
                } else {
                    crc >>= 1;
                }
            }
        }
        crc
    }
}
