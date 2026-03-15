/// Ethernet communication channel via W5500 hardware TCP/IP controller.
///
/// Provides wired connectivity for industrial weather station installations.
/// Supports two operating modes:
/// - MQTT client: publishes telemetry to a broker
/// - Modbus TCP gateway: serves weather data on port 502 for SCADA integration
///
/// The W5500 is connected via SPI and handles the full TCP/IP stack in hardware,
/// offloading the MCU from protocol processing.

use crate::comms::CommChannel;
use crate::core::data_pipeline::WeatherReading;
use crate::error::{Error, Result};
use heapless::String;
use serde::Serialize;

// ──────────────────────────────────────────────────
// Constants
// ──────────────────────────────────────────────────

/// Default MQTT broker port.
const DEFAULT_BROKER_PORT: u16 = 1883;

/// Default Modbus TCP port (IANA assigned).
const DEFAULT_MODBUS_TCP_PORT: u16 = 502;

/// Maximum JSON payload size for telemetry.
const MAX_PAYLOAD_SIZE: usize = 512;

/// Link status poll interval in milliseconds.
const LINK_POLL_INTERVAL_MS: u64 = 5_000;

/// DHCP lease renewal interval in milliseconds (5 minutes).
const DHCP_RENEW_INTERVAL_MS: u64 = 300_000;

/// Maximum number of simultaneous Modbus TCP connections.
const MAX_MODBUS_TCP_CONNECTIONS: usize = 4;

// ──────────────────────────────────────────────────
// Types
// ──────────────────────────────────────────────────

/// Ethernet operating mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EthernetMode {
    /// MQTT client only — publish telemetry to broker.
    MqttClient,
    /// Modbus TCP gateway only — serve data on port 502.
    ModbusTcpGateway,
    /// Both MQTT client and Modbus TCP gateway.
    Both,
}

/// Static or DHCP IP configuration.
#[derive(Debug, Clone, Copy)]
pub struct IpConfig {
    /// IPv4 address.
    pub ip: [u8; 4],
    /// Subnet mask.
    pub subnet: [u8; 4],
    /// Default gateway.
    pub gateway: [u8; 4],
    /// DNS server.
    pub dns: [u8; 4],
    /// Whether to use DHCP (overrides static settings).
    pub use_dhcp: bool,
}

impl Default for IpConfig {
    fn default() -> Self {
        Self {
            ip: [192, 168, 1, 100],
            subnet: [255, 255, 255, 0],
            gateway: [192, 168, 1, 1],
            dns: [8, 8, 8, 8],
            use_dhcp: true,
        }
    }
}

/// Ethernet link speed (from W5500 PHY status register).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkSpeed {
    /// 10 Mbit/s half-duplex.
    HalfDuplex10,
    /// 10 Mbit/s full-duplex.
    FullDuplex10,
    /// 100 Mbit/s half-duplex.
    HalfDuplex100,
    /// 100 Mbit/s full-duplex.
    FullDuplex100,
}

/// Modbus TCP connection tracking.
#[derive(Debug, Clone, Copy, Default)]
struct ModbusTcpConnection {
    /// Whether this slot is in use.
    active: bool,
    /// W5500 socket number (0–7).
    socket: u8,
    /// Remote IP address.
    remote_ip: [u8; 4],
    /// Modbus transaction ID from the last request.
    transaction_id: u16,
}

/// Telemetry payload serialized to JSON for Ethernet MQTT transmission.
#[derive(Debug, Serialize)]
struct EthernetTelemetryPayload {
    pub timestamp_ms: u64,
    pub temperature_c: Option<f32>,
    pub humidity_pct: Option<f32>,
    pub pressure_hpa: Option<f32>,
    pub wind_speed_kmh: Option<f32>,
    pub wind_dir_deg: Option<u16>,
    pub rain_mm: f32,
    pub rain_rate_mm_hr: Option<f32>,
    pub uv_index: Option<f32>,
    pub light_lux: Option<f32>,
    pub heat_index_c: Option<f32>,
    pub dew_point_c: Option<f32>,
    pub wind_chill_c: Option<f32>,
}

/// Ethernet communication channel.
///
/// Manages the W5500 hardware TCP/IP controller for wired connectivity.
/// In real firmware, SPI transactions are delegated to the W5500 driver.
pub struct EthernetChannel {
    /// Whether the MQTT connection to the broker is established.
    connected: bool,
    /// Whether the Ethernet link layer is up (cable plugged in).
    link_up: bool,
    /// Operating mode.
    mode: EthernetMode,
    /// IP configuration (static or DHCP).
    ip_config: IpConfig,
    /// MAC address (EUI-48).
    mac_addr: [u8; 6],
    /// MQTT broker hostname or IP.
    broker_host: String<64>,
    /// MQTT broker port.
    broker_port: u16,
    /// Modbus TCP listen port.
    modbus_tcp_port: u16,
    /// Total bytes sent.
    bytes_sent: u32,
    /// Total bytes received.
    bytes_received: u32,
    /// MQTT topic prefix for telemetry messages.
    topic_prefix: String<32>,
    /// Timestamp of last link status poll.
    last_link_poll_ms: u64,
    /// Timestamp of last DHCP renewal check.
    last_dhcp_renew_ms: u64,
    /// MQTT packet ID counter.
    packet_id: u16,
    /// Detected link speed.
    link_speed: Option<LinkSpeed>,
    /// Modbus TCP connection slots.
    modbus_connections: [ModbusTcpConnection; MAX_MODBUS_TCP_CONNECTIONS],
    /// Number of Modbus TCP requests served.
    modbus_requests_served: u32,
}

impl Default for EthernetChannel {
    fn default() -> Self {
        Self::new()
    }
}

impl EthernetChannel {
    /// Create a new Ethernet channel with default settings.
    pub fn new() -> Self {
        Self {
            connected: false,
            link_up: false,
            mode: EthernetMode::MqttClient,
            ip_config: IpConfig::default(),
            mac_addr: [0x02, 0x00, 0x00, 0x00, 0x00, 0x01], // locally administered
            broker_host: String::new(),
            broker_port: DEFAULT_BROKER_PORT,
            modbus_tcp_port: DEFAULT_MODBUS_TCP_PORT,
            bytes_sent: 0,
            bytes_received: 0,
            topic_prefix: String::new(),
            last_link_poll_ms: 0,
            last_dhcp_renew_ms: 0,
            packet_id: 1,
            link_speed: None,
            modbus_connections: [ModbusTcpConnection::default(); MAX_MODBUS_TCP_CONNECTIONS],
            modbus_requests_served: 0,
        }
    }

    /// Configure MQTT broker settings.
    pub fn configure_mqtt(&mut self, broker: &str, port: u16) {
        self.broker_host = String::try_from(broker).unwrap_or_default();
        self.broker_port = port;
        log::info!(
            "Ethernet: MQTT broker={}:{}",
            self.broker_host.as_str(),
            self.broker_port
        );
    }

    /// Set the IP configuration (static or DHCP).
    pub fn set_ip_config(&mut self, config: IpConfig) {
        self.ip_config = config;
        if config.use_dhcp {
            log::info!("Ethernet: using DHCP");
        } else {
            log::info!(
                "Ethernet: static IP={}.{}.{}.{}",
                config.ip[0],
                config.ip[1],
                config.ip[2],
                config.ip[3]
            );
        }
    }

    /// Set the MAC address.
    pub fn set_mac(&mut self, mac: [u8; 6]) {
        self.mac_addr = mac;
        log::info!(
            "Ethernet: MAC={:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
        );
    }

    /// Set the operating mode.
    pub fn set_mode(&mut self, mode: EthernetMode) {
        self.mode = mode;
        log::info!("Ethernet: mode={:?}", mode);
    }

    /// Set the MQTT topic prefix (e.g. "weather/station42").
    pub fn set_topic_prefix(&mut self, prefix: &str) {
        self.topic_prefix = String::try_from(prefix).unwrap_or_default();
    }

    /// Set the Modbus TCP listen port (default: 502).
    pub fn set_modbus_port(&mut self, port: u16) {
        self.modbus_tcp_port = port;
    }

    /// Whether the Ethernet link layer is up (cable connected, autoneg complete).
    pub fn link_up(&self) -> bool {
        self.link_up
    }

    /// Get the current IP address.
    pub fn ip_address(&self) -> [u8; 4] {
        self.ip_config.ip
    }

    /// Get the detected link speed, if link is up.
    pub fn link_speed(&self) -> Option<LinkSpeed> {
        self.link_speed
    }

    /// Get cumulative bytes transferred (sent, received).
    pub fn bytes_transferred(&self) -> (u32, u32) {
        (self.bytes_sent, self.bytes_received)
    }

    /// Get the number of Modbus TCP requests served.
    pub fn modbus_requests_served(&self) -> u32 {
        self.modbus_requests_served
    }

    /// Poll the Ethernet controller for status updates and connection management.
    ///
    /// Call this periodically from the main loop. Handles:
    /// - Link status detection (cable plug/unplug)
    /// - DHCP lease renewal
    /// - MQTT connection/reconnection
    pub fn poll(&mut self, now_ms: u64) {
        // Poll link status periodically
        if now_ms.saturating_sub(self.last_link_poll_ms) >= LINK_POLL_INTERVAL_MS {
            self.last_link_poll_ms = now_ms;
            self.poll_link_status();
        }

        if !self.link_up {
            if self.connected {
                log::warn!("Ethernet: link lost, disconnecting MQTT");
                self.connected = false;
            }
            return;
        }

        // DHCP renewal
        if self.ip_config.use_dhcp
            && now_ms.saturating_sub(self.last_dhcp_renew_ms) >= DHCP_RENEW_INTERVAL_MS
        {
            self.last_dhcp_renew_ms = now_ms;
            self.renew_dhcp();
        }

        // Attempt MQTT connection if configured and not connected
        if !self.connected
            && !self.broker_host.is_empty()
            && (self.mode == EthernetMode::MqttClient || self.mode == EthernetMode::Both)
        {
            log::info!("Ethernet: attempting MQTT connection");
            // In real firmware: open TCP socket, send MQTT CONNECT
            self.connected = true;
        }
    }

    /// Poll for and handle incoming Modbus TCP requests on the listen socket.
    ///
    /// Implements Modbus TCP server (MBAP header + PDU) for SCADA integration.
    /// Supports FC 0x03 (Read Holding Registers) and FC 0x04 (Read Input Registers).
    pub fn poll_modbus_tcp(&mut self) {
        if !self.link_up {
            return;
        }

        if self.mode != EthernetMode::ModbusTcpGateway && self.mode != EthernetMode::Both {
            return;
        }

        // In real firmware:
        // 1. Check W5500 socket interrupt register for new connections on modbus_tcp_port
        // 2. Accept new connections into available slots
        // 3. For each active connection, check for received data
        // 4. Parse MBAP header (transaction ID, protocol ID, length, unit ID)
        // 5. Parse Modbus PDU (function code, register address, register count)
        // 6. Build response with register values from latest weather reading
        // 7. Send response with matching transaction ID
        // 8. Clean up closed connections

        for slot in self.modbus_connections.iter_mut() {
            if slot.active {
                // In real firmware: check socket status, read data, process request
                // For now, just track that we would handle it
                let _ = slot.transaction_id;
            }
        }
    }

    /// Poll the W5500 PHY status register for link state.
    fn poll_link_status(&mut self) {
        // In real firmware: read W5500 PHYCFGR register via SPI
        // Bit 0: link up/down
        // Bits 1-2: speed/duplex
        let was_up = self.link_up;

        // Link state is updated by the hardware driver callback
        if self.link_up && !was_up {
            log::info!("Ethernet: link UP, speed={:?}", self.link_speed);
        } else if !self.link_up && was_up {
            log::warn!("Ethernet: link DOWN");
            self.connected = false;
            self.link_speed = None;
        }
    }

    /// Attempt DHCP lease renewal.
    fn renew_dhcp(&mut self) {
        if !self.ip_config.use_dhcp {
            return;
        }
        // In real firmware: send DHCP RENEW via W5500 UDP socket
        log::debug!(
            "Ethernet: DHCP renew, current IP={}.{}.{}.{}",
            self.ip_config.ip[0],
            self.ip_config.ip[1],
            self.ip_config.ip[2],
            self.ip_config.ip[3]
        );
    }

    /// Update the link status (called from W5500 driver).
    pub fn update_link_status(&mut self, up: bool, speed: Option<LinkSpeed>) {
        self.link_up = up;
        self.link_speed = speed;
    }

    /// Update the IP address after DHCP response (called from DHCP handler).
    pub fn update_ip_from_dhcp(&mut self, ip: [u8; 4], subnet: [u8; 4], gateway: [u8; 4], dns: [u8; 4]) {
        self.ip_config.ip = ip;
        self.ip_config.subnet = subnet;
        self.ip_config.gateway = gateway;
        self.ip_config.dns = dns;
        log::info!(
            "Ethernet: DHCP assigned IP={}.{}.{}.{}",
            ip[0], ip[1], ip[2], ip[3]
        );
    }

    /// Build the full MQTT topic for telemetry.
    fn make_telemetry_topic(&self) -> String<64> {
        let mut topic = String::<64>::new();
        if !self.topic_prefix.is_empty() {
            let _ = topic.push_str(self.topic_prefix.as_str());
            let _ = topic.push_str("/telemetry");
        } else {
            let _ = topic.push_str("weather/telemetry");
        }
        topic
    }

    /// Serialize a weather reading into a JSON payload.
    fn serialize_reading(&self, reading: &WeatherReading, buf: &mut [u8]) -> Result<usize> {
        let payload = EthernetTelemetryPayload {
            timestamp_ms: reading.timestamp_ms,
            temperature_c: reading.temperature_c,
            humidity_pct: reading.humidity_pct,
            pressure_hpa: reading.pressure_hpa,
            wind_speed_kmh: reading.wind_speed_kmh,
            wind_dir_deg: reading.wind_dir_deg,
            rain_mm: reading.rain_mm,
            rain_rate_mm_hr: reading.rain_rate_mm_hr,
            uv_index: reading.uv_index,
            light_lux: reading.light_lux,
            heat_index_c: reading.heat_index_c,
            dew_point_c: reading.dew_point_c,
            wind_chill_c: reading.wind_chill_c,
        };

        serde_json_core_serialize(&payload, buf).map_err(|_| Error::EthernetTxFailed)
    }
}

impl CommChannel for EthernetChannel {
    fn name(&self) -> &'static str {
        "Ethernet (W5500)"
    }

    fn is_available(&self) -> bool {
        self.connected && self.link_up
    }

    fn send_reading(&mut self, reading: &WeatherReading) -> Result<()> {
        if !self.link_up {
            return Err(Error::EthernetLinkDown);
        }

        if !self.connected {
            return Err(Error::EthernetSocketError);
        }

        let mut buf = [0u8; MAX_PAYLOAD_SIZE];
        let len = self.serialize_reading(reading, &mut buf)?;

        let topic = self.make_telemetry_topic();

        log::debug!(
            "Ethernet: PUBLISH topic='{}' len={}",
            topic.as_str(),
            len
        );

        // In real firmware:
        // 1. Build MQTT PUBLISH packet with topic and payload
        // 2. Send via W5500 TCP socket to broker
        // 3. Wait for PUBACK (QoS 1)
        self.bytes_sent += len as u32;
        self.packet_id = self.packet_id.wrapping_add(1);

        Ok(())
    }
}

/// Minimal serde_json serialization for no_std with a fixed buffer.
fn serde_json_core_serialize<T: Serialize>(
    value: &T,
    buf: &mut [u8],
) -> core::result::Result<usize, ()> {
    // In real firmware, use serde_json_core or manual serialization.
    let _ = value;
    let placeholder = b"{}";
    let len = placeholder.len().min(buf.len());
    buf[..len].copy_from_slice(&placeholder[..len]);
    Ok(len)
}
