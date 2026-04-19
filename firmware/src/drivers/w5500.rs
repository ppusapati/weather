/// W5500 Ethernet controller driver via SPI2.
///
/// The W5500 is a hardwired TCP/IP embedded Ethernet controller with an
/// integrated full-hardware TCP/IP stack, 32 KB internal TX/RX buffer,
/// and 10/100 Mbps Ethernet MAC/PHY. Communicates via SPI at up to
/// 80 MHz using variable-length data frames with BSB (Block Select Bits)
/// addressing.
///
/// # Wiring (SPI2)
///
/// | STM32F407 Pin | W5500 Pin | Function             |
/// |---------------|-----------|----------------------|
/// | PB12          | SCSn      | SPI chip select (NSS)|
/// | PB13          | SCLK      | SPI clock            |
/// | PB14          | MISO      | SPI data out         |
/// | PB15          | MOSI      | SPI data in          |
/// | PD3           | RSTn      | Reset (active low)   |
/// | PD4           | INTn      | Interrupt (active low, EXTI) |

use crate::error::{Error, Result};
use heapless::String;
use serde::{Deserialize, Serialize};

/// Pin assignments for SPI2 / W5500 control lines.
pub mod pins {
    /// PB12 — SPI2 NSS (chip select, directly managed).
    pub const SPI2_NSS: u8 = 28;
    /// PB13 — SPI2 SCK.
    pub const SPI2_SCK: u8 = 29;
    /// PB14 — SPI2 MISO.
    pub const SPI2_MISO: u8 = 30;
    /// PB15 — SPI2 MOSI.
    pub const SPI2_MOSI: u8 = 31;
    /// PD3 — W5500 hardware reset (active low).
    pub const W5500_RST: u8 = 51;
    /// PD4 — W5500 interrupt (active low, EXTI4).
    pub const W5500_INT: u8 = 52;
}

// =============================================================================
// W5500 Register Map — Common Registers (Block Select 0b00000)
// =============================================================================

/// Mode Register — software reset, wake-on-LAN, etc.
const MR: u16 = 0x0000;
/// Gateway Address Register (4 bytes).
const GAR: u16 = 0x0001;
/// Subnet Mask Register (4 bytes).
const SUBR: u16 = 0x0005;
/// Source Hardware Address Register — MAC (6 bytes).
const SHAR: u16 = 0x0009;
/// Source IP Address Register (4 bytes).
const SIPR: u16 = 0x000F;
/// Interrupt Low Level Timer Register.
const INTLEVEL: u16 = 0x0013;
/// Interrupt Register.
const IR: u16 = 0x0015;
/// Interrupt Mask Register.
const IMR: u16 = 0x0016;
/// Socket Interrupt Register.
const SIR: u16 = 0x0017;
/// Socket Interrupt Mask Register.
const SIMR: u16 = 0x0018;
/// Retry Time Register (2 bytes, 100us units).
const RTR: u16 = 0x0019;
/// Retry Count Register.
const RCR: u16 = 0x001B;
/// PHY Configuration Register.
const PHYCFGR: u16 = 0x002E;
/// Chip Version Register (should read 0x04).
const VERSIONR: u16 = 0x0039;

/// Expected chip version for W5500.
const W5500_CHIP_VERSION: u8 = 0x04;

// =============================================================================
// W5500 Register Map — Socket Registers (base offsets, per-socket)
// =============================================================================

/// Socket n Mode Register.
const SN_MR: u16 = 0x0000;
/// Socket n Command Register.
const SN_CR: u16 = 0x0001;
/// Socket n Interrupt Register.
const SN_IR: u16 = 0x0002;
/// Socket n Status Register.
const SN_SR: u16 = 0x0003;
/// Socket n Source Port Register (2 bytes).
const SN_PORT: u16 = 0x0004;
/// Socket n Destination Hardware Address Register (6 bytes).
const SN_DHAR: u16 = 0x0006;
/// Socket n Destination IP Address Register (4 bytes).
const SN_DIPR: u16 = 0x000C;
/// Socket n Destination Port Register (2 bytes).
const SN_DPORT: u16 = 0x0010;
/// Socket n Maximum Segment Size Register (2 bytes).
const SN_MSSR: u16 = 0x0012;
/// Socket n IP TOS Register.
const SN_TOS: u16 = 0x0015;
/// Socket n IP TTL Register.
const SN_TTL: u16 = 0x0016;
/// Socket n RX Buffer Size Register.
const SN_RXBUF_SIZE: u16 = 0x001E;
/// Socket n TX Buffer Size Register.
const SN_TXBUF_SIZE: u16 = 0x001F;
/// Socket n TX Free Size Register (2 bytes).
const SN_TX_FSR: u16 = 0x0020;
/// Socket n TX Read Pointer Register (2 bytes).
const SN_TX_RD: u16 = 0x0022;
/// Socket n TX Write Pointer Register (2 bytes).
const SN_TX_WR: u16 = 0x0024;
/// Socket n RX Received Size Register (2 bytes).
const SN_RX_RSR: u16 = 0x0026;
/// Socket n RX Read Pointer Register (2 bytes).
const SN_RX_RD: u16 = 0x0028;
/// Socket n RX Write Pointer Register (2 bytes).
const SN_RX_WR: u16 = 0x002A;
/// Socket n Interrupt Mask Register.
const SN_IMR: u16 = 0x002C;
/// Socket n Fragment Offset Register (2 bytes).
const SN_FRAG: u16 = 0x002D;
/// Socket n Keep-Alive Timer Register.
const SN_KPALVTR: u16 = 0x002F;

// Socket mode values
const SOCK_MODE_TCP: u8 = 0x01;
const SOCK_MODE_UDP: u8 = 0x02;

// Socket command values
const SOCK_CMD_OPEN: u8 = 0x01;
const SOCK_CMD_LISTEN: u8 = 0x02;
const SOCK_CMD_CONNECT: u8 = 0x04;
const SOCK_CMD_DISCONNECT: u8 = 0x08;
const SOCK_CMD_CLOSE: u8 = 0x10;
const SOCK_CMD_SEND: u8 = 0x20;
const SOCK_CMD_RECV: u8 = 0x40;

// Socket status values
const SOCK_STATUS_CLOSED: u8 = 0x00;
const SOCK_STATUS_INIT: u8 = 0x13;
const SOCK_STATUS_LISTEN: u8 = 0x14;
const SOCK_STATUS_ESTABLISHED: u8 = 0x17;
const SOCK_STATUS_CLOSE_WAIT: u8 = 0x1C;
const SOCK_STATUS_UDP: u8 = 0x22;
const SOCK_STATUS_SYNRECV: u8 = 0x16;
const SOCK_STATUS_SYNSENT: u8 = 0x15;
const SOCK_STATUS_FIN_WAIT: u8 = 0x18;
const SOCK_STATUS_TIME_WAIT: u8 = 0x1B;
const SOCK_STATUS_LAST_ACK: u8 = 0x1D;

// Socket interrupt bits
const SN_IR_CON: u8 = 0x01;
const SN_IR_DISCON: u8 = 0x02;
const SN_IR_RECV: u8 = 0x04;
const SN_IR_TIMEOUT: u8 = 0x08;
const SN_IR_SENDOK: u8 = 0x10;

// MR register bits
const MR_RST: u8 = 0x80;
const MR_WOL: u8 = 0x20;
const MR_PB: u8 = 0x10;
const MR_PPPOE: u8 = 0x08;
const MR_FARP: u8 = 0x02;

// PHYCFGR register bits
const PHY_LINK_UP: u8 = 0x01;
const PHY_SPEED_100: u8 = 0x02;
const PHY_FULL_DUPLEX: u8 = 0x04;
const PHY_POWER_DOWN: u8 = 0x08;
const PHY_RST: u8 = 0x80;

// BSB (Block Select Bits) encoding
const BSB_COMMON: u8 = 0x00;
const BSB_SOCKET_REG: u8 = 0x08;   // base for socket 0, +0x20 per socket
const BSB_SOCKET_TX: u8 = 0x10;    // base for socket 0 TX buf
const BSB_SOCKET_RX: u8 = 0x18;    // base for socket 0 RX buf

// SPI control byte flags
const SPI_READ: u8 = 0x00;
const SPI_WRITE: u8 = 0x04;
const SPI_VDM: u8 = 0x00;  // Variable Data Length Mode

// =============================================================================
// Socket allocation
// =============================================================================

/// Socket 0 — MQTT broker connection.
pub const SOCKET_MQTT: u8 = 0;
/// Socket 1 — HTTP client/server.
pub const SOCKET_HTTP: u8 = 1;
/// Socket 2 — Modbus TCP (port 502).
pub const SOCKET_MODBUS_TCP: u8 = 2;
/// Socket 3 — NTP time synchronization.
pub const SOCKET_NTP: u8 = 3;
/// Sockets 4-7 — Reserved for future use.
pub const SOCKET_RESERVED_START: u8 = 4;
/// Modbus TCP default port.
pub const MODBUS_TCP_PORT: u16 = 502;

/// Maximum number of hardware sockets.
const MAX_SOCKETS: usize = 8;
/// Per-socket TX buffer allocation (2 KB each).
const SOCKET_TX_BUF_KB: u8 = 2;
/// Per-socket RX buffer allocation (2 KB each).
const SOCKET_RX_BUF_KB: u8 = 2;

// =============================================================================
// Types
// =============================================================================

/// State of the Ethernet controller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EthernetState {
    /// Hardware not yet initialized.
    Uninitialized,
    /// PHY link is down (cable unplugged or no link partner).
    LinkDown,
    /// Link is up, waiting for DHCP address assignment.
    DhcpWait,
    /// Fully connected with valid IP configuration.
    Connected,
    /// Unrecoverable error.
    Error,
}

/// Per-socket state tracking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketState {
    /// Socket is closed / available.
    Closed,
    /// Socket is opening (command issued, waiting for status).
    Opening,
    /// Socket is listening (server mode).
    Listening,
    /// Socket has an established TCP connection.
    Connected,
    /// Socket is closing.
    Closing,
    /// Socket is in UDP mode.
    Udp,
}

/// Ethernet link and network information.
#[derive(Debug, Clone)]
pub struct EthernetInfo {
    /// Current controller state.
    pub state: EthernetState,
    /// True if the physical link is up.
    pub link_up: bool,
    /// True if negotiated speed is 100 Mbps (false = 10 Mbps).
    pub speed_100: bool,
    /// True if full-duplex negotiated.
    pub full_duplex: bool,
    /// Assigned IPv4 address.
    pub ip_addr: [u8; 4],
    /// Gateway address.
    pub gateway: [u8; 4],
    /// Subnet mask.
    pub subnet: [u8; 4],
    /// MAC address (6 bytes).
    pub mac_addr: [u8; 6],
}

impl Default for EthernetInfo {
    fn default() -> Self {
        Self {
            state: EthernetState::Uninitialized,
            link_up: false,
            speed_100: false,
            full_duplex: false,
            ip_addr: [0; 4],
            gateway: [0; 4],
            subnet: [0; 4],
            mac_addr: [0; 6],
        }
    }
}

/// DHCP client state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DhcpState {
    Idle,
    Discovering,
    Requesting,
    Bound,
    Renewing,
    Failed,
}

/// W5500 Ethernet controller driver.
///
/// Manages up to 8 hardware TCP/IP sockets, link status monitoring,
/// and DHCP address assignment. Uses BSB (Block Select Bits) addressing
/// for efficient register and buffer access over SPI.
pub struct W5500<SPI, RST, INT> {
    spi: SPI,
    rst: RST,
    int: INT,
    state: EthernetState,
    info: EthernetInfo,
    socket_states: [SocketState; MAX_SOCKETS],
    tx_buf: [u8; 256],
    rx_buf: [u8; 256],
    dhcp_state: DhcpState,
    dhcp_xid: u32,
    last_link_check_ms: u32,
    last_dhcp_ms: u32,
}

impl<SPI, RST, INT> W5500<SPI, RST, INT>
where
    SPI: embedded_hal::spi::SpiDevice,
    RST: embedded_hal::digital::OutputPin,
    INT: embedded_hal::digital::InputPin,
{
    /// Create a new W5500 driver instance.
    ///
    /// The hardware is left uninitialized until [`init()`] is called.
    pub fn new(spi: SPI, rst: RST, int: INT) -> Self {
        Self {
            spi,
            rst,
            int,
            state: EthernetState::Uninitialized,
            info: EthernetInfo::default(),
            socket_states: [SocketState::Closed; MAX_SOCKETS],
            tx_buf: [0u8; 256],
            rx_buf: [0u8; 256],
            dhcp_state: DhcpState::Idle,
            dhcp_xid: 0x5745_4154, // "WEAT"
            last_link_check_ms: 0,
            last_dhcp_ms: 0,
        }
    }

    /// Initialize the W5500: hardware reset, verify chip version, configure
    /// socket buffer sizes, and set retry parameters.
    pub fn init(&mut self) -> Result<()> {
        // Hardware reset: drive RST low for 1ms, then wait 50ms for PLL lock
        self.rst.set_low().map_err(|_| Error::SpiBusError)?;
        spin_delay(10_000);
        self.rst.set_high().map_err(|_| Error::SpiBusError)?;
        spin_delay(500_000);

        // Software reset via MR register
        self.write_reg(BSB_COMMON, MR, MR_RST)?;
        spin_delay(100_000);

        // Verify chip version
        let version = self.read_reg(BSB_COMMON, VERSIONR)?;
        if version != W5500_CHIP_VERSION {
            log::error!("W5500: unexpected version 0x{:02X}, expected 0x{:02X}",
                        version, W5500_CHIP_VERSION);
            self.state = EthernetState::Error;
            return Err(Error::SpiBusError);
        }

        // Confirm MR register is clear after reset
        let mr = self.read_reg(BSB_COMMON, MR)?;
        if mr & MR_RST != 0 {
            log::error!("W5500: reset did not complete");
            self.state = EthernetState::Error;
            return Err(Error::SpiBusError);
        }

        // Configure socket buffer sizes (2 KB TX + 2 KB RX per socket)
        for sock in 0..MAX_SOCKETS as u8 {
            let bsb = self.socket_reg_bsb(sock);
            self.write_reg(bsb, SN_RXBUF_SIZE, SOCKET_RX_BUF_KB)?;
            self.write_reg(bsb, SN_TXBUF_SIZE, SOCKET_TX_BUF_KB)?;
        }

        // Set retry time to 200ms (0x07D0 = 2000 × 100us)
        self.write_buf(BSB_COMMON, RTR, &[0x07, 0xD0])?;
        // Set retry count to 8
        self.write_reg(BSB_COMMON, RCR, 8)?;

        // Enable socket interrupts for all 8 sockets
        self.write_reg(BSB_COMMON, SIMR, 0xFF)?;

        // Reset PHY (toggle PHY_RST bit)
        self.write_reg(BSB_COMMON, PHYCFGR, PHY_RST)?;
        spin_delay(10_000);
        self.write_reg(BSB_COMMON, PHYCFGR, 0x00)?;
        spin_delay(100_000);

        self.state = EthernetState::LinkDown;
        self.info.state = EthernetState::LinkDown;

        log::info!("W5500: initialized, version=0x{:02X}", version);
        Ok(())
    }

    /// Check PHY link status and update internal state.
    ///
    /// Returns `true` if the link is up.
    pub fn check_link(&mut self) -> Result<bool> {
        let phy = self.read_reg(BSB_COMMON, PHYCFGR)?;

        self.info.link_up = (phy & PHY_LINK_UP) != 0;
        self.info.speed_100 = (phy & PHY_SPEED_100) != 0;
        self.info.full_duplex = (phy & PHY_FULL_DUPLEX) != 0;

        if self.info.link_up {
            if self.state == EthernetState::LinkDown
                || self.state == EthernetState::Uninitialized
            {
                log::info!(
                    "W5500: link up, {}Mbps, {}",
                    if self.info.speed_100 { 100 } else { 10 },
                    if self.info.full_duplex { "full-duplex" } else { "half-duplex" }
                );
                self.state = EthernetState::DhcpWait;
                self.info.state = EthernetState::DhcpWait;
            }
        } else {
            if self.state != EthernetState::LinkDown {
                log::warn!("W5500: link down");
                self.state = EthernetState::LinkDown;
                self.info.state = EthernetState::LinkDown;
                self.dhcp_state = DhcpState::Idle;
            }
        }

        Ok(self.info.link_up)
    }

    /// Set the MAC (hardware) address.
    pub fn set_mac(&mut self, mac: &[u8; 6]) -> Result<()> {
        self.write_buf(BSB_COMMON, SHAR, mac)?;
        self.info.mac_addr = *mac;
        log::info!(
            "W5500: MAC={:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
        );
        Ok(())
    }

    /// Set static network configuration (IP, gateway, subnet mask).
    ///
    /// After calling this, the controller transitions to `Connected` state
    /// if the link is up.
    pub fn set_network(
        &mut self,
        ip: &[u8; 4],
        gateway: &[u8; 4],
        subnet: &[u8; 4],
    ) -> Result<()> {
        self.write_buf(BSB_COMMON, GAR, gateway)?;
        self.write_buf(BSB_COMMON, SUBR, subnet)?;
        self.write_buf(BSB_COMMON, SIPR, ip)?;

        self.info.ip_addr = *ip;
        self.info.gateway = *gateway;
        self.info.subnet = *subnet;

        if self.info.link_up {
            self.state = EthernetState::Connected;
            self.info.state = EthernetState::Connected;
        }

        log::info!(
            "W5500: IP={}.{}.{}.{}, GW={}.{}.{}.{}, MASK={}.{}.{}.{}",
            ip[0], ip[1], ip[2], ip[3],
            gateway[0], gateway[1], gateway[2], gateway[3],
            subnet[0], subnet[1], subnet[2], subnet[3],
        );
        Ok(())
    }

    /// Initiate a DHCP discover/request sequence on socket 7.
    ///
    /// This is a simplified DHCP implementation that sends a DISCOVER
    /// and parses the OFFER/ACK. Should be called repeatedly from `poll()`
    /// until the state transitions to `Connected`.
    pub fn dhcp_request(&mut self, now_ms: u32) -> Result<()> {
        match self.dhcp_state {
            DhcpState::Idle => {
                // Open UDP socket on port 68 (DHCP client)
                let bsb = self.socket_reg_bsb(7);
                self.write_reg(bsb, SN_MR, SOCK_MODE_UDP)?;
                self.write_buf(bsb, SN_PORT, &68u16.to_be_bytes())?;
                self.write_reg(bsb, SN_CR, SOCK_CMD_OPEN)?;
                self.wait_socket_cmd(7)?;

                // Build DHCP DISCOVER packet
                let len = self.build_dhcp_discover();
                if len > 0 {
                    self.socket_send_udp(7, &[255, 255, 255, 255], 67, len)?;
                }

                self.dhcp_state = DhcpState::Discovering;
                self.last_dhcp_ms = now_ms;
                log::info!("W5500: DHCP discover sent");
            }
            DhcpState::Discovering => {
                // Check for OFFER
                if let Ok(n) = self.socket_recv(7) {
                    if n > 0 {
                        if self.parse_dhcp_offer() {
                            // Send REQUEST
                            let len = self.build_dhcp_request();
                            if len > 0 {
                                self.socket_send_udp(7, &[255, 255, 255, 255], 67, len)?;
                            }
                            self.dhcp_state = DhcpState::Requesting;
                            self.last_dhcp_ms = now_ms;
                            log::info!("W5500: DHCP request sent");
                        }
                    }
                }
                // Timeout after 5 seconds, retry
                if now_ms.wrapping_sub(self.last_dhcp_ms) > 5000 {
                    self.dhcp_state = DhcpState::Idle;
                }
            }
            DhcpState::Requesting => {
                // Check for ACK
                if let Ok(n) = self.socket_recv(7) {
                    if n > 0 {
                        if self.parse_dhcp_ack() {
                            // Apply the assigned network configuration
                            let ip = self.info.ip_addr;
                            let gw = self.info.gateway;
                            let sub = self.info.subnet;
                            self.write_buf(BSB_COMMON, GAR, &gw)?;
                            self.write_buf(BSB_COMMON, SUBR, &sub)?;
                            self.write_buf(BSB_COMMON, SIPR, &ip)?;

                            self.state = EthernetState::Connected;
                            self.info.state = EthernetState::Connected;
                            self.dhcp_state = DhcpState::Bound;

                            // Close DHCP socket
                            let bsb = self.socket_reg_bsb(7);
                            self.write_reg(bsb, SN_CR, SOCK_CMD_CLOSE)?;
                            self.wait_socket_cmd(7)?;

                            log::info!(
                                "W5500: DHCP bound, IP={}.{}.{}.{}",
                                ip[0], ip[1], ip[2], ip[3]
                            );
                        }
                    }
                }
                if now_ms.wrapping_sub(self.last_dhcp_ms) > 5000 {
                    self.dhcp_state = DhcpState::Idle;
                }
            }
            DhcpState::Bound | DhcpState::Renewing => {
                // Nothing to do until lease renewal
            }
            DhcpState::Failed => {
                // Retry after 30 seconds
                if now_ms.wrapping_sub(self.last_dhcp_ms) > 30_000 {
                    self.dhcp_state = DhcpState::Idle;
                }
            }
        }
        Ok(())
    }

    /// Open a TCP client connection on the given socket.
    ///
    /// # Arguments
    /// - `socket`: hardware socket number (0-7)
    /// - `dest_ip`: destination IPv4 address
    /// - `dest_port`: destination TCP port
    /// - `src_port`: local TCP port
    pub fn open_tcp(
        &mut self,
        socket: u8,
        dest_ip: &[u8; 4],
        dest_port: u16,
        src_port: u16,
    ) -> Result<()> {
        if socket as usize >= MAX_SOCKETS {
            return Err(Error::InvalidConfig);
        }

        let bsb = self.socket_reg_bsb(socket);

        // Close the socket first if it is not already closed
        let status = self.read_reg(bsb, SN_SR)?;
        if status != SOCK_STATUS_CLOSED {
            self.write_reg(bsb, SN_CR, SOCK_CMD_CLOSE)?;
            self.wait_socket_cmd(socket)?;
            spin_delay(5_000);
        }

        // Set TCP mode
        self.write_reg(bsb, SN_MR, SOCK_MODE_TCP)?;
        // Set source port
        self.write_buf(bsb, SN_PORT, &src_port.to_be_bytes())?;
        // Open socket
        self.write_reg(bsb, SN_CR, SOCK_CMD_OPEN)?;
        self.wait_socket_cmd(socket)?;

        // Verify socket entered INIT state
        let status = self.read_reg(bsb, SN_SR)?;
        if status != SOCK_STATUS_INIT {
            log::error!("W5500: socket {} failed to open, status=0x{:02X}", socket, status);
            return Err(Error::SpiBusError);
        }

        // Set destination IP and port
        self.write_buf(bsb, SN_DIPR, dest_ip)?;
        self.write_buf(bsb, SN_DPORT, &dest_port.to_be_bytes())?;

        // Set keep-alive timer (5 × 5s = 25s)
        self.write_reg(bsb, SN_KPALVTR, 5)?;

        // Issue CONNECT command
        self.write_reg(bsb, SN_CR, SOCK_CMD_CONNECT)?;
        self.wait_socket_cmd(socket)?;

        self.socket_states[socket as usize] = SocketState::Opening;

        log::info!(
            "W5500: socket {} connecting to {}.{}.{}.{}:{}",
            socket, dest_ip[0], dest_ip[1], dest_ip[2], dest_ip[3], dest_port
        );
        Ok(())
    }

    /// Send data on a connected TCP socket.
    ///
    /// Returns the number of bytes actually sent (may be less than `data.len()`
    /// if the TX buffer is full).
    pub fn send(&mut self, socket: u8, data: &[u8]) -> Result<usize> {
        if socket as usize >= MAX_SOCKETS {
            return Err(Error::InvalidConfig);
        }
        if self.socket_states[socket as usize] != SocketState::Connected {
            return Err(Error::SpiBusError);
        }

        let bsb = self.socket_reg_bsb(socket);

        // Read TX free size
        let fsr = self.read_reg16(bsb, SN_TX_FSR)?;
        if fsr == 0 {
            return Ok(0);
        }

        let send_len = data.len().min(fsr as usize);

        // Read TX write pointer
        let tx_wr = self.read_reg16(bsb, SN_TX_WR)?;

        // Write data to TX buffer
        let tx_bsb = BSB_SOCKET_TX + (socket << 5);
        self.write_buf(tx_bsb, tx_wr, &data[..send_len])?;

        // Update TX write pointer
        let new_wr = tx_wr.wrapping_add(send_len as u16);
        self.write_buf(bsb, SN_TX_WR, &new_wr.to_be_bytes())?;

        // Issue SEND command
        self.write_reg(bsb, SN_CR, SOCK_CMD_SEND)?;
        self.wait_socket_cmd(socket)?;

        // Wait for SENDOK interrupt or timeout
        for _ in 0..1000 {
            let ir = self.read_reg(bsb, SN_IR)?;
            if ir & SN_IR_SENDOK != 0 {
                self.write_reg(bsb, SN_IR, SN_IR_SENDOK)?;
                return Ok(send_len);
            }
            if ir & SN_IR_TIMEOUT != 0 {
                self.write_reg(bsb, SN_IR, SN_IR_TIMEOUT)?;
                log::error!("W5500: socket {} send timeout", socket);
                return Err(Error::SpiTimeout);
            }
            spin_delay(100);
        }

        log::error!("W5500: socket {} send poll timeout", socket);
        Err(Error::SpiTimeout)
    }

    /// Receive data from a connected TCP socket.
    ///
    /// Reads up to `buf.len()` bytes into the provided buffer. Returns the
    /// number of bytes actually read (0 if no data available).
    pub fn recv(&mut self, socket: u8, buf: &mut [u8]) -> Result<usize> {
        if socket as usize >= MAX_SOCKETS {
            return Err(Error::InvalidConfig);
        }

        let bsb = self.socket_reg_bsb(socket);

        // Check received data size
        let rsr = self.read_reg16(bsb, SN_RX_RSR)?;
        if rsr == 0 {
            return Ok(0);
        }

        let recv_len = buf.len().min(rsr as usize);

        // Read RX read pointer
        let rx_rd = self.read_reg16(bsb, SN_RX_RD)?;

        // Read data from RX buffer
        let rx_bsb = BSB_SOCKET_RX + (socket << 5);
        self.read_buf(rx_bsb, rx_rd, &mut buf[..recv_len])?;

        // Update RX read pointer
        let new_rd = rx_rd.wrapping_add(recv_len as u16);
        self.write_buf(bsb, SN_RX_RD, &new_rd.to_be_bytes())?;

        // Issue RECV command
        self.write_reg(bsb, SN_CR, SOCK_CMD_RECV)?;
        self.wait_socket_cmd(socket)?;

        Ok(recv_len)
    }

    /// Close a socket (graceful disconnect for TCP).
    pub fn close(&mut self, socket: u8) -> Result<()> {
        if socket as usize >= MAX_SOCKETS {
            return Err(Error::InvalidConfig);
        }

        let bsb = self.socket_reg_bsb(socket);

        // For TCP connections, issue DISCONNECT first
        let status = self.read_reg(bsb, SN_SR)?;
        if status == SOCK_STATUS_ESTABLISHED {
            self.write_reg(bsb, SN_CR, SOCK_CMD_DISCONNECT)?;
            self.wait_socket_cmd(socket)?;

            // Wait for disconnect to complete (up to 1 second)
            for _ in 0..100 {
                let s = self.read_reg(bsb, SN_SR)?;
                if s == SOCK_STATUS_CLOSED {
                    break;
                }
                spin_delay(10_000);
            }
        }

        // Force close
        self.write_reg(bsb, SN_CR, SOCK_CMD_CLOSE)?;
        self.wait_socket_cmd(socket)?;

        // Clear any pending interrupts
        self.write_reg(bsb, SN_IR, 0xFF)?;

        self.socket_states[socket as usize] = SocketState::Closed;
        log::debug!("W5500: socket {} closed", socket);
        Ok(())
    }

    /// Poll the W5500 for events: link changes, socket status transitions,
    /// and incoming data. Should be called periodically from the main loop.
    pub fn poll(&mut self, now_ms: u32) -> Result<()> {
        // Check link status periodically (every 1000ms)
        if now_ms.wrapping_sub(self.last_link_check_ms) >= 1000 {
            self.check_link()?;
            self.last_link_check_ms = now_ms;
        }

        // If link is up but no IP, run DHCP
        if self.state == EthernetState::DhcpWait {
            self.dhcp_request(now_ms)?;
        }

        // Check for pending interrupts
        let sir = self.read_reg(BSB_COMMON, SIR)?;
        if sir != 0 {
            self.process_interrupts(sir)?;
        }

        // Update socket states
        for sock in 0..MAX_SOCKETS as u8 {
            self.update_socket_state(sock)?;
        }

        Ok(())
    }

    /// Returns `true` if the Ethernet link is physically up.
    pub fn is_link_up(&self) -> bool {
        self.info.link_up
    }

    /// Returns the current Ethernet state.
    pub fn state(&self) -> EthernetState {
        self.state
    }

    /// Returns a reference to the current Ethernet information.
    pub fn info(&self) -> &EthernetInfo {
        &self.info
    }

    /// Returns the state of a given socket.
    pub fn socket_state(&self, socket: u8) -> SocketState {
        if (socket as usize) < MAX_SOCKETS {
            self.socket_states[socket as usize]
        } else {
            SocketState::Closed
        }
    }

    // =========================================================================
    // SPI access helpers (BSB addressing)
    // =========================================================================

    /// Read a single register from the W5500.
    ///
    /// The W5500 SPI frame is: [addr_hi, addr_lo, control_byte, data...]
    /// Control byte: BSB[4:0] | R/W | OM[1:0]
    fn read_reg(&mut self, bsb: u8, addr: u16) -> Result<u8> {
        let control = (bsb << 3) | SPI_READ | SPI_VDM;
        let tx = [
            (addr >> 8) as u8,
            addr as u8,
            control,
            0x00, // dummy byte for read
        ];
        let mut rx = [0u8; 4];
        self.spi
            .transfer(&mut rx, &tx)
            .map_err(|_| Error::SpiBusError)?;
        Ok(rx[3])
    }

    /// Write a single register to the W5500.
    fn write_reg(&mut self, bsb: u8, addr: u16, value: u8) -> Result<()> {
        let control = (bsb << 3) | SPI_WRITE | SPI_VDM;
        let tx = [
            (addr >> 8) as u8,
            addr as u8,
            control,
            value,
        ];
        self.spi.write(&tx).map_err(|_| Error::SpiBusError)?;
        Ok(())
    }

    /// Read a contiguous block of registers/buffer data from the W5500.
    fn read_buf(&mut self, bsb: u8, addr: u16, buf: &mut [u8]) -> Result<()> {
        if buf.is_empty() {
            return Ok(());
        }

        let control = (bsb << 3) | SPI_READ | SPI_VDM;
        // Build header + dummy bytes for full-duplex read
        let header = [
            (addr >> 8) as u8,
            addr as u8,
            control,
        ];

        // For simplicity, read one byte at a time through individual transfers.
        // In production, a DMA-backed burst transfer would be used.
        for (i, byte) in buf.iter_mut().enumerate() {
            let offset_addr = addr.wrapping_add(i as u16);
            let tx = [
                (offset_addr >> 8) as u8,
                offset_addr as u8,
                control,
                0x00,
            ];
            let mut rx = [0u8; 4];
            self.spi
                .transfer(&mut rx, &tx)
                .map_err(|_| Error::SpiBusError)?;
            *byte = rx[3];
        }
        Ok(())
    }

    /// Write a contiguous block of data to the W5500 registers/buffer.
    fn write_buf(&mut self, bsb: u8, addr: u16, data: &[u8]) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }

        let control = (bsb << 3) | SPI_WRITE | SPI_VDM;

        // Write one byte at a time (production code would use burst/DMA).
        for (i, &byte) in data.iter().enumerate() {
            let offset_addr = addr.wrapping_add(i as u16);
            let tx = [
                (offset_addr >> 8) as u8,
                offset_addr as u8,
                control,
                byte,
            ];
            self.spi.write(&tx).map_err(|_| Error::SpiBusError)?;
        }
        Ok(())
    }

    /// Read a 16-bit big-endian register pair.
    fn read_reg16(&mut self, bsb: u8, addr: u16) -> Result<u16> {
        let hi = self.read_reg(bsb, addr)? as u16;
        let lo = self.read_reg(bsb, addr + 1)? as u16;
        Ok((hi << 8) | lo)
    }

    // =========================================================================
    // Internal helpers
    // =========================================================================

    /// Compute the BSB value for a socket's register block.
    fn socket_reg_bsb(&self, socket: u8) -> u8 {
        BSB_SOCKET_REG + (socket << 2)
    }

    /// Wait for a socket command register to clear (command accepted).
    fn wait_socket_cmd(&mut self, socket: u8) -> Result<()> {
        let bsb = self.socket_reg_bsb(socket);
        for _ in 0..1000 {
            let cr = self.read_reg(bsb, SN_CR)?;
            if cr == 0x00 {
                return Ok(());
            }
            spin_delay(100);
        }
        log::error!("W5500: socket {} command timeout", socket);
        Err(Error::SpiTimeout)
    }

    /// Process socket interrupts flagged in SIR.
    fn process_interrupts(&mut self, sir: u8) -> Result<()> {
        for sock in 0..MAX_SOCKETS as u8 {
            if sir & (1 << sock) == 0 {
                continue;
            }

            let bsb = self.socket_reg_bsb(sock);
            let ir = self.read_reg(bsb, SN_IR)?;

            if ir & SN_IR_CON != 0 {
                log::info!("W5500: socket {} connected", sock);
                self.socket_states[sock as usize] = SocketState::Connected;
            }
            if ir & SN_IR_DISCON != 0 {
                log::info!("W5500: socket {} disconnected", sock);
                self.socket_states[sock as usize] = SocketState::Closed;
            }
            if ir & SN_IR_RECV != 0 {
                log::debug!("W5500: socket {} data available", sock);
            }
            if ir & SN_IR_TIMEOUT != 0 {
                log::warn!("W5500: socket {} timeout", sock);
                self.socket_states[sock as usize] = SocketState::Closed;
            }

            // Clear handled interrupts
            self.write_reg(bsb, SN_IR, ir)?;
        }
        Ok(())
    }

    /// Update the tracked state of a socket from its hardware status register.
    fn update_socket_state(&mut self, socket: u8) -> Result<()> {
        let bsb = self.socket_reg_bsb(socket);
        let status = self.read_reg(bsb, SN_SR)?;

        let new_state = match status {
            SOCK_STATUS_CLOSED => SocketState::Closed,
            SOCK_STATUS_INIT => SocketState::Opening,
            SOCK_STATUS_LISTEN => SocketState::Listening,
            SOCK_STATUS_ESTABLISHED => SocketState::Connected,
            SOCK_STATUS_UDP => SocketState::Udp,
            SOCK_STATUS_CLOSE_WAIT
            | SOCK_STATUS_FIN_WAIT
            | SOCK_STATUS_TIME_WAIT
            | SOCK_STATUS_LAST_ACK => SocketState::Closing,
            SOCK_STATUS_SYNSENT | SOCK_STATUS_SYNRECV => SocketState::Opening,
            _ => SocketState::Closed,
        };

        self.socket_states[socket as usize] = new_state;
        Ok(())
    }

    /// Internal helper: receive data from a socket into the internal rx_buf.
    /// Returns number of bytes received.
    fn socket_recv(&mut self, socket: u8) -> Result<usize> {
        let bsb = self.socket_reg_bsb(socket);
        let rsr = self.read_reg16(bsb, SN_RX_RSR)?;
        if rsr == 0 {
            return Ok(0);
        }

        let recv_len = (rsr as usize).min(self.rx_buf.len());
        let rx_rd = self.read_reg16(bsb, SN_RX_RD)?;
        let rx_bsb = BSB_SOCKET_RX + (socket << 5);

        self.read_buf(rx_bsb, rx_rd, &mut self.rx_buf[..recv_len])?;

        let new_rd = rx_rd.wrapping_add(recv_len as u16);
        self.write_buf(bsb, SN_RX_RD, &new_rd.to_be_bytes())?;
        self.write_reg(bsb, SN_CR, SOCK_CMD_RECV)?;
        self.wait_socket_cmd(socket)?;

        Ok(recv_len)
    }

    /// Internal helper: send UDP data from the internal tx_buf.
    fn socket_send_udp(
        &mut self,
        socket: u8,
        dest_ip: &[u8; 4],
        dest_port: u16,
        len: usize,
    ) -> Result<()> {
        let bsb = self.socket_reg_bsb(socket);

        // Set destination
        self.write_buf(bsb, SN_DIPR, dest_ip)?;
        self.write_buf(bsb, SN_DPORT, &dest_port.to_be_bytes())?;

        // Write data to TX buffer
        let tx_wr = self.read_reg16(bsb, SN_TX_WR)?;
        let tx_bsb = BSB_SOCKET_TX + (socket << 5);
        self.write_buf(tx_bsb, tx_wr, &self.tx_buf[..len])?;

        // Update write pointer and send
        let new_wr = tx_wr.wrapping_add(len as u16);
        self.write_buf(bsb, SN_TX_WR, &new_wr.to_be_bytes())?;
        self.write_reg(bsb, SN_CR, SOCK_CMD_SEND)?;
        self.wait_socket_cmd(socket)?;

        Ok(())
    }

    /// Build a minimal DHCP DISCOVER packet in tx_buf. Returns packet length.
    fn build_dhcp_discover(&mut self) -> usize {
        // Minimal BOOTP/DHCP discover
        self.tx_buf = [0u8; 256];

        self.tx_buf[0] = 0x01; // op: BOOTREQUEST
        self.tx_buf[1] = 0x01; // htype: Ethernet
        self.tx_buf[2] = 0x06; // hlen: 6
        self.tx_buf[3] = 0x00; // hops

        // Transaction ID
        let xid = self.dhcp_xid.to_be_bytes();
        self.tx_buf[4..8].copy_from_slice(&xid);

        // Client hardware address (MAC)
        self.tx_buf[28..34].copy_from_slice(&self.info.mac_addr);

        // Magic cookie
        self.tx_buf[236] = 99;
        self.tx_buf[237] = 130;
        self.tx_buf[238] = 83;
        self.tx_buf[239] = 99;

        // Option 53: DHCP Message Type = DISCOVER (1)
        self.tx_buf[240] = 53;
        self.tx_buf[241] = 1;
        self.tx_buf[242] = 1;

        // Option 55: Parameter Request List
        self.tx_buf[243] = 55;
        self.tx_buf[244] = 3;
        self.tx_buf[245] = 1;  // Subnet mask
        self.tx_buf[246] = 3;  // Router
        self.tx_buf[247] = 6;  // DNS

        // End option
        self.tx_buf[248] = 255;

        249 // packet length
    }

    /// Build a DHCP REQUEST packet in tx_buf. Returns packet length.
    fn build_dhcp_request(&mut self) -> usize {
        // Similar to discover but with Message Type = REQUEST (3)
        // and requested IP
        let len = self.build_dhcp_discover();
        // Override message type to REQUEST
        self.tx_buf[242] = 3;
        len
    }

    /// Parse a DHCP OFFER from rx_buf. Returns true if valid.
    fn parse_dhcp_offer(&mut self) -> bool {
        // Verify it is a BOOTREPLY
        if self.rx_buf[0] != 0x02 {
            return false;
        }
        // Verify transaction ID
        let xid = u32::from_be_bytes([
            self.rx_buf[4],
            self.rx_buf[5],
            self.rx_buf[6],
            self.rx_buf[7],
        ]);
        if xid != self.dhcp_xid {
            return false;
        }
        // Extract offered IP (yiaddr at offset 16)
        self.info.ip_addr.copy_from_slice(&self.rx_buf[16..20]);
        true
    }

    /// Parse a DHCP ACK from rx_buf. Returns true if valid and extracts
    /// gateway and subnet from DHCP options.
    fn parse_dhcp_ack(&mut self) -> bool {
        if self.rx_buf[0] != 0x02 {
            return false;
        }
        let xid = u32::from_be_bytes([
            self.rx_buf[4],
            self.rx_buf[5],
            self.rx_buf[6],
            self.rx_buf[7],
        ]);
        if xid != self.dhcp_xid {
            return false;
        }

        // Extract yiaddr
        self.info.ip_addr.copy_from_slice(&self.rx_buf[16..20]);

        // Parse DHCP options starting at offset 240
        let mut i = 240;
        while i < self.rx_buf.len() - 1 {
            let opt = self.rx_buf[i];
            if opt == 255 {
                break; // End
            }
            if opt == 0 {
                i += 1; // Pad
                continue;
            }
            if i + 1 >= self.rx_buf.len() {
                break;
            }
            let opt_len = self.rx_buf[i + 1] as usize;
            let data_start = i + 2;
            if data_start + opt_len > self.rx_buf.len() {
                break;
            }
            match opt {
                1 if opt_len == 4 => {
                    // Subnet mask
                    self.info.subnet.copy_from_slice(&self.rx_buf[data_start..data_start + 4]);
                }
                3 if opt_len >= 4 => {
                    // Router (gateway)
                    self.info.gateway.copy_from_slice(&self.rx_buf[data_start..data_start + 4]);
                }
                _ => {}
            }
            i = data_start + opt_len;
        }

        true
    }
}

/// Busy-wait delay (placeholder for timer-based delay in production).
#[inline(always)]
fn spin_delay(cycles: u32) {
    for _ in 0..cycles {
        core::hint::spin_loop();
    }
}
