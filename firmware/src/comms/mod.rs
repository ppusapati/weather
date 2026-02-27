/// Communication channel modules.
///
/// Each module implements a specific communication protocol:
/// - WiFi: 802.11 b/g/n connectivity
/// - BLE: Bluetooth Low Energy GATT server
/// - LoRa: Long-range radio via SX1276
/// - MQTT: Message broker client (over WiFi)
/// - HTTP: REST API server (over WiFi)
/// - UART: Serial debug console

pub mod ble;
pub mod http;
pub mod lora;
pub mod mqtt;
pub mod uart_console;
pub mod wifi;

use crate::core::data_pipeline::WeatherReading;
use crate::error::Result;

/// Trait for communication channels that can transmit weather data.
pub trait CommChannel {
    /// Human-readable name of this channel.
    fn name(&self) -> &'static str;

    /// Whether the channel is currently connected / available.
    fn is_available(&self) -> bool;

    /// Send a weather reading through this channel.
    fn send_reading(&mut self, reading: &WeatherReading) -> Result<()>;
}

/// Channel priority for failover logic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ChannelPriority {
    /// WiFi-based channels (MQTT, HTTP) — highest bandwidth.
    Primary = 0,
    /// LoRa — long range, low bandwidth fallback.
    Secondary = 1,
    /// BLE — local only, always available.
    Tertiary = 2,
    /// UART — debug, always active.
    Debug = 3,
}
