/// UART-based driver for the Microchip RN4870 BLE module.
///
/// Connected via USART3 on the STM32F407:
///   PB10 = USART3_TX → RN4870 RX
///   PB11 = USART3_RX ← RN4870 TX
///   PD8  = RST (active low)
///   PD9  = STATUS (input, high when connected)
///
/// The RN4870 uses ASCII commands prefixed with `$` in data mode and
/// `%` in command mode.  All commands are terminated with `\r` and
/// acknowledged with `AOK\r\n` or `Err\r\n`.

use crate::error::{Error, Result};
use heapless::String;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Pin assignments (USART3 on STM32F407)
// ---------------------------------------------------------------------------

/// Pin constants for the RN4870 BLE module on USART3.
pub mod pins {
    /// USART3 TX — PB10.
    pub const USART3_TX: u8 = 26;
    /// USART3 RX — PB11.
    pub const USART3_RX: u8 = 27;
    /// Hardware reset (active low) — PD8.
    pub const RST: u8 = 56;
    /// Connection status (input, high when connected) — PD9.
    pub const STATUS: u8 = 57;
}

// ---------------------------------------------------------------------------
// UART baud rate
// ---------------------------------------------------------------------------

/// UART baud rate for communication with the RN4870 (bps).
pub const UART_BAUD_RATE: u32 = 115_200;

// ---------------------------------------------------------------------------
// Command timing
// ---------------------------------------------------------------------------

/// Delay after entering command mode before sending commands (ms).
const CMD_MODE_DELAY_MS: u64 = 100;

/// Timeout waiting for a command response (ms).
const CMD_RESPONSE_TIMEOUT_MS: u64 = 2_000;

/// Delay after hardware reset before the module is ready (ms).
const RESET_DELAY_MS: u64 = 500;

// ---------------------------------------------------------------------------
// BLE GATT service UUIDs and characteristic handles
// ---------------------------------------------------------------------------

/// Bluetooth SIG Environmental Sensing service UUID.
pub const ENVIRONMENTAL_SENSING_UUID: u16 = 0x181A;

/// Custom weather service UUID.
pub const CUSTOM_WEATHER_SERVICE_UUID: u16 = 0xFFE0;

/// Characteristic handles for the weather GATT profile.
pub mod handles {
    /// Temperature (Environmental Sensing, sint16, 0.01 degC).
    pub const TEMPERATURE: u16 = 0x0072;
    /// Humidity (Environmental Sensing, uint16, 0.01 %).
    pub const HUMIDITY: u16 = 0x0075;
    /// Pressure (Environmental Sensing, uint32, 0.1 Pa).
    pub const PRESSURE: u16 = 0x0078;
    /// Wind speed (Custom, uint16, 0.1 km/h).
    pub const WIND_SPEED: u16 = 0x0082;
    /// Wind direction (Custom, uint16, degrees).
    pub const WIND_DIR: u16 = 0x0085;
    /// Rain accumulation (Custom, uint16, 0.1 mm).
    pub const RAIN: u16 = 0x0088;
    /// UV index (Custom, uint16, 0.1).
    pub const UV: u16 = 0x008B;
    /// Ambient light (Custom, uint32, lux).
    pub const LIGHT: u16 = 0x008E;
}

// ---------------------------------------------------------------------------
// Service configuration string for SS command
// ---------------------------------------------------------------------------

/// Environmental Sensing service bitmask for the `SS` command.
const SERVICE_BITMAP: &str = "C0";

// ---------------------------------------------------------------------------
// BLE module state
// ---------------------------------------------------------------------------

/// Operating state of the RN4870 BLE module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BleModuleState {
    /// Module is powered off (RST held low).
    PowerOff,
    /// Module is in hardware reset sequence.
    Resetting,
    /// Module is in command mode, ready to accept AT commands.
    CommandMode,
    /// Module is being configured (GATT services, name, etc.).
    Configuring,
    /// Module is advertising and waiting for a connection.
    Advertising,
    /// A BLE central is connected.
    Connected,
    /// Module encountered an unrecoverable error.
    Error,
}

// ---------------------------------------------------------------------------
// BLE module info
// ---------------------------------------------------------------------------

/// Runtime information about the RN4870 BLE module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BleModuleInfo {
    /// Current module state.
    pub state: BleModuleState,
    /// Whether a BLE central is currently connected.
    pub connected: bool,
    /// Bluetooth address of the connected peer (e.g. "AA:BB:CC:DD:EE:FF").
    pub peer_addr: String<18>,
    /// Configured BLE device name.
    pub device_name: String<32>,
    /// RN4870 firmware version string.
    pub firmware_version: String<16>,
}

impl Default for BleModuleInfo {
    fn default() -> Self {
        Self {
            state: BleModuleState::PowerOff,
            connected: false,
            peer_addr: String::new(),
            device_name: String::new(),
            firmware_version: String::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Driver struct
// ---------------------------------------------------------------------------

/// UART-based driver for the Microchip RN4870 BLE module.
pub struct Rn4870 {
    /// Current operating state.
    state: BleModuleState,
    /// Public information about the module.
    info: BleModuleInfo,
    /// UART receive ring buffer.
    rx_buf: [u8; 256],
    /// Current write position in `rx_buf`.
    rx_pos: usize,
    /// Whether the module is currently in command mode.
    cmd_mode: bool,
    /// Timestamp of the last command sent (ms), used for timeout detection.
    last_cmd_ms: u64,
}

impl Rn4870 {
    // -------------------------------------------------------------------
    // Construction
    // -------------------------------------------------------------------

    /// Create a new RN4870 driver instance.
    ///
    /// The module starts in `PowerOff` state.  Call [`init`] to reset the
    /// hardware and configure GATT services.
    pub fn new() -> Self {
        Self {
            state: BleModuleState::PowerOff,
            info: BleModuleInfo::default(),
            rx_buf: [0u8; 256],
            rx_pos: 0,
            cmd_mode: false,
            last_cmd_ms: 0,
        }
    }

    // -------------------------------------------------------------------
    // Lifecycle
    // -------------------------------------------------------------------

    /// Full initialisation sequence: hardware reset, enter command mode,
    /// configure GATT services, set device name, and begin advertising.
    pub fn init(&mut self) -> Result<()> {
        log::info!("RN4870: initialising");

        self.reset();
        self.enter_command_mode()?;
        self.configure_services()?;
        self.exit_command_mode()?;
        self.start_advertising()?;

        log::info!("RN4870: initialisation complete");
        Ok(())
    }

    /// Perform a hardware reset by pulsing the RST pin low.
    ///
    /// After reset the module returns to data mode.
    pub fn reset(&mut self) {
        self.state = BleModuleState::Resetting;
        self.info.state = BleModuleState::Resetting;
        self.cmd_mode = false;
        self.rx_pos = 0;

        // In real firmware:
        //   gpio_set_low(pins::RST);
        //   delay_ms(10);
        //   gpio_set_high(pins::RST);
        //   delay_ms(RESET_DELAY_MS);
        spin_delay(100_000);

        self.state = BleModuleState::PowerOff;
        self.info.state = BleModuleState::PowerOff;

        log::info!("RN4870: hardware reset complete");
    }

    /// Enter command mode by sending `$$$`.
    ///
    /// The module responds with `CMD>` when ready.
    pub fn enter_command_mode(&mut self) -> Result<()> {
        if self.cmd_mode {
            return Ok(());
        }

        // Send $$$ (no carriage return for entering command mode)
        self.uart_write(b"$$$");
        spin_delay(50_000);

        // In real firmware we would wait for "CMD>" response
        self.cmd_mode = true;
        self.state = BleModuleState::CommandMode;
        self.info.state = BleModuleState::CommandMode;

        log::debug!("RN4870: entered command mode");
        Ok(())
    }

    /// Exit command mode by sending `---\r`.
    ///
    /// The module responds with `END\r\n`.
    pub fn exit_command_mode(&mut self) -> Result<()> {
        if !self.cmd_mode {
            return Ok(());
        }

        self.send_cmd("---")?;

        self.cmd_mode = false;

        log::debug!("RN4870: exited command mode");
        Ok(())
    }

    /// Set the BLE advertised device name.
    pub fn set_device_name(&mut self, name: &str) -> Result<()> {
        let was_in_cmd = self.cmd_mode;
        if !was_in_cmd {
            self.enter_command_mode()?;
        }

        // Build "SN,<name>\r"
        let mut cmd: String<48> = String::new();
        let _ = cmd.push_str("SN,");
        let _ = cmd.push_str(name);

        self.send_cmd(cmd.as_str())?;

        self.info.device_name.clear();
        let _ = self.info.device_name.push_str(name);

        if !was_in_cmd {
            self.exit_command_mode()?;
        }

        log::info!("RN4870: device name set to '{}'", name);
        Ok(())
    }

    /// Configure GATT services on the RN4870.
    ///
    /// Sets the Environmental Sensing service bitmap, creates
    /// characteristics for all weather data points, and reboots
    /// the module to apply.
    pub fn configure_services(&mut self) -> Result<()> {
        if !self.cmd_mode {
            return Err(Error::BleNotReady);
        }

        self.state = BleModuleState::Configuring;
        self.info.state = BleModuleState::Configuring;

        // Enable Environmental Sensing service
        let mut cmd: String<16> = String::new();
        let _ = cmd.push_str("SS,");
        let _ = cmd.push_str(SERVICE_BITMAP);
        self.send_cmd(cmd.as_str())?;

        // Set default device name
        self.set_device_name("WeatherStation")?;

        // Create characteristics for the environmental sensing service:
        //   PC,<uuid>,<handle>,<properties>,<size>
        // Properties: 02 = Read, 10 = Notify → 12 = Read|Notify
        self.send_cmd("PC,2A6E,0072,12,02")?; // Temperature (sint16)
        self.send_cmd("PC,2A6F,0075,12,02")?; // Humidity (uint16)
        self.send_cmd("PC,2A6D,0078,12,04")?; // Pressure (uint32)

        // Custom weather service characteristics
        self.send_cmd("PC,FFE1,0082,12,02")?; // Wind speed (uint16)
        self.send_cmd("PC,FFE2,0085,12,02")?; // Wind direction (uint16)
        self.send_cmd("PC,FFE3,0088,12,02")?; // Rain (uint16)
        self.send_cmd("PC,FFE4,008B,12,02")?; // UV index (uint16)
        self.send_cmd("PC,FFE5,008E,12,04")?; // Ambient light (uint32)

        // Reboot to apply GATT changes
        self.send_cmd("R,1")?;
        spin_delay(200_000);

        // Re-enter command mode after reboot
        self.cmd_mode = false;
        self.enter_command_mode()?;

        log::info!("RN4870: GATT services configured");
        Ok(())
    }

    /// Start BLE advertising.
    pub fn start_advertising(&mut self) -> Result<()> {
        let was_in_cmd = self.cmd_mode;
        if !was_in_cmd {
            self.enter_command_mode()?;
        }

        self.send_cmd("A")?;

        self.state = BleModuleState::Advertising;
        self.info.state = BleModuleState::Advertising;

        if !was_in_cmd {
            self.exit_command_mode()?;
        }

        log::info!("RN4870: advertising started");
        Ok(())
    }

    /// Stop BLE advertising by entering and immediately exiting command
    /// mode (which implicitly stops advertising on the RN4870).
    pub fn stop_advertising(&mut self) -> Result<()> {
        if self.state != BleModuleState::Advertising {
            return Ok(());
        }

        let was_in_cmd = self.cmd_mode;
        if !was_in_cmd {
            self.enter_command_mode()?;
        }

        // Sending "Y" stops advertising on the RN4870
        self.send_cmd("Y")?;

        self.state = BleModuleState::CommandMode;
        self.info.state = BleModuleState::CommandMode;

        if !was_in_cmd {
            self.exit_command_mode()?;
        }

        log::info!("RN4870: advertising stopped");
        Ok(())
    }

    /// Returns `true` if a BLE central is currently connected.
    pub fn is_connected(&self) -> bool {
        self.info.connected
    }

    /// Write a value to a GATT characteristic by handle.
    ///
    /// `data` is the raw bytes to write; they are hex-encoded for the
    /// `SHW` command.
    pub fn update_characteristic(&mut self, handle: u16, data: &[u8]) -> Result<()> {
        if self.state == BleModuleState::PowerOff || self.state == BleModuleState::Error {
            return Err(Error::BleNotReady);
        }

        let was_in_cmd = self.cmd_mode;
        if !was_in_cmd {
            self.enter_command_mode()?;
        }

        // Build "SHW,<handle>,<hex data>"
        let mut cmd: String<80> = String::new();
        let _ = cmd.push_str("SHW,");
        hex_push_u16(&mut cmd, handle);
        let _ = cmd.push(',');
        for &b in data {
            hex_push_u8(&mut cmd, b);
        }

        self.send_cmd(cmd.as_str())?;

        if !was_in_cmd {
            self.exit_command_mode()?;
        }

        Ok(())
    }

    /// Convenience method to update all weather characteristics at once.
    ///
    /// Values are encoded in the BLE-standard formats:
    /// - `temp_c`: sint16 in units of 0.01 degC
    /// - `humidity`: uint16 in units of 0.01 %
    /// - `pressure`: uint32 in units of 0.1 Pa
    /// - `wind_speed`: uint16 in units of 0.1 km/h
    pub fn update_weather_data(
        &mut self,
        temp_c: f32,
        humidity: f32,
        pressure: f32,
        wind_speed: f32,
    ) -> Result<()> {
        // Temperature — sint16, 0.01 degC resolution
        let temp_raw = (temp_c * 100.0) as i16;
        self.update_characteristic(handles::TEMPERATURE, &temp_raw.to_le_bytes())?;

        // Humidity — uint16, 0.01 % resolution
        let hum_raw = (humidity * 100.0) as u16;
        self.update_characteristic(handles::HUMIDITY, &hum_raw.to_le_bytes())?;

        // Pressure — uint32, 0.1 Pa resolution (input is hPa)
        let press_raw = (pressure * 1000.0) as u32;
        self.update_characteristic(handles::PRESSURE, &press_raw.to_le_bytes())?;

        // Wind speed — uint16, 0.1 km/h resolution
        let wind_raw = (wind_speed * 10.0) as u16;
        self.update_characteristic(handles::WIND_SPEED, &wind_raw.to_le_bytes())?;

        log::debug!(
            "RN4870: weather data updated (T={}, H={}, P={}, W={})",
            temp_c,
            humidity,
            pressure,
            wind_speed,
        );

        Ok(())
    }

    /// Poll the module for incoming UART data and handle async events.
    ///
    /// Should be called periodically from the main loop. `now_ms` is the
    /// current monotonic time in milliseconds.
    pub fn poll(&mut self, now_ms: u64) {
        if self.state == BleModuleState::PowerOff || self.state == BleModuleState::Error {
            return;
        }

        // Check STATUS pin for connection state
        // In real firmware: let status_high = gpio_read(pins::STATUS);
        // if status_high && !self.info.connected {
        //     self.process_event("CONNECT");
        // } else if !status_high && self.info.connected {
        //     self.process_event("DISCONNECT");
        // }

        // Check for command timeout
        if self.last_cmd_ms > 0
            && now_ms.saturating_sub(self.last_cmd_ms) > CMD_RESPONSE_TIMEOUT_MS
        {
            log::warn!("RN4870: command response timeout");
            self.last_cmd_ms = 0;
        }

        // Try to parse any complete lines in the receive buffer
        self.parse_response();
    }

    /// Feed a single byte received from the UART into the driver.
    ///
    /// Called from the USART3 interrupt handler or DMA callback.
    pub fn on_byte_received(&mut self, byte: u8) {
        if self.rx_pos < self.rx_buf.len() {
            self.rx_buf[self.rx_pos] = byte;
            self.rx_pos += 1;
        }

        // If we see a newline, try to parse immediately
        if byte == b'\n' {
            self.parse_response();
        }
    }

    /// Disconnect the currently connected BLE central.
    pub fn disconnect(&mut self) -> Result<()> {
        if !self.info.connected {
            return Ok(());
        }

        let was_in_cmd = self.cmd_mode;
        if !was_in_cmd {
            self.enter_command_mode()?;
        }

        self.send_cmd("K,1")?;

        self.info.connected = false;
        self.info.peer_addr.clear();
        self.state = BleModuleState::Advertising;
        self.info.state = BleModuleState::Advertising;

        if !was_in_cmd {
            self.exit_command_mode()?;
        }

        log::info!("RN4870: disconnected");
        Ok(())
    }

    // -------------------------------------------------------------------
    // Accessors
    // -------------------------------------------------------------------

    /// Get the current module state.
    pub fn state(&self) -> BleModuleState {
        self.state
    }

    /// Get a reference to the module info struct.
    pub fn info(&self) -> &BleModuleInfo {
        &self.info
    }

    // -------------------------------------------------------------------
    // Private helpers
    // -------------------------------------------------------------------

    /// Send a command string to the RN4870 in command mode.
    ///
    /// Appends `\r` and records the timestamp for timeout tracking.
    fn send_cmd(&mut self, cmd: &str) -> Result<()> {
        if !self.cmd_mode {
            return Err(Error::BleNotReady);
        }

        let mut buf: [u8; 80] = [0u8; 80];
        let cmd_bytes = cmd.as_bytes();
        let len = cmd_bytes.len().min(buf.len() - 1);
        buf[..len].copy_from_slice(&cmd_bytes[..len]);
        buf[len] = b'\r';

        self.uart_write(&buf[..len + 1]);

        // Record send time for timeout detection
        // In real firmware: self.last_cmd_ms = now_ms();
        self.rx_pos = 0;

        // Busy-wait for response (simplified; real firmware would be async)
        spin_delay(20_000);

        Ok(())
    }

    /// Parse the contents of `rx_buf` looking for complete response lines.
    ///
    /// Recognises `AOK`, `Err`, `CMD>`, `END`, and asynchronous event
    /// notifications like `%CONNECT,...%` and `%DISCONNECT%`.
    fn parse_response(&mut self) {
        if self.rx_pos == 0 {
            return;
        }

        // Try to find a complete line (terminated by \n or \r)
        let mut line_end = None;
        for i in 0..self.rx_pos {
            if self.rx_buf[i] == b'\n' || self.rx_buf[i] == b'\r' {
                line_end = Some(i);
                break;
            }
        }

        let end = match line_end {
            Some(e) => e,
            None => return,
        };

        // Extract the line as a string
        if let Ok(line) = core::str::from_utf8(&self.rx_buf[..end]) {
            let trimmed = line.trim();

            if !trimmed.is_empty() {
                // Check for async event markers (wrapped in %)
                if trimmed.starts_with('%') {
                    let event = trimmed.trim_start_matches('%').trim_end_matches('%');
                    self.process_event(event);
                } else if trimmed == "AOK" {
                    log::debug!("RN4870: command acknowledged");
                    self.last_cmd_ms = 0;
                } else if trimmed == "Err" {
                    log::warn!("RN4870: command error");
                    self.last_cmd_ms = 0;
                } else if trimmed == "CMD>" {
                    self.cmd_mode = true;
                } else if trimmed == "END" {
                    self.cmd_mode = false;
                }
            }
        }

        // Shift remaining data to the front of the buffer
        let consumed = (end + 1).min(self.rx_pos);
        let remaining = self.rx_pos - consumed;
        if remaining > 0 {
            self.rx_buf.copy_within(consumed..consumed + remaining, 0);
        }
        self.rx_pos = remaining;
    }

    /// Handle an asynchronous event string from the RN4870.
    ///
    /// Known events:
    /// - `CONNECT,<addr>` — a central connected
    /// - `DISCONNECT`     — the central disconnected
    /// - `STREAM_OPEN`    — transparent UART stream opened
    fn process_event(&mut self, event: &str) {
        if event.starts_with("CONNECT,") || event == "CONNECT" {
            self.info.connected = true;
            self.state = BleModuleState::Connected;
            self.info.state = BleModuleState::Connected;

            // Extract peer address if present (e.g. "CONNECT,AABBCCDDEEFF")
            if let Some(addr_hex) = event.strip_prefix("CONNECT,") {
                self.info.peer_addr.clear();
                // Format raw hex into colon-separated pairs
                let bytes = addr_hex.as_bytes();
                let mut i = 0;
                while i + 1 < bytes.len() && i < 12 {
                    if i > 0 {
                        let _ = self.info.peer_addr.push(':');
                    }
                    let _ = self.info.peer_addr.push(bytes[i] as char);
                    let _ = self.info.peer_addr.push(bytes[i + 1] as char);
                    i += 2;
                }
            }

            log::info!("RN4870: connected, peer={}", self.info.peer_addr.as_str());
        } else if event == "DISCONNECT" {
            self.info.connected = false;
            self.info.peer_addr.clear();
            self.state = BleModuleState::Advertising;
            self.info.state = BleModuleState::Advertising;

            log::info!("RN4870: disconnected");
        } else if event == "STREAM_OPEN" {
            log::debug!("RN4870: transparent UART stream opened");
        } else if event.starts_with("REBOOT") {
            self.cmd_mode = false;
            self.state = BleModuleState::PowerOff;
            self.info.state = BleModuleState::PowerOff;
            log::info!("RN4870: module rebooted");
        } else {
            log::debug!("RN4870: unhandled event '{}'", event);
        }
    }

    /// Write raw bytes to the UART peripheral.
    ///
    /// In real firmware this writes to the USART3 data register.
    fn uart_write(&self, data: &[u8]) {
        // Placeholder — in real firmware:
        //   for &byte in data {
        //       while !usart3.sr.read().txe().bit_is_set() {}
        //       usart3.dr.write(|w| w.dr().bits(byte as u16));
        //   }
        let _ = data;
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Busy-wait delay (cycle count, not calibrated).
fn spin_delay(cycles: u32) {
    for _ in 0..cycles {
        core::hint::spin_loop();
    }
}

/// Push a `u8` as two hex ASCII characters onto a heapless string.
fn hex_push_u8<const N: usize>(s: &mut String<N>, byte: u8) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let _ = s.push(HEX[(byte >> 4) as usize] as char);
    let _ = s.push(HEX[(byte & 0x0F) as usize] as char);
}

/// Push a `u16` as four hex ASCII characters onto a heapless string.
fn hex_push_u16<const N: usize>(s: &mut String<N>, value: u16) {
    hex_push_u8(s, (value >> 8) as u8);
    hex_push_u8(s, (value & 0xFF) as u8);
}
