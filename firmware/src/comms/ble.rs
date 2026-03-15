/// BLE GATT server wrapping the RN4870 module on USART3.
///
/// Implements:
/// - Environmental Sensing service (0x181A): temperature, humidity, pressure
/// - Custom Weather service (0xFFE0): wind, rain, UV, light
/// - Configuration service (0xFFE1): WiFi provisioning, sampling rate
///
/// Delegates all hardware operations to the RN4870 driver.

use crate::core::data_pipeline::WeatherReading;
use crate::drivers::rn4870::{BleModuleState, Rn4870};
use crate::error::{Error, Result};
use heapless::String;

/// BLE connection state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BleState {
    Uninitialized,
    Advertising,
    Connected,
}

/// BLE GATT service UUIDs.
pub mod uuid {
    /// Environmental Sensing Service (Bluetooth SIG).
    pub const ENV_SENSING_SERVICE: u16 = 0x181A;
    /// Temperature characteristic.
    pub const TEMPERATURE: u16 = 0x2A6E;
    /// Humidity characteristic.
    pub const HUMIDITY: u16 = 0x2A6F;
    /// Pressure characteristic.
    pub const PRESSURE: u16 = 0x2A6D;

    /// Custom Weather Service.
    pub const WEATHER_SERVICE: u128 = 0x0000FFE0_0000_1000_8000_00805F9B34FB;
    /// Custom Configuration Service.
    pub const CONFIG_SERVICE: u128 = 0x0000FFE1_0000_1000_8000_00805F9B34FB;
}

/// WiFi provisioning data received via BLE.
#[derive(Debug, Clone)]
pub struct WifiProvisionData {
    pub ssid: String<32>,
    pub password: String<64>,
}

/// BLE GATT server wrapping the RN4870 driver.
pub struct BleServer {
    driver: Rn4870,
    state: BleState,
    device_name: String<32>,
    /// Latest reading cached for GATT reads.
    cached_reading: Option<WeatherReading>,
    /// Pending WiFi provisioning data from BLE write.
    pending_wifi_config: Option<WifiProvisionData>,
    /// Whether any central has subscribed to notifications.
    notifications_enabled: bool,
}

impl BleServer {
    pub fn new(device_name: &str) -> Self {
        Self {
            driver: Rn4870::new(),
            state: BleState::Uninitialized,
            device_name: String::try_from(device_name).unwrap_or_default(),
            cached_reading: None,
            pending_wifi_config: None,
            notifications_enabled: false,
        }
    }

    /// Initialize the BLE stack and register GATT services via RN4870.
    pub fn init(&mut self) -> Result<()> {
        log::info!("BLE: initializing RN4870 GATT server");

        // Initialize the RN4870 module (reset, configure, start advertising)
        self.driver.init();

        // Register GATT services via RN4870 AT commands:
        // 1. Environmental Sensing service (0x181A)
        //    - Temperature (0x2A6E): Read, Notify
        //    - Humidity (0x2A6F): Read, Notify
        //    - Pressure (0x2A6D): Read, Notify
        // 2. Custom Weather service (0xFFE0)
        //    - Wind Speed, Wind Dir, Rain, UV, Light: Read, Notify
        // 3. Configuration service (0xFFE1)
        //    - WiFi SSID: Write
        //    - WiFi Password: Write
        //    - Device Name: Read, Write

        self.state = BleState::Advertising;
        log::info!("BLE: RN4870 advertising as '{}'", self.device_name.as_str());
        Ok(())
    }

    /// Poll the RN4870 for events. Call from the scheduler's PollBle task.
    pub fn poll(&mut self, now_ms: u64) {
        self.driver.poll(now_ms);

        // Synchronize our state with the driver's state
        match self.driver.module_state() {
            BleModuleState::Connected => {
                if self.state != BleState::Connected {
                    self.on_connect();
                }
            }
            BleModuleState::Advertising => {
                if self.state == BleState::Connected {
                    self.on_disconnect();
                }
            }
            _ => {}
        }

        // Check for incoming GATT write data from the driver
        if let Some((handle, data)) = self.driver.take_gatt_write() {
            self.handle_write(handle, &data);
        }
    }

    /// Start advertising.
    pub fn start_advertising(&mut self) -> Result<()> {
        if self.state == BleState::Uninitialized {
            return Err(Error::BleInitFailed);
        }
        self.driver.start_advertising();
        self.state = BleState::Advertising;
        log::info!("BLE: advertising started");
        Ok(())
    }

    /// Update cached reading and send notifications if subscribed.
    pub fn update_reading(&mut self, reading: &WeatherReading) -> Result<()> {
        self.cached_reading = Some(reading.clone());

        if self.state == BleState::Connected && self.notifications_enabled {
            // Update weather data on the RN4870 module via SHW commands
            self.driver.update_weather_data(reading);
            self.send_notifications(reading)?;
        }

        Ok(())
    }

    /// Send GATT notifications for all subscribed characteristics.
    fn send_notifications(&self, reading: &WeatherReading) -> Result<()> {
        // Environmental Sensing notifications
        if let Some(temp) = reading.temperature_c {
            let value = (temp * 100.0) as i16;
            log::debug!("BLE: notify temperature = {}", value);
            // RN4870: SHW,<temp_handle>,<hex value>
        }

        if let Some(humidity) = reading.humidity_pct {
            let value = (humidity * 100.0) as u16;
            log::debug!("BLE: notify humidity = {}", value);
        }

        if let Some(pressure) = reading.pressure_hpa {
            let value = (pressure * 1000.0) as u32; // 0.1 Pa resolution
            log::debug!("BLE: notify pressure = {}", value);
        }

        // Custom weather notifications
        if let Some(wind_speed) = reading.wind_speed_kmh {
            let value = (wind_speed * 10.0) as u16;
            log::debug!("BLE: notify wind_speed = {}", value);
        }

        if let Some(wind_dir) = reading.wind_dir_deg {
            log::debug!("BLE: notify wind_dir = {}", wind_dir);
        }

        if let Some(uv) = reading.uv_index {
            let value = (uv * 10.0) as u16;
            log::debug!("BLE: notify uv_index = {}", value);
        }

        if let Some(light) = reading.light_lux {
            let value = light as u32;
            log::debug!("BLE: notify light = {}", value);
        }

        Ok(())
    }

    /// Handle a GATT write event from RN4870.
    pub fn handle_write(&mut self, char_uuid: u16, data: &[u8]) {
        // WiFi SSID write (0xFFE1-0010)
        if char_uuid == 0x0010 {
            if let Ok(ssid) = core::str::from_utf8(data) {
                log::info!("BLE: WiFi SSID set to '{}'", ssid);
                let provision = self.pending_wifi_config.get_or_insert(WifiProvisionData {
                    ssid: String::new(),
                    password: String::new(),
                });
                provision.ssid = String::try_from(ssid).unwrap_or_default();
            }
        }

        // WiFi Password write (0xFFE1-0011)
        if char_uuid == 0x0011 {
            if let Ok(password) = core::str::from_utf8(data) {
                log::info!("BLE: WiFi password set (len={})", password.len());
                let provision = self.pending_wifi_config.get_or_insert(WifiProvisionData {
                    ssid: String::new(),
                    password: String::new(),
                });
                provision.password = String::try_from(password).unwrap_or_default();
            }
        }
    }

    /// Take pending WiFi provisioning data (if both SSID and password set).
    pub fn take_wifi_config(&mut self) -> Option<WifiProvisionData> {
        if let Some(ref config) = self.pending_wifi_config {
            if !config.ssid.is_empty() && !config.password.is_empty() {
                return self.pending_wifi_config.take();
            }
        }
        None
    }

    /// Handle connection event.
    pub fn on_connect(&mut self) {
        self.state = BleState::Connected;
        log::info!("BLE: central connected");
    }

    /// Handle disconnection event.
    pub fn on_disconnect(&mut self) {
        self.state = BleState::Advertising;
        self.notifications_enabled = false;
        log::info!("BLE: central disconnected, resuming advertising");
    }

    /// Handle notification subscription change.
    pub fn on_notify_enable(&mut self, enabled: bool) {
        self.notifications_enabled = enabled;
        log::info!("BLE: notifications {}", if enabled { "enabled" } else { "disabled" });
    }

    pub fn state(&self) -> BleState {
        self.state
    }

    pub fn is_connected(&self) -> bool {
        self.state == BleState::Connected
    }
}
