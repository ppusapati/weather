/// SIM7600E-H 4G LTE cellular modem driver via UART4.
///
/// The SIM7600E-H is a multi-band LTE-FDD/LTE-TDD/HSPA+/GSM/GPRS/EDGE
/// module supporting data, SMS, and voice. Communication is via AT commands
/// over UART at 115200 baud. The module requires a specific power-on
/// sequence using the PWRKEY pin.
///
/// # Wiring (UART4)
///
/// | STM32F407 Pin | SIM7600 Pin | Function                     |
/// |---------------|-------------|------------------------------|
/// | PC10          | RXD         | UART4 TX -> modem RX         |
/// | PC11          | TXD         | UART4 RX <- modem TX         |
/// | PD5           | PWRKEY      | Power key (active low pulse)  |
/// | PD6           | STATUS      | Module status (input, high=on)|
/// | PD7           | RESET       | Hardware reset (active low)   |
/// | PE2           | DTR         | Data Terminal Ready            |
///
/// # Power-On Sequence
///
/// 1. Drive PWRKEY low for >= 1.5s
/// 2. Release PWRKEY (high)
/// 3. Wait for STATUS pin to go high (up to 10s)
/// 4. Wait ~3s for UART to become ready
/// 5. Send AT commands to initialize

use crate::error::{Error, Result};
use heapless::String;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Pin assignments (UART4 on STM32F407)
// ---------------------------------------------------------------------------

/// Pin constants referenced from `crate::drivers::stm32f407::pins`.
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

// ---------------------------------------------------------------------------
// UART configuration
// ---------------------------------------------------------------------------

/// Default baud rate for AT command interface (bps).
pub const UART_BAUD_RATE: u32 = 115_200;

// ---------------------------------------------------------------------------
// Timing constants
// ---------------------------------------------------------------------------

/// PWRKEY pulse duration for power toggle (~1.5s in spin cycles).
const PWRKEY_PULSE_CYCLES: u32 = 15_000_000;
/// Maximum time to wait for STATUS high after power-on (ms).
const POWER_ON_TIMEOUT_MS: u32 = 10_000;
/// Delay after STATUS goes high before sending AT commands (ms).
const POST_BOOT_DELAY_MS: u32 = 3_000;
/// Default AT command response timeout (ms).
const AT_DEFAULT_TIMEOUT_MS: u32 = 5_000;
/// Extended timeout for network/data operations (ms).
const AT_NETWORK_TIMEOUT_MS: u32 = 30_000;
/// Timeout for registration polling (ms).
const REGISTRATION_TIMEOUT_MS: u32 = 120_000;
/// Interval between periodic signal quality checks (ms).
const SIGNAL_POLL_INTERVAL_MS: u32 = 60_000;

// ---------------------------------------------------------------------------
// Buffer sizes
// ---------------------------------------------------------------------------

/// Receive buffer size for AT responses and URCs.
const RX_BUF_SIZE: usize = 512;
/// Transmit staging buffer size for building AT commands.
const TX_BUF_SIZE: usize = 256;

// ---------------------------------------------------------------------------
// AT command byte strings
// ---------------------------------------------------------------------------

const AT_SYNC: &[u8] = b"AT\r\n";
const AT_ECHO_OFF: &[u8] = b"ATE0\r\n";
const AT_CPIN: &[u8] = b"AT+CPIN?\r\n";
const AT_GSN: &[u8] = b"AT+GSN\r\n";
const AT_CICCID: &[u8] = b"AT+CICCID\r\n";
const AT_CSQ: &[u8] = b"AT+CSQ\r\n";
const AT_CESQ: &[u8] = b"AT+CESQ\r\n";
const AT_CREG: &[u8] = b"AT+CREG?\r\n";
const AT_CEREG: &[u8] = b"AT+CEREG?\r\n";
const AT_CREG_URC: &[u8] = b"AT+CREG=1\r\n";
const AT_CEREG_URC: &[u8] = b"AT+CEREG=1\r\n";
const AT_COPS: &[u8] = b"AT+COPS?\r\n";
const AT_CPSI: &[u8] = b"AT+CPSI?\r\n";
const AT_CGDCONT: &[u8] = b"AT+CGDCONT=";
const AT_CGACT_ON: &[u8] = b"AT+CGACT=1,1\r\n";
const AT_CGACT_OFF: &[u8] = b"AT+CGACT=0,1\r\n";
const AT_CIPOPEN: &[u8] = b"AT+CIPOPEN=";
const AT_CIPSEND: &[u8] = b"AT+CIPSEND=";
const AT_CIPCLOSE: &[u8] = b"AT+CIPCLOSE=";
const AT_NETOPEN: &[u8] = b"AT+NETOPEN\r\n";
const AT_NETCLOSE: &[u8] = b"AT+NETCLOSE\r\n";
const AT_IPADDR: &[u8] = b"AT+IPADDR\r\n";
const AT_CPOF: &[u8] = b"AT+CPOF\r\n";
const AT_CMEE: &[u8] = b"AT+CMEE=2\r\n";
const AT_CFUN: &[u8] = b"AT+CFUN=1\r\n";

/// Expected "OK" response.
const RESP_OK: &[u8] = b"OK";
/// Expected "ERROR" response.
const RESP_ERROR: &[u8] = b"ERROR";
/// SIM ready indicator.
const RESP_CPIN_READY: &[u8] = b"+CPIN: READY";

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// SIM7600 module state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CellularState {
    /// Module is powered off.
    PowerOff,
    /// Module is booting / running init sequence.
    Initializing,
    /// SIM card detected and ready.
    SimReady,
    /// Searching for a network.
    Searching,
    /// Registered on the cellular network.
    Registered,
    /// IP data connection is active.
    DataConnected,
    /// Unrecoverable error state.
    Error,
}

/// Network registration status from AT+CREG / AT+CEREG.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistrationStatus {
    NotRegistered,
    RegisteredHome,
    Searching,
    Denied,
    Unknown,
    RegisteredRoaming,
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

/// Signal quality information.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SignalQuality {
    /// RSSI in dBm (e.g. -85). 0 means unknown.
    pub rssi_dbm: i16,
    /// Bit error rate (0-7, 99 = unknown).
    pub ber: u8,
    /// LTE Reference Signal Received Power in dBm.
    pub rsrp: i16,
    /// LTE Reference Signal Received Quality in dB.
    pub rsrq: i16,
}

impl Default for SignalQuality {
    fn default() -> Self {
        Self {
            rssi_dbm: 0,
            ber: 99,
            rsrp: 0,
            rsrq: 0,
        }
    }
}

/// Runtime information about the cellular module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellularInfo {
    /// Current module state.
    pub state: CellularState,
    /// Signal quality metrics.
    pub signal: SignalQuality,
    /// Operator name (e.g. "Airtel", "Jio").
    pub operator: String<32>,
    /// IMEI (International Mobile Equipment Identity, 15 digits).
    pub imei: String<20>,
    /// ICCID (SIM card serial number, up to 22 digits).
    pub iccid: String<24>,
    /// Assigned IP address from PDP context.
    pub ip_addr: [u8; 4],
    /// Current radio access technology: "4G", "3G", or "2G".
    pub network_type: String<8>,
}

impl Default for CellularInfo {
    fn default() -> Self {
        Self {
            state: CellularState::PowerOff,
            signal: SignalQuality::default(),
            operator: String::new(),
            imei: String::new(),
            iccid: String::new(),
            ip_addr: [0; 4],
            network_type: String::new(),
        }
    }
}

/// SIM7600E-H 4G LTE cellular modem driver.
///
/// Manages the modem power state, AT command sequencing, network
/// registration, and TCP/IP data connections over UART4.
pub struct Sim7600<UART, PWRKEY, STATUS, RST, DTR> {
    uart: UART,
    pwrkey: PWRKEY,
    status: STATUS,
    rst: RST,
    dtr: DTR,
    state: CellularState,
    info: CellularInfo,
    rx_buf: [u8; RX_BUF_SIZE],
    rx_pos: usize,
    tx_buf: [u8; TX_BUF_SIZE],
    apn: String<64>,
    socket_connected: bool,
    /// Timestamp of the last AT command sent (ms).
    last_at_ms: u32,
    /// Current AT response timeout (ms).
    at_timeout_ms: u32,
    /// Whether we are waiting for an AT response.
    awaiting_response: bool,
    /// Whether the last response was "OK".
    last_response_ok: bool,
    /// Timestamp of the last signal quality poll (ms).
    last_signal_poll_ms: u32,
}

impl<UART, PWRKEY, STATUS, RST, DTR> Sim7600<UART, PWRKEY, STATUS, RST, DTR>
where
    UART: embedded_hal_nb::serial::Write<u8> + embedded_hal_nb::serial::Read<u8>,
    PWRKEY: embedded_hal::digital::OutputPin,
    STATUS: embedded_hal::digital::InputPin,
    RST: embedded_hal::digital::OutputPin,
    DTR: embedded_hal::digital::OutputPin,
{
    // =======================================================================
    // Construction
    // =======================================================================

    /// Create a new SIM7600 driver instance.
    ///
    /// The modem starts in `PowerOff` state. Call [`power_on`] followed by
    /// [`init`] to bring it online.
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
            tx_buf: [0u8; TX_BUF_SIZE],
            apn: String::new(),
            socket_connected: false,
            last_at_ms: 0,
            at_timeout_ms: AT_DEFAULT_TIMEOUT_MS,
            awaiting_response: false,
            last_response_ok: false,
            last_signal_poll_ms: 0,
        }
    }

    // =======================================================================
    // Power management
    // =======================================================================

    /// Power on the SIM7600 module.
    ///
    /// Drives PWRKEY low for ~1.5s, then waits for the STATUS pin to go
    /// high (indicating the module has booted). Returns an error if the
    /// module does not respond within 10 seconds.
    pub fn power_on(&mut self) -> Result<()> {
        // Ensure DTR is low (active — keeps modem awake)
        self.dtr.set_low().map_err(|_| Error::CellularInitFailed)?;

        // Ensure RST is not asserted
        self.rst.set_high().map_err(|_| Error::CellularInitFailed)?;

        // Pulse PWRKEY low for ~1.5s to toggle power on
        self.pwrkey.set_low().map_err(|_| Error::CellularInitFailed)?;
        spin_delay(PWRKEY_PULSE_CYCLES);
        self.pwrkey.set_high().map_err(|_| Error::CellularInitFailed)?;

        // Wait for STATUS pin to go high (module booted)
        let mut waited: u32 = 0;
        let step_cycles: u32 = 100_000; // ~10ms per step
        let step_ms: u32 = 10;
        while !self.is_status_high() {
            spin_delay(step_cycles);
            waited += step_ms;
            if waited > POWER_ON_TIMEOUT_MS {
                log::error!("SIM7600: power-on timeout, STATUS not high");
                self.state = CellularState::Error;
                self.info.state = CellularState::Error;
                return Err(Error::CellularTimeout);
            }
        }

        // Allow UART to stabilise after boot
        spin_delay(POST_BOOT_DELAY_MS * 10_000);

        self.state = CellularState::Initializing;
        self.info.state = CellularState::Initializing;
        log::info!("SIM7600: powered on, STATUS high after {}ms", waited);
        Ok(())
    }

    /// Power off the SIM7600 module.
    ///
    /// Attempts a graceful shutdown via `AT+CPOF`. If the module is still
    /// running after the command, falls back to a PWRKEY pulse.
    pub fn power_off(&mut self) -> Result<()> {
        // Try graceful shutdown
        let _ = self.send_at(AT_CPOF);
        spin_delay(3_000_000);

        // If still on, force off via PWRKEY pulse
        if self.is_status_high() {
            self.pwrkey.set_low().map_err(|_| Error::CellularInitFailed)?;
            spin_delay(PWRKEY_PULSE_CYCLES);
            self.pwrkey.set_high().map_err(|_| Error::CellularInitFailed)?;
            spin_delay(5_000_000);
        }

        self.state = CellularState::PowerOff;
        self.info.state = CellularState::PowerOff;
        self.awaiting_response = false;
        self.socket_connected = false;
        log::info!("SIM7600: powered off");
        Ok(())
    }

    // =======================================================================
    // Initialisation
    // =======================================================================

    /// Full initialisation sequence.
    ///
    /// Sends the startup AT command sequence: sync, disable echo, enable
    /// verbose errors, check SIM status, read IMEI, and read ICCID.
    /// The module must be powered on before calling this.
    pub fn init(&mut self) -> Result<()> {
        if self.state == CellularState::PowerOff {
            return Err(Error::CellularInitFailed);
        }

        self.state = CellularState::Initializing;
        self.info.state = CellularState::Initializing;

        // AT sync — retry up to 5 times
        let mut synced = false;
        for _ in 0..5 {
            if self.send_at_ok(AT_SYNC, 2_000).is_ok() {
                synced = true;
                break;
            }
            spin_delay(500_000);
        }
        if !synced {
            log::error!("SIM7600: AT sync failed");
            self.state = CellularState::Error;
            self.info.state = CellularState::Error;
            return Err(Error::CellularInitFailed);
        }

        // Disable echo
        self.send_at_ok(AT_ECHO_OFF, AT_DEFAULT_TIMEOUT_MS)?;

        // Enable verbose error reporting
        self.send_at_ok(AT_CMEE, AT_DEFAULT_TIMEOUT_MS)?;

        // Check SIM card
        let cpin_resp = self.send_at_response(AT_CPIN, AT_DEFAULT_TIMEOUT_MS)?;
        if !contains_subsequence(&cpin_resp, RESP_CPIN_READY) {
            log::error!("SIM7600: SIM card not ready");
            self.state = CellularState::Error;
            self.info.state = CellularState::Error;
            return Err(Error::CellularSimError);
        }
        self.state = CellularState::SimReady;
        self.info.state = CellularState::SimReady;
        log::info!("SIM7600: SIM card ready");

        // Read IMEI (AT+GSN)
        let imei_resp = self.send_at_response(AT_GSN, AT_DEFAULT_TIMEOUT_MS)?;
        if let Some(imei) = parse_digit_string(&imei_resp, 15) {
            self.info.imei = imei;
            log::info!("SIM7600: IMEI={}", self.info.imei.as_str());
        }

        // Read ICCID (AT+CICCID)
        let iccid_resp = self.send_at_response(AT_CICCID, AT_DEFAULT_TIMEOUT_MS)?;
        if let Some(pos) = find_subsequence(&iccid_resp, b"+ICCID: ") {
            let start = pos + 8;
            if let Some(iccid) = parse_digit_string(&iccid_resp[start..], 19) {
                self.info.iccid = iccid;
            } else if let Some(iccid) = parse_digit_string(&iccid_resp[start..], 20) {
                self.info.iccid = iccid;
            }
            log::info!("SIM7600: ICCID={}", self.info.iccid.as_str());
        }

        // Enable network registration URCs
        let _ = self.send_at_ok(AT_CREG_URC, AT_DEFAULT_TIMEOUT_MS);
        let _ = self.send_at_ok(AT_CEREG_URC, AT_DEFAULT_TIMEOUT_MS);

        // Set full functionality
        let _ = self.send_at_ok(AT_CFUN, AT_NETWORK_TIMEOUT_MS);

        log::info!("SIM7600: initialisation complete");
        Ok(())
    }

    // =======================================================================
    // APN configuration
    // =======================================================================

    /// Configure the APN for data connections.
    ///
    /// # Example
    /// ```ignore
    /// modem.set_apn("internet")?;
    /// ```
    pub fn set_apn(&mut self, apn: &str) -> Result<()> {
        self.apn.clear();
        self.apn
            .push_str(apn)
            .map_err(|_| Error::InvalidConfig)?;

        // AT+CGDCONT=1,"IP","<apn>"
        let mut cmd_buf = [0u8; 128];
        let mut pos = 0;
        pos += copy_to_buf(&mut cmd_buf, pos, AT_CGDCONT);
        pos += copy_to_buf(&mut cmd_buf, pos, b"1,\"IP\",\"");
        pos += copy_to_buf(&mut cmd_buf, pos, apn.as_bytes());
        pos += copy_to_buf(&mut cmd_buf, pos, b"\"\r\n");

        self.send_at_ok(&cmd_buf[..pos], AT_DEFAULT_TIMEOUT_MS)?;
        log::info!("SIM7600: APN set to '{}'", apn);
        Ok(())
    }

    // =======================================================================
    // Network registration
    // =======================================================================

    /// Wait for network registration.
    ///
    /// Polls AT+CREG? and AT+CEREG? until the module registers on a
    /// network (home or roaming). Times out after ~120 seconds.
    pub fn register_network(&mut self) -> Result<()> {
        self.state = CellularState::Searching;
        self.info.state = CellularState::Searching;

        let mut elapsed: u32 = 0;
        let poll_interval: u32 = 3_000;

        loop {
            // Query CS registration
            if let Ok(resp) = self.send_at_response(AT_CREG, AT_DEFAULT_TIMEOUT_MS) {
                if let Some(reg) = parse_registration_response(&resp) {
                    match reg {
                        RegistrationStatus::RegisteredHome
                        | RegistrationStatus::RegisteredRoaming => {
                            self.state = CellularState::Registered;
                            self.info.state = CellularState::Registered;
                            self.query_operator();
                            self.refresh_signal_quality();
                            self.query_network_type();
                            log::info!("SIM7600: registered on network (CS)");
                            return Ok(());
                        }
                        RegistrationStatus::Denied => {
                            log::error!("SIM7600: registration denied");
                            self.state = CellularState::Error;
                            self.info.state = CellularState::Error;
                            return Err(Error::CellularRegistrationFailed);
                        }
                        _ => {}
                    }
                }
            }

            // Query EPS (LTE) registration
            if let Ok(resp) = self.send_at_response(AT_CEREG, AT_DEFAULT_TIMEOUT_MS) {
                if let Some(reg) = parse_registration_response(&resp) {
                    match reg {
                        RegistrationStatus::RegisteredHome
                        | RegistrationStatus::RegisteredRoaming => {
                            self.state = CellularState::Registered;
                            self.info.state = CellularState::Registered;
                            self.query_operator();
                            self.refresh_signal_quality();
                            self.query_network_type();
                            log::info!("SIM7600: registered on network (EPS/LTE)");
                            return Ok(());
                        }
                        RegistrationStatus::Denied => {
                            log::error!("SIM7600: EPS registration denied");
                            self.state = CellularState::Error;
                            self.info.state = CellularState::Error;
                            return Err(Error::CellularRegistrationFailed);
                        }
                        _ => {}
                    }
                }
            }

            elapsed += poll_interval;
            if elapsed > REGISTRATION_TIMEOUT_MS {
                log::error!("SIM7600: registration timeout after {}ms", elapsed);
                self.state = CellularState::Error;
                self.info.state = CellularState::Error;
                return Err(Error::CellularRegistrationFailed);
            }

            spin_delay(poll_interval * 10_000);
        }
    }

    // =======================================================================
    // Data connection
    // =======================================================================

    /// Establish a data (IP) connection.
    ///
    /// Activates the PDP context and opens the network stack. The APN
    /// must be configured via [`set_apn`] before calling this.
    pub fn open_data_connection(&mut self) -> Result<()> {
        if self.state != CellularState::Registered {
            return Err(Error::CellularDataConnectionFailed);
        }

        // Activate PDP context
        self.send_at_ok(AT_CGACT_ON, AT_NETWORK_TIMEOUT_MS)
            .map_err(|_| Error::CellularDataConnectionFailed)?;

        // Open network stack
        self.send_at_ok(AT_NETOPEN, AT_NETWORK_TIMEOUT_MS)
            .map_err(|_| Error::CellularDataConnectionFailed)?;

        // Query assigned IP address
        if let Ok(resp) = self.send_at_response(AT_IPADDR, AT_DEFAULT_TIMEOUT_MS) {
            self.parse_ipaddr(&resp);
        }

        self.state = CellularState::DataConnected;
        self.info.state = CellularState::DataConnected;
        log::info!(
            "SIM7600: data connected, IP={}.{}.{}.{}",
            self.info.ip_addr[0],
            self.info.ip_addr[1],
            self.info.ip_addr[2],
            self.info.ip_addr[3],
        );
        Ok(())
    }

    /// Tear down the data connection.
    ///
    /// Closes the network stack and deactivates the PDP context.
    pub fn close_data_connection(&mut self) -> Result<()> {
        // Close any open socket first
        if self.socket_connected {
            let _ = self.close_tcp();
        }

        let _ = self.send_at_ok(AT_NETCLOSE, AT_NETWORK_TIMEOUT_MS);
        let _ = self.send_at_ok(AT_CGACT_OFF, AT_DEFAULT_TIMEOUT_MS);

        if self.state == CellularState::DataConnected {
            self.state = CellularState::Registered;
            self.info.state = CellularState::Registered;
        }
        self.info.ip_addr = [0; 4];
        log::info!("SIM7600: data connection closed");
        Ok(())
    }

    // =======================================================================
    // Signal quality
    // =======================================================================

    /// Query current signal quality.
    ///
    /// Sends AT+CSQ for RSSI/BER and AT+CESQ for LTE-specific RSRP/RSRQ
    /// metrics. Returns the cached [`SignalQuality`] struct.
    pub fn signal_quality(&mut self) -> Result<SignalQuality> {
        self.refresh_signal_quality();
        if self.info.signal.rssi_dbm == 0 && self.info.signal.ber == 99 {
            return Err(Error::CellularNoSignal);
        }
        Ok(self.info.signal)
    }

    // =======================================================================
    // TCP operations
    // =======================================================================

    /// Open a TCP connection and send data.
    ///
    /// # Arguments
    /// - `host`: remote hostname or IP address as a string
    /// - `port`: remote TCP port number
    /// - `data`: payload to send
    pub fn send_tcp(&mut self, host: &str, port: u16, data: &[u8]) -> Result<()> {
        if self.state != CellularState::DataConnected {
            return Err(Error::CellularTxFailed);
        }
        if data.is_empty() || data.len() > 1460 {
            return Err(Error::InvalidConfig);
        }

        // AT+CIPOPEN=0,"TCP","<host>",<port>
        let mut cmd = [0u8; 160];
        let mut pos = 0;
        pos += copy_to_buf(&mut cmd, pos, AT_CIPOPEN);
        pos += copy_to_buf(&mut cmd, pos, b"0,\"TCP\",\"");
        pos += copy_to_buf(&mut cmd, pos, host.as_bytes());
        pos += copy_to_buf(&mut cmd, pos, b"\",");
        pos += copy_u16_ascii(&mut cmd, pos, port);
        pos += copy_to_buf(&mut cmd, pos, b"\r\n");

        self.send_at_ok(&cmd[..pos], AT_NETWORK_TIMEOUT_MS)
            .map_err(|_| Error::CellularTxFailed)?;
        self.socket_connected = true;

        // AT+CIPSEND=0,<length>
        let mut send_cmd = [0u8; 32];
        let mut sp = 0;
        sp += copy_to_buf(&mut send_cmd, sp, AT_CIPSEND);
        sp += copy_to_buf(&mut send_cmd, sp, b"0,");
        sp += copy_u16_ascii(&mut send_cmd, sp, data.len() as u16);
        sp += copy_to_buf(&mut send_cmd, sp, b"\r\n");

        self.send_at(&send_cmd[..sp])?;

        // Wait for '>' prompt
        spin_delay(100_000);

        // Send the payload
        for &byte in data {
            self.uart_write_byte(byte)?;
        }

        // Wait for send confirmation
        self.wait_response(AT_NETWORK_TIMEOUT_MS)?;
        if !self.last_response_ok {
            return Err(Error::CellularTxFailed);
        }

        log::debug!("SIM7600: sent {} bytes to {}:{}", data.len(), host, port);
        Ok(())
    }

    /// Read data from the TCP receive buffer.
    ///
    /// Returns the number of bytes copied into `buf`. Returns 0 if no
    /// data is available.
    pub fn recv_tcp(&mut self, buf: &mut [u8]) -> Result<usize> {
        // Drain UART to pick up any pending data
        self.drain_uart();

        let available = self.rx_pos.min(buf.len());
        if available == 0 {
            return Ok(0);
        }

        buf[..available].copy_from_slice(&self.rx_buf[..available]);

        // Shift remaining data forward
        let remaining = self.rx_pos - available;
        if remaining > 0 {
            self.rx_buf.copy_within(available..self.rx_pos, 0);
        }
        self.rx_pos = remaining;

        Ok(available)
    }

    /// Close the current TCP connection.
    pub fn close_tcp(&mut self) -> Result<()> {
        if !self.socket_connected {
            return Ok(());
        }

        // AT+CIPCLOSE=0
        self.send_at_ok(b"AT+CIPCLOSE=0\r\n", AT_DEFAULT_TIMEOUT_MS)
            .map_err(|_| Error::CellularTxFailed)?;

        self.socket_connected = false;
        log::info!("SIM7600: TCP connection closed");
        Ok(())
    }

    // =======================================================================
    // Polling / main loop
    // =======================================================================

    /// Periodic poll function. Call from the main loop.
    ///
    /// Drains the UART, processes URCs, checks registration state, and
    /// periodically refreshes signal quality. `now_ms` is the current
    /// monotonic time in milliseconds.
    pub fn poll(&mut self, now_ms: u32) {
        if self.state == CellularState::PowerOff || self.state == CellularState::Error {
            return;
        }

        // Drain incoming UART bytes and process URCs
        self.drain_uart();
        self.process_urcs();

        // Check for AT command timeout
        if self.awaiting_response {
            if now_ms.wrapping_sub(self.last_at_ms) > self.at_timeout_ms {
                log::warn!("SIM7600: AT command timeout");
                self.awaiting_response = false;
                self.last_response_ok = false;
                self.rx_pos = 0;
            }
            return;
        }

        // Periodic signal quality refresh
        if self.state == CellularState::DataConnected
            || self.state == CellularState::Registered
        {
            if now_ms.wrapping_sub(self.last_signal_poll_ms) > SIGNAL_POLL_INTERVAL_MS {
                self.refresh_signal_quality();
                self.last_signal_poll_ms = now_ms;
            }
        }
    }

    /// Returns `true` if the module has an active data connection.
    pub fn is_connected(&self) -> bool {
        self.state == CellularState::DataConnected
    }

    /// Query the current network type (2G / 3G / 4G) via AT+CPSI?.
    pub fn network_type(&mut self) -> Result<&str> {
        self.query_network_type();
        if self.info.network_type.is_empty() {
            return Err(Error::CellularNoSignal);
        }
        Ok(self.info.network_type.as_str())
    }

    /// Returns the current module state.
    pub fn state(&self) -> CellularState {
        self.state
    }

    /// Returns a reference to the module info.
    pub fn info(&self) -> &CellularInfo {
        &self.info
    }

    // =======================================================================
    // Private: AT command engine
    // =======================================================================

    /// Send a raw AT command (byte slice) over UART.
    fn send_at(&mut self, cmd: &[u8]) -> Result<()> {
        self.rx_pos = 0;
        self.awaiting_response = true;
        self.last_response_ok = false;

        for &byte in cmd {
            self.uart_write_byte(byte)?;
        }

        Ok(())
    }

    /// Wait for an AT response until OK, ERROR, or timeout.
    ///
    /// Blocks by spinning and draining the UART. On return,
    /// `self.last_response_ok` indicates whether OK was received.
    fn wait_response(&mut self, timeout_ms: u32) -> Result<()> {
        let step_cycles: u32 = 10_000; // ~1ms per step
        let mut waited: u32 = 0;

        while waited < timeout_ms {
            self.drain_uart();

            // Check for OK or ERROR in the buffer
            if contains_subsequence(&self.rx_buf[..self.rx_pos], RESP_OK) {
                self.last_response_ok = true;
                self.awaiting_response = false;
                return Ok(());
            }
            if contains_subsequence(&self.rx_buf[..self.rx_pos], RESP_ERROR) {
                self.last_response_ok = false;
                self.awaiting_response = false;
                return Ok(());
            }

            spin_delay(step_cycles);
            waited += 1;
        }

        self.awaiting_response = false;
        self.last_response_ok = false;
        Err(Error::CellularTimeout)
    }

    /// Send an AT command and expect an OK response within `timeout_ms`.
    fn send_at_ok(&mut self, cmd: &[u8], timeout_ms: u32) -> Result<()> {
        self.send_at(cmd)?;
        self.wait_response(timeout_ms)?;
        if self.last_response_ok {
            Ok(())
        } else {
            Err(Error::CellularInitFailed)
        }
    }

    /// Send an AT command and return the full response buffer contents.
    fn send_at_response(
        &mut self,
        cmd: &[u8],
        timeout_ms: u32,
    ) -> Result<[u8; RX_BUF_SIZE]> {
        self.send_at(cmd)?;
        let _ = self.wait_response(timeout_ms);

        let mut response = [0u8; RX_BUF_SIZE];
        let len = self.rx_pos.min(RX_BUF_SIZE);
        response[..len].copy_from_slice(&self.rx_buf[..len]);
        Ok(response)
    }

    // =======================================================================
    // Private: UART helpers
    // =======================================================================

    /// Write a single byte to the UART, blocking until the TX register
    /// is ready or a retry limit is reached.
    fn uart_write_byte(&mut self, byte: u8) -> Result<()> {
        for _ in 0..10_000 {
            match self.uart.write(byte) {
                Ok(()) => return Ok(()),
                Err(nb::Error::WouldBlock) => {
                    core::hint::spin_loop();
                    continue;
                }
                Err(nb::Error::Other(_)) => {
                    return Err(Error::CellularTxFailed);
                }
            }
        }
        Err(Error::CellularTimeout)
    }

    /// Drain all available bytes from the UART into the receive buffer.
    fn drain_uart(&mut self) {
        loop {
            match self.uart.read() {
                Ok(byte) => {
                    if self.rx_pos < RX_BUF_SIZE {
                        self.rx_buf[self.rx_pos] = byte;
                        self.rx_pos += 1;
                    }
                }
                Err(nb::Error::WouldBlock) => break,
                Err(nb::Error::Other(_)) => break,
            }
        }
    }

    /// Feed a single byte from the UART interrupt handler.
    pub fn on_byte_received(&mut self, byte: u8) {
        if self.rx_pos < RX_BUF_SIZE {
            self.rx_buf[self.rx_pos] = byte;
            self.rx_pos += 1;
        }
    }

    /// Check if the STATUS pin reads high (module is powered on).
    fn is_status_high(&self) -> bool {
        self.status.is_high().unwrap_or(false)
    }

    // =======================================================================
    // Private: URC processing
    // =======================================================================

    /// Scan the receive buffer for unsolicited result codes and update
    /// internal state accordingly.
    fn process_urcs(&mut self) {
        let buf = &self.rx_buf[..self.rx_pos];

        // +CREG: <stat> (URC — single digit, no <n> prefix)
        if let Some(pos) = find_subsequence(buf, b"+CREG: ") {
            let start = pos + 7;
            if start < buf.len() {
                let stat = buf[start].wrapping_sub(b'0');
                if stat <= 5 {
                    let reg = RegistrationStatus::from(stat);
                    match reg {
                        RegistrationStatus::RegisteredHome
                        | RegistrationStatus::RegisteredRoaming => {
                            if self.state == CellularState::Searching {
                                self.state = CellularState::Registered;
                                self.info.state = CellularState::Registered;
                                log::info!("SIM7600: URC registered (CREG={})", stat);
                            }
                        }
                        RegistrationStatus::NotRegistered | RegistrationStatus::Denied => {
                            if self.state == CellularState::Registered
                                || self.state == CellularState::DataConnected
                            {
                                self.state = CellularState::Searching;
                                self.info.state = CellularState::Searching;
                                self.socket_connected = false;
                                log::warn!("SIM7600: URC lost registration (CREG={})", stat);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // +CEREG: <stat>
        if let Some(pos) = find_subsequence(buf, b"+CEREG: ") {
            let start = pos + 8;
            if start < buf.len() {
                let stat = buf[start].wrapping_sub(b'0');
                if stat <= 5 {
                    let reg = RegistrationStatus::from(stat);
                    if (reg == RegistrationStatus::RegisteredHome
                        || reg == RegistrationStatus::RegisteredRoaming)
                        && self.state == CellularState::Searching
                    {
                        self.state = CellularState::Registered;
                        self.info.state = CellularState::Registered;
                        log::info!("SIM7600: URC LTE registered (CEREG={})", stat);
                    }
                }
            }
        }

        // +CIPCLOSE: <link>,<reason> — remote closed the connection
        if find_subsequence(buf, b"+CIPCLOSE:").is_some() {
            self.socket_connected = false;
            log::info!("SIM7600: URC socket closed by remote");
        }
    }

    // =======================================================================
    // Private: response parsers
    // =======================================================================

    /// Parse a +CSQ response to extract RSSI and BER.
    ///
    /// Response format: `+CSQ: <rssi>,<ber>`
    /// RSSI code 0-31 maps to -113..-51 dBm; 99 = unknown.
    fn parse_csq(&mut self, data: &[u8]) {
        if let Some(pos) = find_subsequence(data, b"+CSQ: ") {
            let start = pos + 6;
            let mut rssi: u8 = 0;
            let mut ber: u8 = 0;
            let mut i = start;
            let mut parsing_ber = false;

            while i < data.len() {
                let ch = data[i];
                if ch == b',' {
                    parsing_ber = true;
                    i += 1;
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
                i += 1;
            }

            self.info.signal.rssi_dbm = if rssi == 99 {
                0
            } else {
                -113 + 2 * rssi as i16
            };
            self.info.signal.ber = ber;
        }
    }

    /// Parse a +CESQ response to extract LTE RSRP and RSRQ.
    ///
    /// Response format: `+CESQ: <rxlev>,<ber>,<rscp>,<ecno>,<rsrq>,<rsrp>`
    fn parse_cesq(&mut self, data: &[u8]) {
        if let Some(pos) = find_subsequence(data, b"+CESQ: ") {
            let start = pos + 7;
            let mut fields = [0u8; 6];
            let mut field_idx = 0;
            let mut i = start;

            while i < data.len() && field_idx < 6 {
                let ch = data[i];
                if ch == b',' || ch == b'\r' || ch == b'\n' {
                    field_idx += 1;
                    if ch != b',' {
                        break;
                    }
                    i += 1;
                    continue;
                }
                if ch >= b'0' && ch <= b'9' {
                    fields[field_idx] =
                        fields[field_idx].wrapping_mul(10).wrapping_add(ch - b'0');
                }
                i += 1;
            }

            // RSRQ: index 4, maps to -19.5 + 0.5 * val dB (we store as integer)
            if fields[4] != 255 {
                self.info.signal.rsrq = -20 + fields[4] as i16;
            }
            // RSRP: index 5, maps to -140 + val dBm
            if fields[5] != 255 {
                self.info.signal.rsrp = -140 + fields[5] as i16;
            }
        }
    }

    /// Parse a +IPADDR response to extract the assigned IP address.
    fn parse_ipaddr(&mut self, data: &[u8]) {
        if let Some(pos) = find_subsequence(data, b"+IPADDR: ") {
            let start = pos + 9;
            let mut octets = [0u8; 4];
            let mut octet_idx = 0;
            let mut current: u16 = 0;

            for &ch in &data[start..] {
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

            if octet_idx == 3 {
                octets[3] = current as u8;
                octet_idx = 4;
            }

            if octet_idx == 4 {
                self.info.ip_addr = octets;
            }
        }
    }

    // =======================================================================
    // Private: query helpers
    // =======================================================================

    /// Query the operator name from AT+COPS?.
    fn query_operator(&mut self) {
        if let Ok(resp) = self.send_at_response(AT_COPS, AT_DEFAULT_TIMEOUT_MS) {
            if let Some(pos) = find_subsequence(&resp, b"+COPS: ") {
                let data = &resp[pos + 7..];
                // Format: <mode>,<format>,"<operator_name>"
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
        }
    }

    /// Refresh cached signal quality metrics.
    fn refresh_signal_quality(&mut self) {
        // Basic RSSI/BER
        if let Ok(resp) = self.send_at_response(AT_CSQ, AT_DEFAULT_TIMEOUT_MS) {
            self.parse_csq(&resp);
        }

        // LTE-specific RSRP/RSRQ
        if let Ok(resp) = self.send_at_response(AT_CESQ, AT_DEFAULT_TIMEOUT_MS) {
            self.parse_cesq(&resp);
        }
    }

    /// Query the current network type via AT+CPSI?.
    ///
    /// Parses the system info response to determine if the module is
    /// connected via LTE (4G), WCDMA/HSDPA (3G), or GSM (2G).
    fn query_network_type(&mut self) {
        if let Ok(resp) = self.send_at_response(AT_CPSI, AT_DEFAULT_TIMEOUT_MS) {
            self.info.network_type.clear();
            if contains_subsequence(&resp, b"LTE") {
                let _ = self.info.network_type.push_str("4G");
            } else if contains_subsequence(&resp, b"WCDMA")
                || contains_subsequence(&resp, b"HSDPA")
                || contains_subsequence(&resp, b"HSPA")
            {
                let _ = self.info.network_type.push_str("3G");
            } else if contains_subsequence(&resp, b"GSM")
                || contains_subsequence(&resp, b"EDGE")
                || contains_subsequence(&resp, b"GPRS")
            {
                let _ = self.info.network_type.push_str("2G");
            }
        }
    }
}

// ===========================================================================
// Free-standing utility functions
// ===========================================================================

/// Busy-wait delay (cycle count, not calibrated — placeholder for
/// timer-based delay in production firmware).
#[inline(always)]
fn spin_delay(cycles: u32) {
    for _ in 0..cycles {
        core::hint::spin_loop();
    }
}

/// Check if `haystack` contains `needle` as a contiguous subsequence.
fn contains_subsequence(haystack: &[u8], needle: &[u8]) -> bool {
    find_subsequence(haystack, needle).is_some()
}

/// Find the byte offset of `needle` within `haystack`, or `None`.
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

/// Parse a registration response (`+CREG:` or `+CEREG:`) to extract the
/// status digit.  Handles both query form `<n>,<stat>` and URC form `<stat>`.
fn parse_registration_response(buf: &[u8]) -> Option<RegistrationStatus> {
    // Look for "+CREG: " or "+CEREG: "
    let start = if let Some(pos) = find_subsequence(buf, b"+CREG: ") {
        pos + 7
    } else if let Some(pos) = find_subsequence(buf, b"+CEREG: ") {
        pos + 8
    } else {
        return None;
    };

    // Skip <n>, if present (find first comma after the prefix)
    let data = &buf[start..];
    let mut i = 0;
    let mut found_comma = false;
    while i < data.len() && data[i] != b'\r' && data[i] != b'\n' {
        if data[i] == b',' {
            found_comma = true;
            i += 1;
            break;
        }
        i += 1;
    }

    let stat_pos = if found_comma { i } else { 0 };
    if stat_pos < data.len() && data[stat_pos] >= b'0' && data[stat_pos] <= b'9' {
        Some(RegistrationStatus::from(data[stat_pos] - b'0'))
    } else {
        None
    }
}

/// Try to extract a string of exactly `min_len` consecutive digits from
/// a byte buffer. Returns a heapless `String<24>` on success.
fn parse_digit_string<const N: usize>(buf: &[u8], min_len: usize) -> Option<String<N>> {
    let mut start = None;
    let mut count: usize = 0;

    for (i, &ch) in buf.iter().enumerate() {
        if ch >= b'0' && ch <= b'9' {
            if start.is_none() {
                start = Some(i);
            }
            count += 1;
            if count >= min_len {
                let s = start.unwrap();
                let mut result: String<N> = String::new();
                for &digit in &buf[s..s + count] {
                    if result.push(digit as char).is_err() {
                        return None;
                    }
                }
                return Some(result);
            }
        } else {
            start = None;
            count = 0;
        }
    }
    None
}

/// Copy `src` bytes into `dst` starting at `offset`. Returns the number
/// of bytes written.
fn copy_to_buf(dst: &mut [u8], offset: usize, src: &[u8]) -> usize {
    let space = dst.len().saturating_sub(offset);
    let len = src.len().min(space);
    dst[offset..offset + len].copy_from_slice(&src[..len]);
    len
}

/// Write a `u16` value as decimal ASCII into `dst` at `offset`.
/// Returns the number of bytes written.
fn copy_u16_ascii(dst: &mut [u8], offset: usize, val: u16) -> usize {
    let mut buf = [0u8; 5];
    let len = fmt_u16(val, &mut buf);
    copy_to_buf(dst, offset, &buf[..len])
}

/// Format a `u16` as decimal ASCII into a fixed buffer. Returns the
/// number of bytes written (1-5).
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
