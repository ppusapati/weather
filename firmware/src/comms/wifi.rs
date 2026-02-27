/// WiFi manager for ESP32-S3.
///
/// Handles connection, reconnection with exponential backoff,
/// NTP time synchronization, and connection state tracking.

use crate::config;
use crate::error::{Error, Result};
use crate::utils::fmt::HeaplessWriter;
use heapless::String;

/// WiFi connection state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WifiState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Failed,
}

/// WiFi event for state machine transitions.
#[derive(Debug, Clone, Copy)]
pub enum WifiEvent {
    Connect,
    Connected { ip: [u8; 4] },
    Disconnected,
    ConnectionFailed,
    ReconnectTimeout,
}

/// WiFi connection info.
#[derive(Debug, Clone)]
pub struct WifiInfo {
    pub ip_address: [u8; 4],
    pub rssi_dbm: i8,
    pub channel: u8,
    pub ssid: String<32>,
}

/// WiFi manager handling connection lifecycle.
pub struct WifiManager {
    state: WifiState,
    retry_count: u8,
    max_retries: u8,
    backoff_ms: u64,
    ssid: String<32>,
    password: String<64>,
    info: Option<WifiInfo>,
    ntp_synced: bool,
}

impl Default for WifiManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WifiManager {
    pub fn new() -> Self {
        Self {
            state: WifiState::Disconnected,
            retry_count: 0,
            max_retries: config::WIFI_RETRY_MAX,
            backoff_ms: config::WIFI_BACKOFF_INITIAL_MS,
            ssid: String::new(),
            password: String::new(),
            info: None,
            ntp_synced: false,
        }
    }

    /// Set WiFi credentials.
    pub fn set_credentials(&mut self, ssid: &str, password: &str) {
        self.ssid = String::try_from(ssid).unwrap_or_default();
        self.password = String::try_from(password).unwrap_or_default();
    }

    /// Check if credentials are configured.
    pub fn has_credentials(&self) -> bool {
        !self.ssid.is_empty()
    }

    /// Initialize WiFi hardware and attempt connection.
    pub fn init(&mut self) -> Result<()> {
        if !self.has_credentials() {
            log::warn!("WiFi: no credentials configured, staying disconnected");
            self.state = WifiState::Disconnected;
            return Ok(());
        }

        log::info!("WiFi: initializing, SSID={}", self.ssid.as_str());
        self.state = WifiState::Connecting;
        self.connect()
    }

    /// Attempt to connect to the configured AP.
    fn connect(&mut self) -> Result<()> {
        log::info!(
            "WiFi: connecting to '{}' (attempt {}/{})",
            self.ssid.as_str(),
            self.retry_count + 1,
            self.max_retries
        );

        // In real firmware: esp_wifi::wifi::WiFi::connect()
        // The event handler calls handle_event() when the result is known.
        Ok(())
    }

    /// Handle a WiFi event (called from WiFi event loop).
    pub fn handle_event(&mut self, event: WifiEvent) {
        match event {
            WifiEvent::Connected { ip } => {
                self.state = WifiState::Connected;
                self.retry_count = 0;
                self.backoff_ms = config::WIFI_BACKOFF_INITIAL_MS;
                self.info = Some(WifiInfo {
                    ip_address: ip,
                    rssi_dbm: 0,
                    channel: 0,
                    ssid: self.ssid.clone(),
                });
                log::info!(
                    "WiFi: connected, IP={}.{}.{}.{}",
                    ip[0], ip[1], ip[2], ip[3]
                );
            }
            WifiEvent::Disconnected => {
                log::warn!("WiFi: disconnected");
                self.state = WifiState::Reconnecting;
                self.info = None;
                self.attempt_reconnect();
            }
            WifiEvent::ConnectionFailed => {
                self.retry_count += 1;
                if self.retry_count >= self.max_retries {
                    log::error!("WiFi: max retries exceeded, giving up");
                    self.state = WifiState::Failed;
                } else {
                    self.state = WifiState::Reconnecting;
                    self.attempt_reconnect();
                }
            }
            _ => {}
        }
    }

    fn attempt_reconnect(&mut self) {
        let delay = self.backoff_ms.min(config::WIFI_BACKOFF_MAX_MS);
        log::info!("WiFi: reconnecting in {}ms", delay);
        self.backoff_ms = (self.backoff_ms * 2).min(config::WIFI_BACKOFF_MAX_MS);
        if let Err(e) = self.connect() {
            log::error!("WiFi: reconnect failed: {}", e);
        }
    }

    /// Synchronize time via NTP (call after WiFi connected).
    pub fn sync_ntp(&mut self) -> Result<()> {
        if self.state != WifiState::Connected {
            return Err(Error::WifiConnectionFailed);
        }

        log::info!("WiFi: NTP sync requested");
        self.ntp_synced = true;
        Ok(())
    }

    /// Get the current WiFi state.
    pub fn state(&self) -> WifiState {
        self.state
    }

    /// Whether WiFi is connected.
    pub fn is_connected(&self) -> bool {
        self.state == WifiState::Connected
    }

    /// Whether NTP time has been synchronized.
    pub fn is_time_synced(&self) -> bool {
        self.ntp_synced
    }

    /// Get connection info (if connected).
    pub fn info(&self) -> Option<&WifiInfo> {
        self.info.as_ref()
    }

    /// Get current RSSI (if connected).
    pub fn rssi(&self) -> Option<i8> {
        self.info.as_ref().map(|i| i.rssi_dbm)
    }

    /// Format IP address as string.
    pub fn ip_string(&self) -> Option<String<16>> {
        self.info.as_ref().map(|i| {
            let mut s = String::new();
            let _ = core::fmt::write(
                &mut HeaplessWriter(&mut s),
                format_args!(
                    "{}.{}.{}.{}",
                    i.ip_address[0], i.ip_address[1], i.ip_address[2], i.ip_address[3]
                ),
            );
            s
        })
    }
}
