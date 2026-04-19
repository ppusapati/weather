/// SPI-based driver for the Microchip ATWINC1500 WiFi module.
///
/// Connected via SPI3 on the STM32F407:
///   PB3 = SPI3_SCK, PB4 = SPI3_MISO, PB5 = SPI3_MOSI
///   PE3 = CS (active low), PE4 = RESET (active low)
///   PE5 = IRQ (active low), PE6 = CHIP_EN (active high)
///
/// Implements register-level SPI access following the ATWINC1500
/// host interface protocol for WiFi connectivity and TCP sockets.

use crate::error::{Error, Result};
use heapless::String;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Pin assignments (SPI3 on STM32F407)
// ---------------------------------------------------------------------------

/// Pin constants for the ATWINC1500 WiFi module on SPI3.
pub mod pins {
    /// SPI3 clock — PB3.
    pub const SPI3_SCK: u8 = 19;
    /// SPI3 MISO — PB4.
    pub const SPI3_MISO: u8 = 20;
    /// SPI3 MOSI — PB5.
    pub const SPI3_MOSI: u8 = 21;
    /// Chip select (active low) — PE3.
    pub const CS: u8 = 67;
    /// Hardware reset (active low) — PE4.
    pub const RESET: u8 = 68;
    /// Interrupt request (active low) — PE5.
    pub const IRQ: u8 = 69;
    /// Chip enable (active high) — PE6.
    pub const CHIP_EN: u8 = 70;
}

// ---------------------------------------------------------------------------
// SPI clock
// ---------------------------------------------------------------------------

/// Maximum SPI clock frequency for the ATWINC1500 (Hz).
pub const SPI_MAX_CLOCK_HZ: u32 = 48_000_000;

// ---------------------------------------------------------------------------
// ATWINC1500 SPI protocol constants
// ---------------------------------------------------------------------------

/// SPI command: read register / memory.
const CMD_READ: u8 = 0xC0;
/// SPI command: write register / memory.
const CMD_WRITE: u8 = 0xC8;
/// SPI command: read block (DMA).
const CMD_DMA_READ: u8 = 0xC2;
/// SPI command: write block (DMA).
const CMD_DMA_WRITE: u8 = 0xCA;
/// SPI command: single-byte read (internal register).
const CMD_INTERNAL_READ: u8 = 0xC4;
/// SPI response byte indicating data is ready.
const SPI_RESP_READY: u8 = 0xF3;

// Register addresses
const REG_CHIP_ID: u32 = 0x1000;
const REG_EFUSE_MAC_0: u32 = 0x1040;
const REG_EFUSE_MAC_1: u32 = 0x1044;
const REG_WIFI_STATUS: u32 = 0x20000;
const REG_CONN_STATE: u32 = 0x20004;
const REG_WIFI_RSSI: u32 = 0x20008;
const REG_WIFI_CHANNEL: u32 = 0x2000C;
const REG_WIFI_IP_ADDR: u32 = 0x20010;
const REG_FW_VERSION: u32 = 0x20014;
const REG_SCAN_CTRL: u32 = 0x20100;
const REG_SCAN_RESULT_COUNT: u32 = 0x20104;
const REG_CONN_SSID: u32 = 0x20200;
const REG_CONN_PSK: u32 = 0x20240;
const REG_CONN_CTRL: u32 = 0x20280;
const REG_DISCONNECT_CTRL: u32 = 0x20284;
const REG_SOCKET_CMD: u32 = 0x30000;
const REG_SOCKET_STATUS: u32 = 0x30004;
const REG_SOCKET_TX_BUF: u32 = 0x30100;
const REG_SOCKET_RX_BUF: u32 = 0x31100;
const REG_SOCKET_TX_LEN: u32 = 0x30008;
const REG_SOCKET_RX_LEN: u32 = 0x3000C;

// Expected chip ID for ATWINC1500.
const ATWINC1500_CHIP_ID: u32 = 0x001503A0;

// Connection control commands
const CONN_CMD_CONNECT: u32 = 0x01;
const CONN_CMD_DISCONNECT: u32 = 0x02;

// Scan control commands
const SCAN_CMD_START: u32 = 0x01;

// Socket commands
const SOCK_CMD_OPEN_TCP: u8 = 0x10;
const SOCK_CMD_SEND: u8 = 0x20;
const SOCK_CMD_RECV: u8 = 0x30;
const SOCK_CMD_CLOSE: u8 = 0x40;

/// Maximum number of concurrent TCP sockets.
pub const MAX_SOCKETS: u8 = 7;

/// RSSI polling interval (milliseconds).
const RSSI_POLL_INTERVAL_MS: u64 = 5_000;

/// Maximum connection attempts before entering error state.
const MAX_CONNECT_ATTEMPTS: u8 = 5;

// ---------------------------------------------------------------------------
// WiFi module state
// ---------------------------------------------------------------------------

/// Operating state of the ATWINC1500 WiFi module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WifiModuleState {
    /// Module is powered off (CHIP_EN deasserted).
    PowerOff,
    /// Module is powering up and being configured.
    Initializing,
    /// Module is initialized and idle.
    Ready,
    /// Module is performing an AP scan.
    Scanning,
    /// Module is connecting to an access point.
    Connecting,
    /// Module is associated and has an IP address.
    Connected,
    /// Module encountered an unrecoverable error.
    Error,
}

// ---------------------------------------------------------------------------
// WiFi module info
// ---------------------------------------------------------------------------

/// Runtime information about the ATWINC1500 WiFi connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WifiModuleInfo {
    /// Current module state.
    pub state: WifiModuleState,
    /// Received signal strength indicator (dBm), 0 if unknown.
    pub rssi: i8,
    /// IPv4 address assigned by DHCP, all zeros if none.
    pub ip_addr: [u8; 4],
    /// MAC address read from eFuse.
    pub mac_addr: [u8; 6],
    /// SSID of the connected (or connecting) network.
    pub ssid: String<32>,
    /// WiFi channel number (1–14), 0 if unknown.
    pub channel: u8,
    /// ATWINC1500 firmware version string.
    pub firmware_version: String<16>,
}

impl Default for WifiModuleInfo {
    fn default() -> Self {
        Self {
            state: WifiModuleState::PowerOff,
            rssi: 0,
            ip_addr: [0u8; 4],
            mac_addr: [0u8; 6],
            ssid: String::new(),
            channel: 0,
            firmware_version: String::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Socket tracking
// ---------------------------------------------------------------------------

/// State of a single TCP socket slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SocketState {
    /// Slot is unused.
    Free,
    /// Socket is open and connected.
    Open,
}

// ---------------------------------------------------------------------------
// Driver struct
// ---------------------------------------------------------------------------

/// SPI-based driver for the Microchip ATWINC1500 WiFi module.
pub struct Atwinc1500<SPI, CS, RST, EN> {
    spi: SPI,
    cs: CS,
    rst: RST,
    chip_en: EN,
    state: WifiModuleState,
    info: WifiModuleInfo,
    connect_attempts: u8,
    last_rssi_update_ms: u64,
    tx_buf: [u8; 256],
    rx_buf: [u8; 256],
    sockets: [SocketState; MAX_SOCKETS as usize],
}

impl<SPI, CS, RST, EN> Atwinc1500<SPI, CS, RST, EN>
where
    SPI: embedded_hal::spi::SpiDevice,
    CS: embedded_hal::digital::OutputPin,
    RST: embedded_hal::digital::OutputPin,
    EN: embedded_hal::digital::OutputPin,
{
    /// Create a new ATWINC1500 driver instance.
    ///
    /// The module starts in `PowerOff` state. Call [`power_on`] followed by
    /// [`init`] to bring it up.
    pub fn new(spi: SPI, cs: CS, rst: RST, chip_en: EN) -> Self {
        Self {
            spi,
            cs,
            rst,
            chip_en,
            state: WifiModuleState::PowerOff,
            info: WifiModuleInfo::default(),
            connect_attempts: 0,
            last_rssi_update_ms: 0,
            tx_buf: [0u8; 256],
            rx_buf: [0u8; 256],
            sockets: [SocketState::Free; MAX_SOCKETS as usize],
        }
    }

    // -----------------------------------------------------------------------
    // Lifecycle
    // -----------------------------------------------------------------------

    /// Assert CHIP_EN (high) and deassert RESET (high) to power on the module.
    pub fn power_on(&mut self) {
        // CHIP_EN high to enable the module
        let _ = self.chip_en.set_high();
        spin_delay(10_000);

        // Deassert reset (active low, so set high)
        let _ = self.rst.set_high();
        spin_delay(50_000);

        self.state = WifiModuleState::Initializing;
        self.info.state = WifiModuleState::Initializing;
        log::info!("ATWINC1500: powered on");
    }

    /// Deassert CHIP_EN (low) to power off the module.
    pub fn power_off(&mut self) {
        let _ = self.rst.set_low();
        let _ = self.chip_en.set_low();
        self.state = WifiModuleState::PowerOff;
        self.info.state = WifiModuleState::PowerOff;
        self.info.rssi = 0;
        self.info.ip_addr = [0u8; 4];
        self.info.channel = 0;

        // Mark all sockets as free
        for s in self.sockets.iter_mut() {
            *s = SocketState::Free;
        }

        log::info!("ATWINC1500: powered off");
    }

    /// Initialize the ATWINC1500 after power-on.
    ///
    /// Performs a hardware reset sequence, reads the chip ID, reads the MAC
    /// address from eFuse, and verifies the on-chip firmware version.
    pub fn init(&mut self) -> Result<()> {
        if self.state == WifiModuleState::PowerOff {
            self.power_on();
        }

        self.state = WifiModuleState::Initializing;
        self.info.state = WifiModuleState::Initializing;

        // Hardware reset pulse (active low)
        self.rst.set_low().map_err(|_| Error::WifiConnectionFailed)?;
        spin_delay(10_000);
        self.rst.set_high().map_err(|_| Error::WifiConnectionFailed)?;
        spin_delay(100_000);

        // Deassert CS before first transaction
        let _ = self.cs.set_high();

        // Read and verify chip ID
        let chip_id = self.read_reg(REG_CHIP_ID)?;
        if chip_id != ATWINC1500_CHIP_ID {
            log::error!(
                "ATWINC1500: unexpected chip ID 0x{:08X} (expected 0x{:08X})",
                chip_id,
                ATWINC1500_CHIP_ID
            );
            self.state = WifiModuleState::Error;
            self.info.state = WifiModuleState::Error;
            return Err(Error::WifiConnectionFailed);
        }

        // Read MAC address from eFuse
        let mac_lo = self.read_reg(REG_EFUSE_MAC_0)?;
        let mac_hi = self.read_reg(REG_EFUSE_MAC_1)?;
        self.info.mac_addr[0] = ((mac_hi >> 8) & 0xFF) as u8;
        self.info.mac_addr[1] = (mac_hi & 0xFF) as u8;
        self.info.mac_addr[2] = ((mac_lo >> 24) & 0xFF) as u8;
        self.info.mac_addr[3] = ((mac_lo >> 16) & 0xFF) as u8;
        self.info.mac_addr[4] = ((mac_lo >> 8) & 0xFF) as u8;
        self.info.mac_addr[5] = (mac_lo & 0xFF) as u8;

        // Read firmware version
        let fw_raw = self.read_reg(REG_FW_VERSION)?;
        let major = ((fw_raw >> 24) & 0xFF) as u8;
        let minor = ((fw_raw >> 16) & 0xFF) as u8;
        let patch = (fw_raw & 0xFFFF) as u16;
        self.info.firmware_version.clear();
        let _ = core::fmt::write(
            &mut FmtWriter(&mut self.info.firmware_version),
            format_args!("{}.{}.{}", major, minor, patch),
        );

        self.state = WifiModuleState::Ready;
        self.info.state = WifiModuleState::Ready;

        log::info!(
            "ATWINC1500: initialized, chip_id=0x{:08X}, MAC={:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}, FW={}",
            chip_id,
            self.info.mac_addr[0], self.info.mac_addr[1], self.info.mac_addr[2],
            self.info.mac_addr[3], self.info.mac_addr[4], self.info.mac_addr[5],
            self.info.firmware_version.as_str(),
        );

        Ok(())
    }

    // -----------------------------------------------------------------------
    // WiFi operations
    // -----------------------------------------------------------------------

    /// Start an AP scan and return the number of networks found.
    pub fn scan(&mut self) -> Result<u8> {
        if self.state != WifiModuleState::Ready && self.state != WifiModuleState::Connected {
            return Err(Error::WifiConnectionFailed);
        }

        let prev_state = self.state;
        self.state = WifiModuleState::Scanning;
        self.info.state = WifiModuleState::Scanning;

        // Issue scan command
        self.write_reg(REG_SCAN_CTRL, SCAN_CMD_START)?;

        // Poll for scan completion (wait for SCAN_CTRL to clear)
        for _ in 0..200 {
            spin_delay(50_000);
            let status = self.read_reg(REG_SCAN_CTRL)?;
            if status == 0 {
                let count = self.read_reg(REG_SCAN_RESULT_COUNT)? as u8;
                self.state = prev_state;
                self.info.state = prev_state;
                log::info!("ATWINC1500: scan complete, {} networks found", count);
                return Ok(count);
            }
        }

        self.state = prev_state;
        self.info.state = prev_state;
        log::error!("ATWINC1500: scan timeout");
        Err(Error::WifiTimeout)
    }

    /// Connect to a WiFi access point with the given SSID and password.
    pub fn connect(&mut self, ssid: &str, password: &str) -> Result<()> {
        if self.state != WifiModuleState::Ready {
            return Err(Error::WifiConnectionFailed);
        }

        self.state = WifiModuleState::Connecting;
        self.info.state = WifiModuleState::Connecting;
        self.info.ssid.clear();
        let _ = self.info.ssid.push_str(ssid);
        self.connect_attempts = 0;

        // Write SSID to module
        let ssid_bytes = ssid.as_bytes();
        let mut ssid_buf = [0u8; 32];
        let len = ssid_bytes.len().min(32);
        ssid_buf[..len].copy_from_slice(&ssid_bytes[..len]);
        self.write_block(REG_CONN_SSID, &ssid_buf)?;

        // Write PSK to module
        let psk_bytes = password.as_bytes();
        let mut psk_buf = [0u8; 64];
        let psk_len = psk_bytes.len().min(64);
        psk_buf[..psk_len].copy_from_slice(&psk_bytes[..psk_len]);
        self.write_block(REG_CONN_PSK, &psk_buf)?;

        // Issue connect command
        self.write_reg(REG_CONN_CTRL, CONN_CMD_CONNECT)?;

        // Poll for connection result
        for _ in 0..300 {
            spin_delay(50_000);
            let conn_state = self.read_reg(REG_CONN_STATE)?;

            if conn_state == 1 {
                // Connected — read IP and channel
                let ip_raw = self.read_reg(REG_WIFI_IP_ADDR)?;
                self.info.ip_addr[0] = ((ip_raw >> 24) & 0xFF) as u8;
                self.info.ip_addr[1] = ((ip_raw >> 16) & 0xFF) as u8;
                self.info.ip_addr[2] = ((ip_raw >> 8) & 0xFF) as u8;
                self.info.ip_addr[3] = (ip_raw & 0xFF) as u8;

                self.info.channel = (self.read_reg(REG_WIFI_CHANNEL)? & 0xFF) as u8;
                self.info.rssi = self.read_reg(REG_WIFI_RSSI)? as i8;

                self.state = WifiModuleState::Connected;
                self.info.state = WifiModuleState::Connected;
                self.connect_attempts = 0;

                log::info!(
                    "ATWINC1500: connected to '{}', IP={}.{}.{}.{}, ch={}, RSSI={}",
                    ssid,
                    self.info.ip_addr[0], self.info.ip_addr[1],
                    self.info.ip_addr[2], self.info.ip_addr[3],
                    self.info.channel, self.info.rssi,
                );
                return Ok(());
            }

            if conn_state == 0xFF {
                // Connection explicitly rejected
                break;
            }
        }

        self.connect_attempts += 1;
        if self.connect_attempts >= MAX_CONNECT_ATTEMPTS {
            self.state = WifiModuleState::Error;
            self.info.state = WifiModuleState::Error;
            log::error!("ATWINC1500: max connect attempts reached");
        } else {
            self.state = WifiModuleState::Ready;
            self.info.state = WifiModuleState::Ready;
        }

        log::error!("ATWINC1500: connection to '{}' failed", ssid);
        Err(Error::WifiConnectionFailed)
    }

    /// Disconnect from the current access point.
    pub fn disconnect(&mut self) -> Result<()> {
        if self.state != WifiModuleState::Connected {
            return Ok(());
        }

        self.write_reg(REG_DISCONNECT_CTRL, CONN_CMD_DISCONNECT)?;

        // Close all open sockets
        for i in 0..MAX_SOCKETS {
            if self.sockets[i as usize] == SocketState::Open {
                let _ = self.close_tcp(i);
            }
        }

        self.state = WifiModuleState::Ready;
        self.info.state = WifiModuleState::Ready;
        self.info.ip_addr = [0u8; 4];
        self.info.rssi = 0;
        self.info.channel = 0;

        log::info!("ATWINC1500: disconnected");
        Ok(())
    }

    /// Returns `true` if the module is connected to an access point.
    pub fn is_connected(&self) -> bool {
        self.state == WifiModuleState::Connected
    }

    /// Returns the current RSSI in dBm, or `None` if not connected.
    pub fn rssi(&self) -> Option<i8> {
        if self.state == WifiModuleState::Connected {
            Some(self.info.rssi)
        } else {
            None
        }
    }

    /// Returns the assigned IPv4 address, or `None` if not connected.
    pub fn ip_addr(&self) -> Option<[u8; 4]> {
        if self.state == WifiModuleState::Connected {
            Some(self.info.ip_addr)
        } else {
            None
        }
    }

    /// Poll the module for asynchronous events and update internal state.
    ///
    /// Should be called periodically from the main loop. `now_ms` is the
    /// current monotonic time in milliseconds.
    pub fn poll(&mut self, now_ms: u64) {
        if self.state == WifiModuleState::PowerOff || self.state == WifiModuleState::Error {
            return;
        }

        // Periodically refresh RSSI when connected
        if self.state == WifiModuleState::Connected
            && now_ms.saturating_sub(self.last_rssi_update_ms) >= RSSI_POLL_INTERVAL_MS
        {
            if let Ok(rssi_raw) = self.read_reg(REG_WIFI_RSSI) {
                self.info.rssi = rssi_raw as i8;
            }
            self.last_rssi_update_ms = now_ms;
        }

        // Check for spontaneous disconnect
        if self.state == WifiModuleState::Connected {
            if let Ok(conn_state) = self.read_reg(REG_CONN_STATE) {
                if conn_state != 1 {
                    log::warn!("ATWINC1500: connection lost (state=0x{:08X})", conn_state);
                    self.state = WifiModuleState::Ready;
                    self.info.state = WifiModuleState::Ready;
                    self.info.ip_addr = [0u8; 4];
                    self.info.rssi = 0;
                    self.info.channel = 0;

                    for s in self.sockets.iter_mut() {
                        *s = SocketState::Free;
                    }
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // TCP socket operations
    // -----------------------------------------------------------------------

    /// Open a TCP connection to the given host and port.
    ///
    /// Returns a socket ID (0..6) on success.
    pub fn open_tcp(&mut self, host: &[u8; 4], port: u16) -> Result<u8> {
        if self.state != WifiModuleState::Connected {
            return Err(Error::WifiConnectionFailed);
        }

        // Find a free socket slot
        let slot = self
            .sockets
            .iter()
            .position(|s| *s == SocketState::Free)
            .ok_or(Error::WifiConnectionFailed)? as u8;

        // Pack host IP + port into command register
        let addr: u32 = ((host[0] as u32) << 24)
            | ((host[1] as u32) << 16)
            | ((host[2] as u32) << 8)
            | (host[3] as u32);
        let sock_base = REG_SOCKET_CMD + (slot as u32) * 0x100;

        self.write_reg(sock_base, SOCK_CMD_OPEN_TCP as u32)?;
        self.write_reg(sock_base + 4, addr)?;
        self.write_reg(sock_base + 8, port as u32)?;

        // Wait for socket to open
        for _ in 0..100 {
            spin_delay(10_000);
            let status = self.read_reg(REG_SOCKET_STATUS + (slot as u32) * 4)?;
            if status == 1 {
                self.sockets[slot as usize] = SocketState::Open;
                log::debug!(
                    "ATWINC1500: TCP socket {} opened to {}.{}.{}.{}:{}",
                    slot, host[0], host[1], host[2], host[3], port
                );
                return Ok(slot);
            }
            if status == 0xFF {
                break;
            }
        }

        log::error!("ATWINC1500: failed to open TCP socket");
        Err(Error::WifiTimeout)
    }

    /// Send data over an open TCP socket.
    ///
    /// Returns the number of bytes actually sent.
    pub fn send_tcp(&mut self, socket: u8, data: &[u8]) -> Result<usize> {
        self.validate_socket(socket)?;

        let len = data.len().min(self.tx_buf.len());
        self.tx_buf[..len].copy_from_slice(&data[..len]);

        let sock_base = REG_SOCKET_TX_BUF + (socket as u32) * 0x100;

        // Write payload length
        self.write_reg(REG_SOCKET_TX_LEN + (socket as u32) * 4, len as u32)?;

        // Write payload data
        self.write_block(sock_base, &self.tx_buf[..len])?;

        // Issue send command
        let cmd_base = REG_SOCKET_CMD + (socket as u32) * 0x100;
        self.write_reg(cmd_base, SOCK_CMD_SEND as u32)?;

        // Wait for send acknowledgement
        for _ in 0..100 {
            spin_delay(5_000);
            let status = self.read_reg(cmd_base)?;
            if status == 0 {
                log::debug!("ATWINC1500: sent {} bytes on socket {}", len, socket);
                return Ok(len);
            }
        }

        log::error!("ATWINC1500: TCP send timeout on socket {}", socket);
        Err(Error::WifiTimeout)
    }

    /// Receive data from an open TCP socket into the provided buffer.
    ///
    /// Returns the number of bytes read (0 if no data available).
    pub fn recv_tcp(&mut self, socket: u8, buf: &mut [u8]) -> Result<usize> {
        self.validate_socket(socket)?;

        // Check how many bytes are available
        let avail = self.read_reg(REG_SOCKET_RX_LEN + (socket as u32) * 4)? as usize;
        if avail == 0 {
            return Ok(0);
        }

        let to_read = avail.min(buf.len()).min(self.rx_buf.len());
        let sock_base = REG_SOCKET_RX_BUF + (socket as u32) * 0x100;

        // Issue receive command
        let cmd_base = REG_SOCKET_CMD + (socket as u32) * 0x100;
        self.write_reg(cmd_base, SOCK_CMD_RECV as u32)?;

        // Read data from module buffer
        self.read_block(sock_base, &mut self.rx_buf[..to_read])?;
        buf[..to_read].copy_from_slice(&self.rx_buf[..to_read]);

        log::debug!(
            "ATWINC1500: received {} bytes on socket {}",
            to_read,
            socket
        );
        Ok(to_read)
    }

    /// Close an open TCP socket.
    pub fn close_tcp(&mut self, socket: u8) -> Result<()> {
        if socket >= MAX_SOCKETS {
            return Err(Error::WifiConnectionFailed);
        }

        if self.sockets[socket as usize] == SocketState::Free {
            return Ok(());
        }

        let cmd_base = REG_SOCKET_CMD + (socket as u32) * 0x100;
        self.write_reg(cmd_base, SOCK_CMD_CLOSE as u32)?;

        self.sockets[socket as usize] = SocketState::Free;
        log::debug!("ATWINC1500: socket {} closed", socket);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Accessors
    // -----------------------------------------------------------------------

    /// Get the current module state.
    pub fn state(&self) -> WifiModuleState {
        self.state
    }

    /// Get a reference to the module info struct.
    pub fn info(&self) -> &WifiModuleInfo {
        &self.info
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    /// Validate that a socket ID is in range and open.
    fn validate_socket(&self, socket: u8) -> Result<()> {
        if socket >= MAX_SOCKETS {
            return Err(Error::WifiConnectionFailed);
        }
        if self.sockets[socket as usize] != SocketState::Open {
            return Err(Error::WifiConnectionFailed);
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // SPI register access (ATWINC1500 host interface protocol)
    // -----------------------------------------------------------------------

    /// Read a 32-bit register from the ATWINC1500.
    fn read_reg(&mut self, addr: u32) -> Result<u32> {
        let mut cmd = [0u8; 8];
        cmd[0] = CMD_READ;
        cmd[1] = ((addr >> 16) & 0xFF) as u8;
        cmd[2] = ((addr >> 8) & 0xFF) as u8;
        cmd[3] = (addr & 0xFF) as u8;

        let mut resp = [0u8; 8];

        self.cs.set_low().map_err(|_| Error::SpiBusError)?;
        let result = self.spi.transfer(&mut resp, &cmd);
        let _ = self.cs.set_high();
        result.map_err(|_| Error::SpiBusError)?;

        let value = ((resp[4] as u32) << 24)
            | ((resp[5] as u32) << 16)
            | ((resp[6] as u32) << 8)
            | (resp[7] as u32);

        Ok(value)
    }

    /// Write a 32-bit value to an ATWINC1500 register.
    fn write_reg(&mut self, addr: u32, value: u32) -> Result<()> {
        let cmd = [
            CMD_WRITE,
            ((addr >> 16) & 0xFF) as u8,
            ((addr >> 8) & 0xFF) as u8,
            (addr & 0xFF) as u8,
            ((value >> 24) & 0xFF) as u8,
            ((value >> 16) & 0xFF) as u8,
            ((value >> 8) & 0xFF) as u8,
            (value & 0xFF) as u8,
        ];

        self.cs.set_low().map_err(|_| Error::SpiBusError)?;
        let result = self.spi.write(&cmd);
        let _ = self.cs.set_high();
        result.map_err(|_| Error::SpiBusError)?;

        Ok(())
    }

    /// Read a block of bytes from the ATWINC1500 via DMA read.
    fn read_block(&mut self, addr: u32, buf: &mut [u8]) -> Result<()> {
        // Send DMA read header
        let header = [
            CMD_DMA_READ,
            ((addr >> 16) & 0xFF) as u8,
            ((addr >> 8) & 0xFF) as u8,
            (addr & 0xFF) as u8,
        ];

        self.cs.set_low().map_err(|_| Error::SpiBusError)?;

        // Write command header
        self.spi.write(&header).map_err(|e| {
            let _ = self.cs.set_high();
            Error::SpiBusError
        })?;

        // Read payload — send dummy bytes, receive data
        let zeros = [0u8; 256];
        let len = buf.len().min(256);
        let mut rx = [0u8; 256];
        self.spi
            .transfer(&mut rx[..len], &zeros[..len])
            .map_err(|e| {
                let _ = self.cs.set_high();
                Error::SpiBusError
            })?;
        buf[..len].copy_from_slice(&rx[..len]);

        let _ = self.cs.set_high();
        Ok(())
    }

    /// Write a block of bytes to the ATWINC1500 via DMA write.
    fn write_block(&mut self, addr: u32, data: &[u8]) -> Result<()> {
        let header = [
            CMD_DMA_WRITE,
            ((addr >> 16) & 0xFF) as u8,
            ((addr >> 8) & 0xFF) as u8,
            (addr & 0xFF) as u8,
        ];

        self.cs.set_low().map_err(|_| Error::SpiBusError)?;

        // Write command header
        self.spi.write(&header).map_err(|e| {
            let _ = self.cs.set_high();
            Error::SpiBusError
        })?;

        // Write payload
        self.spi.write(data).map_err(|e| {
            let _ = self.cs.set_high();
            Error::SpiBusError
        })?;

        let _ = self.cs.set_high();
        Ok(())
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

/// Minimal `core::fmt::Write` adapter for `heapless::String`.
struct FmtWriter<'a, const N: usize>(&'a mut String<N>);

impl<const N: usize> core::fmt::Write for FmtWriter<'_, N> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.0.push_str(s).map_err(|_| core::fmt::Error)
    }
}
