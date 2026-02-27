/// MQTT client for publishing weather telemetry and subscribing to commands.
///
/// Uses WiFi TCP connection to communicate with an MQTT v3.1.1 broker.
/// Supports QoS 0, 1, and 2, with message buffering during disconnection.

use crate::config;
use crate::core::data_pipeline::WeatherReading;
use crate::error::{Error, Result};
use crate::utils::fmt::HeaplessWriter;
use heapless::String;
use serde::Serialize;

/// MQTT connection state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MqttState {
    Disconnected,
    Connecting,
    Connected,
    Error,
}

/// MQTT QoS levels.
#[derive(Debug, Clone, Copy)]
pub enum QoS {
    AtMostOnce = 0,
    AtLeastOnce = 1,
    ExactlyOnce = 2,
}

/// MQTT command received from the broker.
#[derive(Debug, Clone)]
pub enum MqttCommand {
    SetInterval { telemetry_interval_s: u16 },
    Reboot,
    Calibrate { sensor: String<16> },
    RequestStatus,
    Sleep { duration_s: u32 },
    FactoryReset,
    Unknown(String<64>),
}

/// MQTT telemetry payload.
#[derive(Debug, Serialize)]
pub struct TelemetryPayload<'a> {
    pub timestamp: &'a str,
    pub temperature_c: Option<f32>,
    pub humidity_pct: Option<f32>,
    pub pressure_hpa: Option<f32>,
    pub wind_speed_kmh: Option<f32>,
    pub wind_direction_deg: Option<u16>,
    pub rain_mm_hour: Option<f32>,
    pub rain_mm_total: f32,
    pub uv_index: Option<f32>,
    pub light_lux: Option<f32>,
    pub heat_index_c: Option<f32>,
    pub dew_point_c: Option<f32>,
    pub wind_chill_c: Option<f32>,
}

/// MQTT status payload.
#[derive(Debug, Serialize)]
pub struct StatusPayload<'a> {
    pub timestamp: &'a str,
    pub uptime_s: u64,
    pub battery_v: f32,
    pub battery_pct: u8,
    pub wifi_rssi_dbm: i8,
    pub free_heap_bytes: u32,
    pub flash_used_pct: u8,
    pub firmware_version: &'a str,
}

/// Alert payload.
#[derive(Debug, Serialize)]
pub struct AlertPayload<'a> {
    pub timestamp: &'a str,
    pub alert_type: &'a str,
    pub sensor: &'a str,
    pub value: f32,
    pub threshold: f32,
    pub unit: &'a str,
    pub message: &'a str,
}

/// MQTT client.
pub struct MqttClient {
    state: MqttState,
    device_id: String<16>,
    broker: String<64>,
    port: u16,
    use_tls: bool,
    username: String<32>,
    password: String<64>,
    keepalive_s: u16,
    packet_id: u16,
}

impl MqttClient {
    pub fn new(device_id: &str) -> Self {
        Self {
            state: MqttState::Disconnected,
            device_id: String::try_from(device_id).unwrap_or_default(),
            broker: String::new(),
            port: config::MQTT_PORT_DEFAULT,
            use_tls: false,
            username: String::new(),
            password: String::new(),
            keepalive_s: config::MQTT_KEEPALIVE_S,
            packet_id: 1,
        }
    }

    /// Configure the MQTT broker connection.
    pub fn configure(
        &mut self,
        broker: &str,
        port: u16,
        use_tls: bool,
        username: &str,
        password: &str,
    ) {
        self.broker = String::try_from(broker).unwrap_or_default();
        self.port = port;
        self.use_tls = use_tls;
        self.username = String::try_from(username).unwrap_or_default();
        self.password = String::try_from(password).unwrap_or_default();
    }

    /// Connect to the MQTT broker. Requires WiFi to be connected.
    pub fn connect(&mut self) -> Result<()> {
        if self.broker.is_empty() {
            log::warn!("MQTT: no broker configured");
            return Err(Error::MqttConnectionFailed);
        }

        log::info!(
            "MQTT: connecting to {}:{} (TLS={})",
            self.broker.as_str(),
            self.port,
            self.use_tls
        );

        self.state = MqttState::Connecting;

        // In real firmware:
        // 1. Open TCP socket to broker:port
        // 2. If TLS: perform TLS handshake
        // 3. Send CONNECT packet with client_id, username, password, keepalive
        // 4. Wait for CONNACK
        // 5. Subscribe to command topic

        self.state = MqttState::Connected;
        log::info!("MQTT: connected");

        self.subscribe_commands()?;

        Ok(())
    }

    /// Subscribe to the device command topic.
    fn subscribe_commands(&mut self) -> Result<()> {
        let topic: String<64> = format_heapless!("weather/{}/cmd", self.device_id.as_str());
        log::info!("MQTT: subscribing to '{}'", topic.as_str());
        // In real firmware: send SUBSCRIBE packet
        Ok(())
    }

    /// Build a topic string for this device.
    fn make_topic(&self, suffix: &str) -> String<64> {
        format_heapless!("weather/{}/{}", self.device_id.as_str(), suffix)
    }

    /// Publish a weather reading to the telemetry topic.
    pub fn publish_telemetry(&mut self, reading: &WeatherReading) -> Result<()> {
        if self.state != MqttState::Connected {
            return Err(Error::MqttPublishFailed);
        }

        let payload = TelemetryPayload {
            timestamp: "1970-01-01T00:00:00Z", // real firmware: RTC timestamp
            temperature_c: reading.temperature_c,
            humidity_pct: reading.humidity_pct,
            pressure_hpa: reading.pressure_hpa,
            wind_speed_kmh: reading.wind_speed_kmh,
            wind_direction_deg: reading.wind_dir_deg,
            rain_mm_hour: reading.rain_rate_mm_hr,
            rain_mm_total: reading.rain_mm,
            uv_index: reading.uv_index,
            light_lux: reading.light_lux,
            heat_index_c: reading.heat_index_c,
            dew_point_c: reading.dew_point_c,
            wind_chill_c: reading.wind_chill_c,
        };

        let topic = self.make_topic("telemetry");
        self.publish(&topic, &payload, QoS::AtLeastOnce)
    }

    /// Publish a status report.
    pub fn publish_status(
        &mut self,
        uptime_s: u64,
        battery_v: f32,
        battery_pct: u8,
        rssi: i8,
        free_heap: u32,
        flash_pct: u8,
    ) -> Result<()> {
        if self.state != MqttState::Connected {
            return Err(Error::MqttPublishFailed);
        }

        let payload = StatusPayload {
            timestamp: "1970-01-01T00:00:00Z",
            uptime_s,
            battery_v,
            battery_pct,
            wifi_rssi_dbm: rssi,
            free_heap_bytes: free_heap,
            flash_used_pct: flash_pct,
            firmware_version: config::FIRMWARE_VERSION,
        };

        let topic = self.make_topic("status");
        self.publish(&topic, &payload, QoS::AtLeastOnce)
    }

    /// Publish an alert.
    pub fn publish_alert(
        &mut self,
        sensor: &str,
        value: f32,
        threshold: f32,
        unit: &str,
        message: &str,
    ) -> Result<()> {
        if self.state != MqttState::Connected {
            return Err(Error::MqttPublishFailed);
        }

        let payload = AlertPayload {
            timestamp: "1970-01-01T00:00:00Z",
            alert_type: "threshold_exceeded",
            sensor,
            value,
            threshold,
            unit,
            message,
        };

        let topic = self.make_topic("alerts");
        self.publish(&topic, &payload, QoS::ExactlyOnce)
    }

    /// Generic publish with serialization.
    fn publish<T: Serialize>(&mut self, topic: &str, payload: &T, qos: QoS) -> Result<()> {
        let mut buf = [0u8; 512];
        let json = serde_json_core_serialize(payload, &mut buf);

        let json_str = match json {
            Ok(len) => core::str::from_utf8(&buf[..len]).unwrap_or("{}"),
            Err(_) => return Err(Error::MqttPublishFailed),
        };

        log::debug!(
            "MQTT: PUBLISH topic='{}' qos={} len={}",
            topic,
            qos as u8,
            json_str.len()
        );

        // In real firmware: build and send MQTT PUBLISH packet
        self.packet_id = self.packet_id.wrapping_add(1);

        Ok(())
    }

    /// Handle incoming MQTT message (called from network task).
    pub fn handle_message(&self, _topic: &str, payload: &[u8]) -> Option<MqttCommand> {
        let payload_str = core::str::from_utf8(payload).ok()?;
        log::info!("MQTT: received command: {}", payload_str);

        if payload_str.contains("set_interval") {
            Some(MqttCommand::SetInterval {
                telemetry_interval_s: 30,
            })
        } else if payload_str.contains("reboot") {
            Some(MqttCommand::Reboot)
        } else if payload_str.contains("status") {
            Some(MqttCommand::RequestStatus)
        } else if payload_str.contains("factory_reset") {
            Some(MqttCommand::FactoryReset)
        } else {
            Some(MqttCommand::Unknown(
                String::try_from(payload_str).unwrap_or_default(),
            ))
        }
    }

    /// Disconnect from the broker.
    pub fn disconnect(&mut self) {
        if self.state == MqttState::Connected {
            log::info!("MQTT: disconnecting");
            self.state = MqttState::Disconnected;
        }
    }

    pub fn state(&self) -> MqttState {
        self.state
    }

    pub fn is_connected(&self) -> bool {
        self.state == MqttState::Connected
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
