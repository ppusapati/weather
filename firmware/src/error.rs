/// Unified error type for the weather station firmware.

use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    // Sensor errors
    SensorNotFound(SensorKind),
    SensorReadFailed(SensorKind),
    SensorOutOfRange(SensorKind),
    SensorDegraded(SensorKind),

    // Communication errors
    WifiConnectionFailed,
    WifiTimeout,
    MqttConnectionFailed,
    MqttPublishFailed,
    MqttSubscribeFailed,
    BleInitFailed,
    BleAdvertiseFailed,
    LoraInitFailed,
    LoraTxFailed,
    LoraRxTimeout,
    HttpServerError,

    // I2C / SPI bus errors
    I2cNack,
    I2cBusError,
    I2cTimeout,
    SpiBusError,
    SpiTimeout,

    // Storage errors
    FlashWriteFailed,
    FlashReadFailed,
    FlashFull,
    NvsError,

    // OTA errors
    OtaDownloadFailed,
    OtaVerifyFailed,
    OtaFlashFailed,

    // System errors
    OutOfMemory,
    WatchdogTimeout,
    InvalidConfig,
    CrcMismatch,

    // Sensor lifecycle
    SensorNotReady,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SensorKind {
    Bme280,
    WindSpeed,
    WindDirection,
    RainGauge,
    UvIndex,
    AmbientLight,
    Battery,
    Temperature,
    SoilMoisture,
    SoilTemperature,
    LeafWetness,
    Pyranometer,
    PanelTemperature,
    Pm25,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::SensorNotFound(s) => write!(f, "sensor not found: {:?}", s),
            Error::SensorReadFailed(s) => write!(f, "sensor read failed: {:?}", s),
            Error::SensorOutOfRange(s) => write!(f, "sensor out of range: {:?}", s),
            Error::SensorDegraded(s) => write!(f, "sensor degraded: {:?}", s),
            Error::WifiConnectionFailed => write!(f, "WiFi connection failed"),
            Error::WifiTimeout => write!(f, "WiFi timeout"),
            Error::MqttConnectionFailed => write!(f, "MQTT connection failed"),
            Error::MqttPublishFailed => write!(f, "MQTT publish failed"),
            Error::MqttSubscribeFailed => write!(f, "MQTT subscribe failed"),
            Error::BleInitFailed => write!(f, "BLE init failed"),
            Error::BleAdvertiseFailed => write!(f, "BLE advertise failed"),
            Error::LoraInitFailed => write!(f, "LoRa init failed"),
            Error::LoraTxFailed => write!(f, "LoRa TX failed"),
            Error::LoraRxTimeout => write!(f, "LoRa RX timeout"),
            Error::HttpServerError => write!(f, "HTTP server error"),
            Error::I2cNack => write!(f, "I2C NACK"),
            Error::I2cBusError => write!(f, "I2C bus error"),
            Error::I2cTimeout => write!(f, "I2C timeout"),
            Error::SpiBusError => write!(f, "SPI bus error"),
            Error::SpiTimeout => write!(f, "SPI timeout"),
            Error::FlashWriteFailed => write!(f, "flash write failed"),
            Error::FlashReadFailed => write!(f, "flash read failed"),
            Error::FlashFull => write!(f, "flash storage full"),
            Error::NvsError => write!(f, "NVS error"),
            Error::OtaDownloadFailed => write!(f, "OTA download failed"),
            Error::OtaVerifyFailed => write!(f, "OTA verify failed"),
            Error::OtaFlashFailed => write!(f, "OTA flash failed"),
            Error::OutOfMemory => write!(f, "out of memory"),
            Error::WatchdogTimeout => write!(f, "watchdog timeout"),
            Error::InvalidConfig => write!(f, "invalid configuration"),
            Error::CrcMismatch => write!(f, "CRC mismatch"),
            Error::SensorNotReady => write!(f, "sensor not ready"),
        }
    }
}

impl fmt::Display for SensorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SensorKind::Bme280 => write!(f, "BME280"),
            SensorKind::WindSpeed => write!(f, "Wind Speed"),
            SensorKind::WindDirection => write!(f, "Wind Direction"),
            SensorKind::RainGauge => write!(f, "Rain Gauge"),
            SensorKind::UvIndex => write!(f, "UV Index"),
            SensorKind::AmbientLight => write!(f, "Ambient Light"),
            SensorKind::Battery => write!(f, "Battery"),
            SensorKind::Temperature => write!(f, "Temperature"),
            SensorKind::SoilMoisture => write!(f, "Soil Moisture"),
            SensorKind::SoilTemperature => write!(f, "Soil Temperature"),
            SensorKind::LeafWetness => write!(f, "Leaf Wetness"),
            SensorKind::Pyranometer => write!(f, "Pyranometer"),
            SensorKind::PanelTemperature => write!(f, "Panel Temperature"),
            SensorKind::Pm25 => write!(f, "PM2.5"),
        }
    }
}

pub type Result<T> = core::result::Result<T, Error>;
