/// Cellular communication channel via SIM7600E-H modem.
///
/// Provides MQTT-over-cellular for remote weather stations without WiFi/Ethernet.
/// Supports both raw TCP and the SIM7600's built-in MQTT AT-command interface.
/// Designed for low-bandwidth periodic telemetry with automatic reconnection.

use crate::comms::CommChannel;
use crate::core::data_pipeline::WeatherReading;
use crate::error::{Error, Result};
use heapless::String;
use serde::Serialize;

// ──────────────────────────────────────────────────
// Constants
// ──────────────────────────────────────────────────

/// Minimum RSSI considered usable for data transmission.
const MIN_USABLE_RSSI: i16 = -100;

/// Default MQTT broker port.
const DEFAULT_BROKER_PORT: u16 = 1883;

/// Maximum JSON payload size for telemetry.
const MAX_PAYLOAD_SIZE: usize = 512;

/// Signal poll interval in milliseconds.
const SIGNAL_POLL_INTERVAL_MS: u64 = 30_000;

// ──────────────────────────────────────────────────
// Types
// ──────────────────────────────────────────────────

/// Transport mode for the cellular channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellularTransport {
    /// Raw TCP socket — application builds MQTT packets.
    Tcp,
    /// SIM7600 built-in MQTT support via AT+CMQTT commands.
    MqttNative,
}

/// Cellular network registration state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistrationState {
    /// Not registered, not searching.
    NotRegistered,
    /// Searching for network.
    Searching,
    /// Registered on home network.
    RegisteredHome,
    /// Registered, roaming.
    RegisteredRoaming,
    /// Registration denied.
    Denied,
}

/// Telemetry payload serialized to JSON for cellular transmission.
#[derive(Debug, Serialize)]
struct CellularTelemetryPayload {
    pub ts: u64,
    pub t: Option<f32>,
    pub h: Option<f32>,
    pub p: Option<f32>,
    pub ws: Option<f32>,
    pub wd: Option<u16>,
    pub r: f32,
    pub rr: Option<f32>,
    pub uv: Option<f32>,
    pub lx: Option<f32>,
    pub hi: Option<f32>,
    pub dp: Option<f32>,
    pub wc: Option<f32>,
}

/// Cellular communication channel.
///
/// Manages the SIM7600E-H modem for MQTT telemetry over 4G LTE.
/// In real firmware, this delegates AT commands to the SIM7600 UART driver.
pub struct CellularChannel {
    /// Whether a data connection to the broker is established.
    connected: bool,
    /// Network registration state.
    registration: RegistrationState,
    /// Transport mode (raw TCP vs native MQTT).
    transport: CellularTransport,
    /// MQTT broker hostname or IP.
    broker_host: String<64>,
    /// MQTT broker port.
    broker_port: u16,
    /// Cellular APN string.
    apn: String<64>,
    /// Last measured signal strength (RSSI in dBm, 0 = unknown).
    signal_rssi: i16,
    /// Total bytes sent over the cellular link.
    bytes_sent: u32,
    /// Total bytes received over the cellular link.
    bytes_received: u32,
    /// Number of failed transmit attempts.
    tx_failures: u32,
    /// MQTT topic prefix for telemetry messages.
    topic_prefix: String<32>,
    /// Timestamp of last signal strength poll.
    last_signal_poll_ms: u64,
    /// MQTT packet ID counter.
    packet_id: u16,
}

impl Default for CellularChannel {
    fn default() -> Self {
        Self::new()
    }
}

impl CellularChannel {
    /// Create a new cellular channel with default settings.
    pub fn new() -> Self {
        Self {
            connected: false,
            registration: RegistrationState::NotRegistered,
            transport: CellularTransport::MqttNative,
            broker_host: String::new(),
            broker_port: DEFAULT_BROKER_PORT,
            apn: String::new(),
            signal_rssi: 0,
            bytes_sent: 0,
            bytes_received: 0,
            tx_failures: 0,
            topic_prefix: String::new(),
            last_signal_poll_ms: 0,
            packet_id: 1,
        }
    }

    /// Configure the cellular channel with broker and APN settings.
    ///
    /// Must be called before `poll()` can establish a connection.
    pub fn configure(&mut self, broker: &str, port: u16, apn: &str) {
        self.broker_host = String::try_from(broker).unwrap_or_default();
        self.broker_port = port;
        self.apn = String::try_from(apn).unwrap_or_default();
        log::info!(
            "Cellular: configured broker={}:{}, APN={}",
            self.broker_host.as_str(),
            self.broker_port,
            self.apn.as_str()
        );
    }

    /// Set the transport mode (raw TCP or native MQTT).
    pub fn set_transport(&mut self, transport: CellularTransport) {
        self.transport = transport;
    }

    /// Set the MQTT topic prefix (e.g. "weather/station42").
    pub fn set_topic_prefix(&mut self, prefix: &str) {
        self.topic_prefix = String::try_from(prefix).unwrap_or_default();
    }

    /// Get the last measured signal strength in dBm.
    ///
    /// Returns 0 if no measurement has been taken.
    /// Typical range: -51 (excellent) to -113 (marginal).
    pub fn signal_strength(&self) -> i16 {
        self.signal_rssi
    }

    /// Get the current network registration state.
    pub fn registration_state(&self) -> RegistrationState {
        self.registration
    }

    /// Get cumulative bytes transferred (sent, received).
    pub fn bytes_transferred(&self) -> (u32, u32) {
        (self.bytes_sent, self.bytes_received)
    }

    /// Get the number of failed transmit attempts.
    pub fn tx_failure_count(&self) -> u32 {
        self.tx_failures
    }

    /// Poll the cellular modem for status updates.
    ///
    /// Call this periodically from the main loop. Handles:
    /// - Network registration checks
    /// - Signal strength polling
    /// - Connection keep-alive
    pub fn poll(&mut self, now_ms: u64) {
        // Periodically poll signal strength
        if now_ms.saturating_sub(self.last_signal_poll_ms) >= SIGNAL_POLL_INTERVAL_MS {
            self.last_signal_poll_ms = now_ms;
            self.poll_signal_strength();
        }

        // In real firmware:
        // 1. Check AT+CREG? for registration state
        // 2. If registered but not connected, attempt data connection (AT+CGACT)
        // 3. If data connection up, open MQTT connection if needed
        // 4. Send MQTT PINGREQ for keep-alive

        if self.registration == RegistrationState::RegisteredHome
            || self.registration == RegistrationState::RegisteredRoaming
        {
            if !self.connected && !self.broker_host.is_empty() {
                log::info!("Cellular: attempting MQTT connection");
                // In real firmware: AT+CMQTTSTART / AT+CMQTTCONNECT
                self.connected = true;
            }
        }
    }

    /// Poll the modem for current signal strength.
    fn poll_signal_strength(&mut self) {
        // In real firmware: send AT+CSQ, parse response
        // CSQ returns 0–31 (mapped to -113 to -51 dBm) or 99 (unknown)
        // For now, signal_rssi is set by the hardware driver callback.
        log::debug!("Cellular: signal RSSI={} dBm", self.signal_rssi);
    }

    /// Update the signal strength reading (called from modem driver).
    pub fn update_signal(&mut self, rssi_dbm: i16) {
        self.signal_rssi = rssi_dbm;
    }

    /// Update the registration state (called from modem driver).
    pub fn update_registration(&mut self, state: RegistrationState) {
        if self.registration != state {
            log::info!("Cellular: registration {:?} -> {:?}", self.registration, state);
            self.registration = state;

            // If we lost registration, we also lose the data connection
            if state == RegistrationState::NotRegistered
                || state == RegistrationState::Denied
                || state == RegistrationState::Searching
            {
                self.connected = false;
            }
        }
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

    /// Serialize a weather reading into a compact JSON payload.
    fn serialize_reading(&self, reading: &WeatherReading, buf: &mut [u8]) -> Result<usize> {
        let payload = CellularTelemetryPayload {
            ts: reading.timestamp_ms,
            t: reading.temperature_c,
            h: reading.humidity_pct,
            p: reading.pressure_hpa,
            ws: reading.wind_speed_kmh,
            wd: reading.wind_dir_deg,
            r: reading.rain_mm,
            rr: reading.rain_rate_mm_hr,
            uv: reading.uv_index,
            lx: reading.light_lux,
            hi: reading.heat_index_c,
            dp: reading.dew_point_c,
            wc: reading.wind_chill_c,
        };

        serde_json_core_serialize(&payload, buf).map_err(|_| Error::CellularTxFailed)
    }
}

impl CommChannel for CellularChannel {
    fn name(&self) -> &'static str {
        "Cellular (SIM7600)"
    }

    fn is_available(&self) -> bool {
        self.connected && self.signal_rssi >= MIN_USABLE_RSSI && self.signal_rssi != 0
    }

    fn send_reading(&mut self, reading: &WeatherReading) -> Result<()> {
        if !self.connected {
            self.tx_failures += 1;
            return Err(Error::CellularDataConnectionFailed);
        }

        if self.signal_rssi != 0 && self.signal_rssi < MIN_USABLE_RSSI {
            self.tx_failures += 1;
            return Err(Error::CellularNoSignal);
        }

        let mut buf = [0u8; MAX_PAYLOAD_SIZE];
        let len = self.serialize_reading(reading, &mut buf)?;

        let topic = self.make_telemetry_topic();

        log::debug!(
            "Cellular: PUBLISH topic='{}' len={} via {:?}",
            topic.as_str(),
            len,
            self.transport
        );

        // In real firmware:
        // - MqttNative: AT+CMQTTPUB with topic and payload
        // - Tcp: build MQTT PUBLISH packet, send over TCP socket
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
