/// SIM7600E-H 4G LTE cellular modem driver via UART4.
///
/// The SIM7600E-H is a multi-band LTE-FDD/LTE-TDD/HSPA+/GSM/GPRS/EDGE
/// module supporting data, SMS, and voice. Communication is via AT commands
/// over UART at 115200 baud. The module requires a specific power-on
/// sequence using the PWRKEY pin.
///
/// # Wiring (UART4)
///
/// | STM32F407 Pin | SIM7600 Pin | Function                    |
/// |---------------|-------------|-----------------------------|
/// | PC10          | RXD         | UART4 TX -> modem RX        |
/// | PC11          | TXD         | UART4 RX <- modem TX        |
/// | PD5           | PWRKEY      | Power key (active low pulse) |
/// | PD6           | STATUS      | Module status (input, high=on) |
/// | PD7           | RESET       | Hardware reset (active low)  |
/// | PE2           | DTR         | Data Terminal Ready          |
///
/// # Power-On Sequence
///
/// 1. Drive PWRKEY low for >= 500ms
/// 2. Release PWRKEY (high)
/// 3. Wait for STATUS pin to go high (up to 10s)
/// 4. Wait ~3s for UART to become ready
/// 5. Send AT commands to initialize

use crate::error::{Error, Result};
use heapless::String;
use serde::{Deserialize, Serialize};

/// Pin assignments for UART4 / SIM7600 control lines.
pub mod pins {
    /// PC10 — UART4 TX (to SIM7600 RXD).
    pub const UART4_TX: u8 = 42;
    /// PC11 — UART4 RX (from SIM7600 TXD).
    pub const UART4_RX: u8 = 43;
    /// PD5 — SIM7600 PWRKEY (active low pulse to toggle power).
    pub const SIM7600_PWRKEY: u8 = 53;
    /// PD6 — SIM7600 STATUS (input, high when module is on).
    pub const SIM7600_STATUS: u8 = 54;
    /// PD7 — SIM7600 RESET (active low, hardware reset).
    pub const SIM7600_RESET: u8 = 55;
    /// PE2 — SIM7600 DTR (Data Terminal Ready, low = active).
    pub const SIM7600_DTR: u8 = 66;
}

// =============================================================================
// UART Configuration
// =============================================================================

/// Default baud rate for AT command interface.
pub const UART_BAUD_RATE: u32 = 115_200;

// =============================================================================
// AT Command Constants
// =============================================================================

/// Basic AT attention command.
const AT: &[u8] = b"AT\r\n";
/// Disable command echo.
const AT_ECHO_OFF: &[u8] = b"ATE0\r\n";
/// Query SIM card status.
const AT_CPIN: &[u8] = b"AT+CPIN?\r\n";
/// Query signal quality (RSSI, BER).
const AT_CSQ: &[u8] = b"AT+CSQ\r\n";
/// Query network registration status.
const AT_CREG: &[u8] = b"AT+CREG?\r\n";
/// Query GPRS registration status.
const AT_CGREG: &[u8] = b"AT+CGREG?\r\n";
/// Set PDP context (APN configuration).
const AT_CGDCONT: &[u8] = b"AT+CGDCONT=";
/// Open a network connection.
const AT_CIPOPEN: &[u8] = b"AT+CIPOPEN=";
/// Send data on a connection.
const AT_CIPSEND: &[u8] = b"AT+CIPSEND=";
/// Close a network connection.
const AT_CIPCLOSE: &[u8] = b"AT+CIPCLOSE=";
/// Power off the module.
const AT_CPOWD: &[u8] = b"AT+CPOWD=1\r\n";
/// Set phone functionality.
const AT_CFUN: &[u8] = b"AT+CFUN=";
/// Query IMEI (serial number).
const AT_CGSN: &[u8] = b"AT+CGSN\r\n";
/// Query IMSI (SIM identity).
const AT_CIMI: &[u8] = b"AT+CIMI\r\n";
/// Query operator name.
const AT_COPS: &[u8] = b"AT+COPS?\r\n";
/// Start data connection.
const AT_NETOPEN: &[u8] = b"AT+NETOPEN\r\n";
/// Close data connection.
const AT_NETCLOSE: &[u8] = b"AT+NETCLOSE\r\n";
/// Query IP address.
const AT_IPADDR: &[u8] = b"AT+IPADDR\r\n";
/// Send SMS in text mode.
const AT_CMGF: &[u8] = b"AT+CMGF=1\r\n";
/// Send SMS.
const AT_CMGS: &[u8] = b"AT+CMGS=";
/// Enable URC for network registration.
const AT_CREG_URC: &[u8] = b"AT+CREG=1\r\n";
/// Enable URC for GPRS registration.
const AT_CGREG_URC: &[u8] = b"AT+CGREG=1\r\n";
/// Set error report format to verbose.
const AT_CMEE: &[u8] = b"AT+CMEE=2\r\n";

/// Expected "OK" response suffix.
const RESP_OK: &[u8] = b"OK";
/// Expected "ERROR" response.
const RESP_ERROR: &[u8] = b"ERROR";
/// CPIN ready response.
const RESP_CPIN_READY: &[u8] = b"+CPIN: READY";

/// AT command response timeout (ms).
const AT_DEFAULT_TIMEOUT_MS: u32 = 5_000;
/// Extended timeout for network operations (ms).
const AT_NETWORK_TIMEOUT_MS: u32 = 30_000;
/// PWRKEY pulse duration (ms equivalent in cycles).
const PWRKEY_PULSE_MS: u32 = 500;
/// Maximum time to wait for STATUS after power-on (ms).
const POWER_ON_WAIT_MS: u32 = 10_000;

/// Maximum receive buffer size.
const RX_BUF_SIZE: usize = 512;
/// Maximum number of TCP connections supported.
const MAX_CONNECTIONS: usize = 10;

// =============================================================================
// Types
// =============================================================================

/// State of the cellular modem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CellularState {
    /// Module is powered off.
    PowerOff,
    /// Module is booting (PWRKEY pulsed, waiting for STATUS).
    Booting,
    /// Checking SIM card presence and PIN.
    SimCheck,
    /// Registering on the cellular network.
    Registering,
    /// Registered on the network (voice/SMS available).
    Registered,
    /// Establishing data (PDP/IP) connection.
    DataConnecting,
    /// Data connection is active (IP assigned).
    DataConnected,
    /// Unrecoverable error state.
    Error,
}

/// Registration status from AT+CREG/AT+CGREG.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistrationStatus {
    NotRegistered = 0,
    RegisteredHome = 1,
    Searching = 2,
    Denied = 3,
    Unknown = 4,
    RegisteredRoaming = 5,
}

impl From<u8> for RegistrationStatus {
    fn from(val: u8) -> Self {
        match val {
            0 => RegistrationStatus::NotRegistered,
            1 => RegistrationStatus::RegisteredHome,
            2 => RegistrationStatus::Searching,
            3 => RegistrationStatus::Denied,
            5 => RegistrationStatus::RegisteredRoaming,
            _ => RegistrationStatus::Unknown,
        }
    }
}

/// Cellular modem status information.
#[derive(Debug, Clone)]
pub struct CellularInfo {
    /// Current modem state.
    pub state: CellularState,
    /// Received Signal Strength Indicator in dBm (e.g., -85).
    /// 0 means unknown.
    pub rssi_dbm: i8,
    /// Bit Error Rate (0-7, 99 = unknown).
    pub ber: u8,
    /// Operator name (e.g., "Airtel", "Jio").
    pub operator: String<32>,
    /// IMEI (International Mobile Equipment Identity).
    pub imei: String<16>,
    /// Network registration status.
    pub registration_status: u8,
    /// Assigned IP address (from PDP context).
    pub ip_addr: [u8; 4],
}

impl Default for CellularInfo {
    fn default() -> Self {
        Self {
            state: CellularState::PowerOff,
            rssi_dbm: 0,
            ber: 99,
            operator: String::new(),
            imei: String::new(),
            registration_status: 0,
            ip_addr: [0; 4],
        }
    }
}

/// Unsolicited Result Code (URC) type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Urc {
    /// Network registration change: +CREG: <stat>
    Creg(u8),
    /// GPRS registration change: +CGREG: <stat>
    Cgreg(u8),
    /// Incoming call.
    Ring,
    /// Connection closed by remote: +CIPCLOSE: <link_num>,<reason>
    CipClose(u8),
    /// Incoming data: +CIPRXGET: <mode>,<link_num>,<data_len>
    CipRxGet(u8, u16),
    /// Unknown / unhandled URC.
    Unknown,
}

/// SIM7600E-H 4G LTE cellular modem driver.
///
/// Manages the modem power state, AT command sequencing, network
/// registration, and TCP/IP data connections. Incoming bytes from
/// the UART should be fed to `on_byte_received()` which buffers
/// and parses responses and URCs.
pub struct Sim7600<UART, PWRKEY, STATUS, RST, DTR> {
    uart: UART,
    pwrkey: PWRKEY,
    status: STATUS,
    rst: RST,
    dtr: DTR,
    state: CellularState,
    info: CellularInfo,
    /// Internal receive buffer for AT responses.
    rx_buf: [u8; RX_BUF_SIZE],
    /// Current write position in rx_buf.
    rx_pos: usize,
    /// Timestamp (ms) of the last AT command sent.
    last_at_ms: u32,
    /// Timestamp (ms) when power-on sequence started.
    power_on_ms: u32,
    /// Current AT command timeout (ms).
    at_timeout_ms: u32,
    /// True if we are waiting for an AT response.
    awaiting_response: bool,
    /// True if the last response was OK.
    last_response_ok: bool,
    /// Initialization step counter.
    init_step: u8,
}

impl<UART, PWRKEY, STATUS, RST, DTR> Sim7600<UART, PWRKEY, STATUS, RST, DTR>
where
    UART: embedded_hal_nb::serial::Write<u8> + embedded_hal_nb::serial::Read<u8>,
    PWRKEY: embedded_hal::digital::OutputPin,
    STATUS: embedded_hal::digital::InputPin,
    RST: embedded_hal::digital::OutputPin,
    DTR: embedded_hal::digital::OutputPin,
{
    /// Create a new SIM7600 driver instance.
    ///
    /// The modem starts in `PowerOff` state. Call `power_on()` to begin
    /// the boot sequence.
    pub fn new(uart: UART, pwrkey: PWRKEY, status: STATUS, rst: RST, dtr: DTR) -> Self {
        Self {
            uart,
            pwrkey,
            status,
            rst,
            dtr,
            state: CellularState::PowerOff,
            info: CellularInfo::default(),
            rx_buf: [0u8; RX_BUF_SIZE],
            rx_pos: 0,
            last_at_ms: 0,
            power_on_ms: 0,
            at_timeout_ms: AT_DEFAULT_TIMEOUT_MS,
            awaiting_response: false,
            last_response_ok: false,
            init_step: 0,
        }
    }

    /// Power on the SIM7600 module.
    ///
    /// Drives PWRKEY low for 500ms, then waits for the STATUS pin to go
    /// high (indicating the module has booted). The actual waiting is done
    /// in `poll()` to avoid blocking.
    pub fn power_on(&mut self, now_ms: u32) -> Result<()> {
        // Ensure DTR is low (active — modem stays awake)
        self.dtr.set_low().map_err(|_| Error::SpiBusError)?;

        // Pulse PWRKEY low for >= 500ms
        self.pwrkey.set_low().map_err(|_| Error::SpiBusError)?;
        spin_delay(PWRKEY_PULSE_MS * 10_000); // ~500ms busy wait
        self.pwrkey.set_high().map_err(|_| Error::SpiBusError)?;

        self.state = CellularState::Booting;
        self.info.state = CellularState::Booting;
        self.power_on_ms = now_ms;
        self.init_step = 0;

        log::info!("SIM7600: power-on sequence initiated");
        Ok(())
    }

    /// Power off the SIM7600 module gracefully via AT command.
    ///
    /// Falls back to PWRKEY pulse if AT command fails.
    pub fn power_off(&mut self) -> Result<()> {
        // Try graceful shutdown via AT+CPOWD
        let _ = self.send_at(AT_CPOWD);
        spin_delay(2_000_000);

        // If still on, force off via PWRKEY pulse
        if self.is_status_high() {
            self.pwrkey.set_low().map_err(|_| Error::SpiBusError)?;
            spin_delay(PWRKEY_PULSE_MS * 30_000); // ~1.5s for power-off
            self.pwrkey.set_high().map_err(|_| Error::SpiBusError)?;
            spin_delay(5_000_000);
        }

        self.state = CellularState::PowerOff;
        self.info.state = CellularState::PowerOff;
        self.awaiting_response = false;

        log::info!("SIM7600: powered off");
        Ok(())
    }

    /// Initialize the modem after boot. Sends the startup AT command sequence.
    ///
    /// This is called internally by `poll()` once the STATUS pin goes high.
    /// The initialization proceeds step by step across multiple poll cycles.
    pub fn init(&mut self, now_ms: u32) -> Result<()> {
        if self.awaiting_response {
            return Ok(()); // Wait for current command to complete
        }

        match self.init_step {
            0 => {
                // Send basic AT to test communication
                self.send_at(AT)?;
                self.at_timeout_ms = 2_000;
                self.init_step = 1;
            }
            1 => {
                if !self.last_response_ok {
                    // Retry AT command
                    self.init_step = 0;
                    return Ok(());
                }
                // Disable echo
                self.send_at(AT_ECHO_OFF)?;
                self.init_step = 2;
            }
            2 => {
                // Enable verbose error reporting
                self.send_at(AT_CMEE)?;
                self.init_step = 3;
            }
            3 => {
                // Enable network registration URCs
                self.send_at(AT_CREG_URC)?;
                self.init_step = 4;
            }
            4 => {
                // Enable GPRS registration URCs
                self.send_at(AT_CGREG_URC)?;
                self.init_step = 5;
            }
            5 => {
                // Query IMEI
                self.send_at(AT_CGSN)?;
                self.init_step = 6;
            }
            6 => {
                // Check SIM card
                self.send_at(AT_CPIN)?;
                self.at_timeout_ms = AT_DEFAULT_TIMEOUT_MS;
                self.init_step = 7;
            }
            7 => {
                // SIM check done, move to registering
                self.state = CellularState::SimCheck;
                self.info.state = CellularState::SimCheck;

                // Set full functionality
                self.send_at_with_param(AT_CFUN, b"1\r\n")?;
                self.at_timeout_ms = AT_NETWORK_TIMEOUT_MS;
                self.init_step = 8;
            }
            8 => {
                // Query registration status
                self.send_at(AT_CREG)?;
                self.init_step = 9;
            }
            9 => {
                // Check if registered
                let reg = RegistrationStatus::from(self.info.registration_status);
                if reg == RegistrationStatus::RegisteredHome
                    || reg == RegistrationStatus::RegisteredRoaming
                {
                    self.state = CellularState::Registered;
                    self.info.state = CellularState::Registered;

                    // Query operator
                    self.send_at(AT_COPS)?;
                    self.init_step = 10;
                } else if reg == RegistrationStatus::Searching {
                    self.state = CellularState::Registering;
                    self.info.state = CellularState::Registering;
                    // Will be re-checked in poll()
                    self.init_step = 8;
                } else {
                    // Not yet registered, retry
                    self.init_step = 8;
                }
            }
            10 => {
                // Query signal quality
                self.send_at(AT_CSQ)?;
                self.init_step = 11;
            }
            11 => {
                // Initialization complete
                self.at_timeout_ms = AT_DEFAULT_TIMEOUT_MS;
                log::info!(
                    "SIM7600: initialized, IMEI={}, operator={}, RSSI={}dBm",
                    self.info.imei.as_str(),
                    self.info.operator.as_str(),
                    self.info.rssi_dbm,
                );
                self.init_step = 255; // Done
            }
            _ => {}
        }

        Ok(())
    }

    /// Main poll function. Should be called periodically from the main loop.
    ///
    /// Handles: power-on sequencing, AT command timeout, initialization
    /// steps, and periodic signal quality checks.
    pub fn poll(&mut self, now_ms: u32) -> Result<()> {
        // Drain incoming UART bytes
        self.drain_uart();

        match self.state {
            CellularState::PowerOff => {
                // Nothing to do
            }
            CellularState::Booting => {
                // Wait for STATUS pin to go high
                if self.is_status_high() {
                    // Allow 3 seconds for UART to stabilize after STATUS
                    if now_ms.wrapping_sub(self.power_on_ms) > 3_000 {
                        log::info!("SIM7600: module booted, starting init");
                        self.init_step = 0;
                        self.state = CellularState::SimCheck;
                        self.info.state = CellularState::SimCheck;
                    }
                } else if now_ms.wrapping_sub(self.power_on_ms) > POWER_ON_WAIT_MS {
                    log::error!("SIM7600: boot timeout, STATUS not high");
                    self.state = CellularState::Error;
                    self.info.state = CellularState::Error;
                    return Err(Error::SpiBusError);
                }
            }
            CellularState::SimCheck
            | CellularState::Registering
            | CellularState::Registered => {
                // Check AT command timeout
                if self.awaiting_response {
                    if now_ms.wrapping_sub(self.last_at_ms) > self.at_timeout_ms {
                        log::warn!("SIM7600: AT command timeout");
                        self.awaiting_response = false;
                        self.last_response_ok = false;
                        self.rx_pos = 0;
                    }
                    return Ok(());
                }

                // Continue initialization if not done
                if self.init_step < 255 {
                    self.init(now_ms)?;
                }
            }
            CellularState::DataConnecting => {
                if self.awaiting_response {
                    if now_ms.wrapping_sub(self.last_at_ms) > AT_NETWORK_TIMEOUT_MS {
                        log::warn!("SIM7600: data connect timeout");
                        self.awaiting_response = false;
                        self.state = CellularState::Registered;
                        self.info.state = CellularState::Registered;
                    }
                }
            }
            CellularState::DataConnected => {
                // Periodic signal quality check every 60 seconds
                if !self.awaiting_response
                    && now_ms.wrapping_sub(self.last_at_ms) > 60_000
                {
                    let _ = self.signal_quality();
                }
            }
            CellularState::Error => {
                // Could attempt recovery here
            }
        }

        Ok(())
    }

    /// Query signal quality. Returns (RSSI in dBm, BER).
    pub fn signal_quality(&mut self) -> Result<(i8, u8)> {
        self.send_at(AT_CSQ)?;
        // Response will be parsed asynchronously; return cached values
        Ok((self.info.rssi_dbm, self.info.ber))
    }

    /// Returns `true` if the modem is registered on the network.
    pub fn is_registered(&self) -> bool {
        let reg = RegistrationStatus::from(self.info.registration_status);
        reg == RegistrationStatus::RegisteredHome
            || reg == RegistrationStatus::RegisteredRoaming
    }

    /// Set the APN (Access Point Name) for data connections.
    ///
    /// # Example
    /// ```ignore
    /// modem.set_apn(b"internet")?;
    /// ```
    pub fn set_apn(&mut self, apn: &[u8]) -> Result<()> {
        // AT+CGDCONT=1,"IP","<apn>"
        self.tx_buf_clear();
        self.tx_write(AT_CGDCONT);
        self.tx_write(b"1,\"IP\",\"");
        self.tx_write(apn);
        self.tx_write(b"\"\r\n");
        self.tx_flush()?;
        self.awaiting_response = true;
        Ok(())
    }

    /// Establish a data connection (PDP context activation).
    pub fn data_connect(&mut self) -> Result<()> {
        if !self.is_registered() {
            return Err(Error::SpiBusError);
        }

        self.state = CellularState::DataConnecting;
        self.info.state = CellularState::DataConnecting;
        self.at_timeout_ms = AT_NETWORK_TIMEOUT_MS;

        self.send_at(AT_NETOPEN)?;
        Ok(())
    }

    /// Open a TCP connection.
    ///
    /// # Arguments
    /// - `link_num`: connection number (0-9)
    /// - `ip`: destination IP as a dotted-decimal string (e.g., b"192.168.1.1")
    /// - `port`: destination TCP port
    pub fn open_tcp(&mut self, link_num: u8, ip: &[u8], port: u16) -> Result<()> {
        if self.state != CellularState::DataConnected {
            return Err(Error::SpiBusError);
        }
        if link_num as usize >= MAX_CONNECTIONS {
            return Err(Error::InvalidConfig);
        }

        // AT+CIPOPEN=<link_num>,"TCP","<ip>",<port>
        self.tx_buf_clear();
        self.tx_write(AT_CIPOPEN);
        self.tx_write_u8(link_num);
        self.tx_write(b",\"TCP\",\"");
        self.tx_write(ip);
        self.tx_write(b"\",");
        self.tx_write_u16(port);
        self.tx_write(b"\r\n");
        self.tx_flush()?;

        self.awaiting_response = true;
        self.at_timeout_ms = AT_NETWORK_TIMEOUT_MS;
        Ok(())
    }

    /// Send data on an open TCP connection.
    ///
    /// The data is sent in command mode using AT+CIPSEND.
    pub fn send_tcp(&mut self, link_num: u8, data: &[u8]) -> Result<()> {
        if data.is_empty() || data.len() > 1460 {
            return Err(Error::InvalidConfig);
        }

        // AT+CIPSEND=<link_num>,<length>
        self.tx_buf_clear();
        self.tx_write(AT_CIPSEND);
        self.tx_write_u8(link_num);
        self.tx_write(b",");
        self.tx_write_u16(data.len() as u16);
        self.tx_write(b"\r\n");
        self.tx_flush()?;

        // Wait for '>' prompt (simplified: small delay then send data)
        spin_delay(50_000);

        // Send the actual data
        for &byte in data {
            self.uart_write_byte(byte)?;
        }

        self.awaiting_response = true;
        self.at_timeout_ms = AT_NETWORK_TIMEOUT_MS;
        Ok(())
    }

    /// Receive data from a TCP connection into the provided buffer.
    ///
    /// Returns the number of bytes read (0 if no data available).
    /// Data is collected by `on_byte_received()` and buffered internally.
    pub fn recv_tcp(&mut self, _link_num: u8, buf: &mut [u8]) -> Result<usize> {
        // In a full implementation, incoming +CIPRXGET data would be
        // buffered per-connection. For now, copy from the internal
        // rx_buf if data is available.
        let available = self.rx_pos.min(buf.len());
        if available == 0 {
            return Ok(0);
        }

        buf[..available].copy_from_slice(&self.rx_buf[..available]);
        // Shift remaining data
        let remaining = self.rx_pos - available;
        if remaining > 0 {
            self.rx_buf.copy_within(available..self.rx_pos, 0);
        }
        self.rx_pos = remaining;

        Ok(available)
    }

    /// Close a TCP connection.
    pub fn close_tcp(&mut self, link_num: u8) -> Result<()> {
        // AT+CIPCLOSE=<link_num>
        self.tx_buf_clear();
        self.tx_write(AT_CIPCLOSE);
        self.tx_write_u8(link_num);
        self.tx_write(b"\r\n");
        self.tx_flush()?;

        self.awaiting_response = true;
        self.at_timeout_ms = AT_DEFAULT_TIMEOUT_MS;
        Ok(())
    }

    /// Send an SMS message in text mode.
    ///
    /// # Arguments
    /// - `number`: phone number as ASCII bytes (e.g., b"+919876543210")
    /// - `message`: SMS text content (ASCII)
    pub fn send_sms(&mut self, number: &[u8], message: &[u8]) -> Result<()> {
        // Set text mode
        self.send_at(AT_CMGF)?;
        // Wait for OK (simplified)
        spin_delay(500_000);
        self.drain_uart();

        // AT+CMGS="<number>"
        self.tx_buf_clear();
        self.tx_write(AT_CMGS);
        self.tx_write(b"\"");
        self.tx_write(number);
        self.tx_write(b"\"\r\n");
        self.tx_flush()?;

        // Wait for '>' prompt
        spin_delay(100_000);

        // Send message text followed by Ctrl-Z (0x1A)
        for &byte in message {
            self.uart_write_byte(byte)?;
        }
        self.uart_write_byte(0x1A)?; // Ctrl-Z to send

        self.awaiting_response = true;
        self.at_timeout_ms = AT_NETWORK_TIMEOUT_MS;
        Ok(())
    }

    /// Returns `true` if the modem has an active data connection.
    pub fn is_data_connected(&self) -> bool {
        self.state == CellularState::DataConnected
    }

    /// Returns the current modem state.
    pub fn state(&self) -> CellularState {
        self.state
    }

    /// Returns a reference to the current cellular info.
    pub fn info(&self) -> &CellularInfo {
        &self.info
    }

    // =========================================================================
    // AT command engine
    // =========================================================================

    /// Send an AT command (raw bytes). Resets the receive buffer and marks
    /// the driver as awaiting a response.
    fn send_at(&mut self, cmd: &[u8]) -> Result<()> {
        self.rx_pos = 0;
        self.awaiting_response = true;
        self.last_response_ok = false;

        for &byte in cmd {
            self.uart_write_byte(byte)?;
        }

        // Record timestamp (caller must provide via poll)
        // In practice, last_at_ms is set by the poll loop.
        Ok(())
    }

    /// Send an AT command prefix followed by a parameter suffix.
    fn send_at_with_param(&mut self, prefix: &[u8], param: &[u8]) -> Result<()> {
        self.rx_pos = 0;
        self.awaiting_response = true;
        self.last_response_ok = false;

        for &byte in prefix {
            self.uart_write_byte(byte)?;
        }
        for &byte in param {
            self.uart_write_byte(byte)?;
        }

        Ok(())
    }

    /// Check if the response buffer contains an expected pattern.
    ///
    /// Returns `true` if `expected` is found as a substring.
    fn expect_response(&self, expected: &[u8]) -> bool {
        if self.rx_pos < expected.len() {
            return false;
        }
        // Linear search for substring
        for i in 0..=(self.rx_pos - expected.len()) {
            if self.rx_buf[i..i + expected.len()] == *expected {
                return true;
            }
        }
        false
    }

    /// Parse the AT response buffer for known response patterns.
    ///
    /// Called whenever a complete line (terminated by \r\n) is detected.
    fn parse_response(&mut self) {
        let buf = &self.rx_buf[..self.rx_pos];

        // Check for final response codes
        if contains_subsequence(buf, RESP_OK) {
            self.last_response_ok = true;
            self.awaiting_response = false;

            // Check if NETOPEN succeeded
            if self.state == CellularState::DataConnecting {
                self.state = CellularState::DataConnected;
                self.info.state = CellularState::DataConnected;
                log::info!("SIM7600: data connected");
            }
        } else if contains_subsequence(buf, RESP_ERROR) {
            self.last_response_ok = false;
            self.awaiting_response = false;
            log::warn!("SIM7600: AT error response");
        }

        // Parse informational responses
        self.parse_info_responses(buf);
    }

    /// Parse informational / query responses from the buffer.
    fn parse_info_responses(&mut self, buf: &[u8]) {
        // +CSQ: <rssi>,<ber>
        if let Some(pos) = find_subsequence(buf, b"+CSQ: ") {
            let start = pos + 6;
            if let Some((rssi, ber)) = self.parse_csq(&buf[start..]) {
                // Convert RSSI code to dBm: -113 + 2*rssi
                self.info.rssi_dbm = if rssi == 99 {
                    0 // Unknown
                } else {
                    (-113i16 + 2 * rssi as i16) as i8
                };
                self.info.ber = ber;
            }
        }

        // +CREG: <n>,<stat>
        if let Some(pos) = find_subsequence(buf, b"+CREG: ") {
            let start = pos + 7;
            if let Some(stat) = self.parse_creg(&buf[start..]) {
                self.info.registration_status = stat;
                let reg = RegistrationStatus::from(stat);
                if reg == RegistrationStatus::RegisteredHome
                    || reg == RegistrationStatus::RegisteredRoaming
                {
                    if self.state == CellularState::Registering
                        || self.state == CellularState::SimCheck
                    {
                        self.state = CellularState::Registered;
                        self.info.state = CellularState::Registered;
                        log::info!("SIM7600: registered on network");
                    }
                }
            }
        }

        // +CGREG: <n>,<stat>
        if let Some(pos) = find_subsequence(buf, b"+CGREG: ") {
            let start = pos + 8;
            if let Some(_stat) = self.parse_creg(&buf[start..]) {
                // GPRS registration tracked separately if needed
            }
        }

        // +CPIN: READY
        if contains_subsequence(buf, RESP_CPIN_READY) {
            log::info!("SIM7600: SIM card ready");
        }

        // IMEI response (numeric line after AT+CGSN)
        if self.info.imei.is_empty() && buf.len() >= 15 {
            if let Some(imei) = self.try_parse_imei(buf) {
                self.info.imei = imei;
            }
        }

        // +COPS: <mode>,<format>,"<operator>"
        if let Some(pos) = find_subsequence(buf, b"+COPS: ") {
            self.parse_cops(&buf[pos + 7..]);
        }

        // +IPADDR: <ip>
        if let Some(pos) = find_subsequence(buf, b"+IPADDR: ") {
            self.parse_ipaddr(&buf[pos + 9..]);
        }
    }

    /// Process a single byte received from the UART.
    ///
    /// Bytes are accumulated in the rx_buf. When a complete line (\n) is
    /// detected, the response is parsed. URCs are detected and dispatched.
    pub fn on_byte_received(&mut self, byte: u8) {
        if self.rx_pos < RX_BUF_SIZE {
            self.rx_buf[self.rx_pos] = byte;
            self.rx_pos += 1;
        }

        // Parse on newline
        if byte == b'\n' {
            // Check for URCs first
            if let Some(urc) = self.detect_urc() {
                self.handle_urc(urc);
            }

            // Parse response if awaiting one
            if self.awaiting_response {
                self.parse_response();
            }

            // If response is complete, reset buffer for next command
            if !self.awaiting_response {
                self.rx_pos = 0;
            }
        }
    }

    // =========================================================================
    // URC (Unsolicited Result Code) handling
    // =========================================================================

    /// Detect a URC in the current receive buffer line.
    fn detect_urc(&self) -> Option<Urc> {
        let buf = &self.rx_buf[..self.rx_pos];

        if contains_subsequence(buf, b"RING") {
            return Some(Urc::Ring);
        }

        if let Some(pos) = find_subsequence(buf, b"+CREG: ") {
            let start = pos + 7;
            // URC format: +CREG: <stat> (single digit, no <n> prefix)
            if start < buf.len() {
                let stat = buf[start].wrapping_sub(b'0');
                if stat <= 5 {
                    return Some(Urc::Creg(stat));
                }
            }
        }

        if let Some(pos) = find_subsequence(buf, b"+CGREG: ") {
            let start = pos + 8;
            if start < buf.len() {
                let stat = buf[start].wrapping_sub(b'0');
                if stat <= 5 {
                    return Some(Urc::Cgreg(stat));
                }
            }
        }

        if let Some(pos) = find_subsequence(buf, b"+CIPCLOSE: ") {
            let start = pos + 11;
            if start < buf.len() {
                let link = buf[start].wrapping_sub(b'0');
                return Some(Urc::CipClose(link));
            }
        }

        None
    }

    /// Handle a detected URC.
    fn handle_urc(&mut self, urc: Urc) {
        match urc {
            Urc::Creg(stat) => {
                self.info.registration_status = stat;
                let reg = RegistrationStatus::from(stat);
                log::info!("SIM7600: URC CREG={} ({:?})", stat, reg);

                if reg == RegistrationStatus::RegisteredHome
                    || reg == RegistrationStatus::RegisteredRoaming
                {
                    if self.state == CellularState::Registering {
                        self.state = CellularState::Registered;
                        self.info.state = CellularState::Registered;
                    }
                } else if reg == RegistrationStatus::NotRegistered
                    || reg == RegistrationStatus::Denied
                {
                    if self.state == CellularState::Registered
                        || self.state == CellularState::DataConnected
                    {
                        self.state = CellularState::Registering;
                        self.info.state = CellularState::Registering;
                    }
                }
            }
            Urc::Cgreg(stat) => {
                log::debug!("SIM7600: URC CGREG={}", stat);
            }
            Urc::Ring => {
                log::info!("SIM7600: incoming call (RING)");
                // Auto-hang-up: not handling voice calls
            }
            Urc::CipClose(link) => {
                log::info!("SIM7600: connection {} closed by remote", link);
            }
            Urc::CipRxGet(link, len) => {
                log::debug!("SIM7600: data available on link {}, {} bytes", link, len);
            }
            Urc::Unknown => {}
        }
    }

    // =========================================================================
    // Response parsers
    // =========================================================================

    /// Parse +CSQ response: <rssi>,<ber>
    fn parse_csq(&self, data: &[u8]) -> Option<(u8, u8)> {
        let mut rssi: u8 = 0;
        let mut ber: u8 = 0;
        let mut pos = 0;
        let mut parsing_ber = false;

        while pos < data.len() {
            let ch = data[pos];
            if ch == b',' {
                parsing_ber = true;
                pos += 1;
                continue;
            }
            if ch == b'\r' || ch == b'\n' {
                break;
            }
            if ch >= b'0' && ch <= b'9' {
                if parsing_ber {
                    ber = ber.wrapping_mul(10).wrapping_add(ch - b'0');
                } else {
                    rssi = rssi.wrapping_mul(10).wrapping_add(ch - b'0');
                }
            }
            pos += 1;
        }

        Some((rssi, ber))
    }

    /// Parse +CREG/+CGREG response. Handles both:
    /// - Query response: <n>,<stat>
    /// - URC format: <stat>
    fn parse_creg(&self, data: &[u8]) -> Option<u8> {
        // Skip <n>, if present (look for comma)
        let mut pos = 0;
        while pos < data.len() && data[pos] != b'\r' && data[pos] != b'\n' {
            if data[pos] == b',' {
                pos += 1;
                break;
            }
            pos += 1;
        }

        // If no comma found, reset to start (URC format)
        if pos >= data.len() || data[pos.wrapping_sub(1)] != b',' {
            pos = 0;
        }

        // Parse stat digit
        if pos < data.len() && data[pos] >= b'0' && data[pos] <= b'9' {
            Some(data[pos] - b'0')
        } else {
            None
        }
    }

    /// Try to parse an IMEI from a buffer line (15 digits).
    fn try_parse_imei(&self, buf: &[u8]) -> Option<String<16>> {
        // Find a run of 15 digits
        let mut start = None;
        let mut count = 0u8;

        for (i, &ch) in buf.iter().enumerate() {
            if ch >= b'0' && ch <= b'9' {
                if start.is_none() {
                    start = Some(i);
                }
                count += 1;
                if count == 15 {
                    let s = start.unwrap();
                    let mut imei = String::new();
                    for &digit in &buf[s..s + 15] {
                        let _ = imei.push(digit as char);
                    }
                    return Some(imei);
                }
            } else {
                start = None;
                count = 0;
            }
        }
        None
    }

    /// Parse +COPS response to extract operator name.
    fn parse_cops(&mut self, data: &[u8]) {
        // Format: <mode>,<format>,"<operator_name>"
        // Find opening quote
        let mut i = 0;
        while i < data.len() && data[i] != b'"' {
            i += 1;
        }
        i += 1; // skip opening quote

        let start = i;
        while i < data.len() && data[i] != b'"' {
            i += 1;
        }

        if i > start {
            self.info.operator.clear();
            for &ch in &data[start..i] {
                let _ = self.info.operator.push(ch as char);
            }
        }
    }

    /// Parse +IPADDR response to extract IP address.
    fn parse_ipaddr(&mut self, data: &[u8]) {
        let mut octets = [0u8; 4];
        let mut octet_idx = 0;
        let mut current: u16 = 0;

        for &ch in data {
            if ch == b'.' || ch == b'\r' || ch == b'\n' {
                if octet_idx < 4 {
                    octets[octet_idx] = current as u8;
                    octet_idx += 1;
                }
                current = 0;
                if ch != b'.' {
                    break;
                }
            } else if ch >= b'0' && ch <= b'9' {
                current = current * 10 + (ch - b'0') as u16;
            }
        }
        // Handle last octet if no trailing newline
        if octet_idx == 3 {
            octets[3] = current as u8;
            octet_idx = 4;
        }

        if octet_idx == 4 {
            self.info.ip_addr = octets;
        }
    }

    // =========================================================================
    // UART helpers
    // =========================================================================

    /// Drain all available bytes from the UART into the receive buffer.
    fn drain_uart(&mut self) {
        loop {
            match self.uart.read() {
                Ok(byte) => {
                    self.on_byte_received(byte);
                }
                Err(nb::Error::WouldBlock) => break,
                Err(nb::Error::Other(_)) => break,
            }
        }
    }

    /// Write a single byte to the UART.
    fn uart_write_byte(&mut self, byte: u8) -> Result<()> {
        // Blocking write with timeout
        for _ in 0..10_000 {
            match self.uart.write(byte) {
                Ok(()) => return Ok(()),
                Err(nb::Error::WouldBlock) => {
                    core::hint::spin_loop();
                    continue;
                }
                Err(nb::Error::Other(_)) => {
                    return Err(Error::SpiBusError);
                }
            }
        }
        Err(Error::SpiTimeout)
    }

    /// Check if the STATUS pin is high (module is powered on).
    fn is_status_high(&self) -> bool {
        self.status.is_high().unwrap_or(false)
    }

    // =========================================================================
    // TX buffer helpers (for building AT commands with parameters)
    // =========================================================================

    /// Small internal transmit staging buffer.
    /// We reuse the lower portion of rx_buf temporarily during TX construction.
    /// This avoids an extra buffer allocation.

    fn tx_buf_clear(&mut self) {
        // No-op: we write directly to UART in tx_flush
        self.rx_pos = 0;
    }

    fn tx_write(&mut self, data: &[u8]) {
        let space = RX_BUF_SIZE - self.rx_pos;
        let len = data.len().min(space);
        self.rx_buf[self.rx_pos..self.rx_pos + len].copy_from_slice(&data[..len]);
        self.rx_pos += len;
    }

    fn tx_write_u8(&mut self, val: u8) {
        let mut buf = [0u8; 3];
        let len = fmt_u8(val, &mut buf);
        self.tx_write(&buf[..len]);
    }

    fn tx_write_u16(&mut self, val: u16) {
        let mut buf = [0u8; 5];
        let len = fmt_u16(val, &mut buf);
        self.tx_write(&buf[..len]);
    }

    fn tx_flush(&mut self) -> Result<()> {
        for i in 0..self.rx_pos {
            self.uart_write_byte(self.rx_buf[i])?;
        }
        self.rx_pos = 0;
        self.awaiting_response = true;
        self.last_response_ok = false;
        Ok(())
    }
}

// =============================================================================
// Utility functions
// =============================================================================

/// Busy-wait delay (placeholder for timer-based delay in production).
#[inline(always)]
fn spin_delay(cycles: u32) {
    for _ in 0..cycles {
        core::hint::spin_loop();
    }
}

/// Check if `haystack` contains `needle` as a subsequence.
fn contains_subsequence(haystack: &[u8], needle: &[u8]) -> bool {
    find_subsequence(haystack, needle).is_some()
}

/// Find the position of `needle` in `haystack`.
fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    if haystack.len() < needle.len() {
        return None;
    }
    for i in 0..=(haystack.len() - needle.len()) {
        if haystack[i..i + needle.len()] == *needle {
            return Some(i);
        }
    }
    None
}

/// Format a u8 as decimal ASCII into `buf`. Returns number of bytes written.
fn fmt_u8(mut val: u8, buf: &mut [u8; 3]) -> usize {
    if val == 0 {
        buf[0] = b'0';
        return 1;
    }
    let mut pos = 0;
    let mut tmp = [0u8; 3];
    while val > 0 {
        tmp[pos] = b'0' + (val % 10);
        val /= 10;
        pos += 1;
    }
    for i in 0..pos {
        buf[i] = tmp[pos - 1 - i];
    }
    pos
}

/// Format a u16 as decimal ASCII into `buf`. Returns number of bytes written.
fn fmt_u16(mut val: u16, buf: &mut [u8; 5]) -> usize {
    if val == 0 {
        buf[0] = b'0';
        return 1;
    }
    let mut pos = 0;
    let mut tmp = [0u8; 5];
    while val > 0 {
        tmp[pos] = b'0' + (val % 10) as u8;
        val /= 10;
        pos += 1;
    }
    for i in 0..pos {
        buf[i] = tmp[pos - 1 - i];
    }
    pos
}
