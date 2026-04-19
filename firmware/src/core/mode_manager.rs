/// Operating mode manager for SCADA / Cloud / Hybrid operation.
///
/// Controls how the weather station communicates its data based on
/// the configured operating mode:
///
/// | Mode | Cloud (MQTT/HTTP) | SCADA (Modbus RS485) |
/// |------|:-----------------:|:--------------------:|
/// | Cloud | Yes | — |
/// | SCADA | — | Yes |
/// | Hybrid | Yes | Yes |
///
/// # Architecture (Single-MCU)
///
/// ```text
/// ┌─────────────────────────────────────────────────────────────────┐
/// │                   STM32F407VGT6 (Single MCU)                    │
/// │            ARM Cortex-M4F @ 168 MHz, -40/+105°C                 │
/// ├─────────────────────────────────────────────────────────────────┤
/// │ SPI1  ── SX1276 LoRa (PA4-PA7, PC4, PC5)                       │
/// │ SPI2  ── W5500 Ethernet (PB12-PB15, PD3, PD4) [optional]       │
/// │ SPI3  ── ATWINC1500 WiFi (PB3-PB5, PE3-PE6)                    │
/// │ USART2 ── MAX3485 RS485 Modbus (PA2, PA3, PA1)                 │
/// │ USART3 ── RN4870 BLE (PB10, PB11, PD8, PD9)                   │
/// │ UART4 ── SIM7600E-H Cellular (PC10, PC11, PD5-PD7) [optional]  │
/// │ USART1 ── Debug Console (PA9, PA10)                             │
/// │ I2C1  ── Sensors: BME280, AS5600, SI1145, BH1750 (PB6, PB7)   │
/// │ ADC1  ── Battery, Wind, PM2.5, Soil (PA0, PC0-PC3)             │
/// │ TIM3  ── Wind/Rain pulse (PB0, PB1)                            │
/// │ GPIO  ── LEDs (PD0-PD2), DIP switch (PE0-PE1)                  │
/// └──────────────┬──────────────────────────────────────────────────┘
///                │
///                │ USART2 (RS485)
///                │
///          ┌─────┴─────┐
///          │  MAX3485   │
///          │ RS485 Xcvr │
///          └─────┬─────┘
///                │
///          ══════╧══════  RS485 Bus (A/B differential pair)
///                │
///          ┌─────┴─────┐
///          │  SCADA/PLC │
///          │  Master    │
///          └───────────┘
/// ```
///
/// # Mode Selection
///
/// The operating mode is determined at boot by:
/// 1. DIP switch on PE0/PE1 (hardware override)
/// 2. NVS-stored configuration (software default)
/// 3. Modbus holding register HR_OP_MODE (runtime change)

use crate::drivers::stm32f407::OperatingMode;
use serde::{Deserialize, Serialize};

/// Mode manager state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeManager {
    /// Current operating mode.
    mode: OperatingMode,
    /// Whether cloud subsystem is initialized and healthy.
    cloud_healthy: bool,
    /// Whether SCADA subsystem is initialized and healthy.
    scada_healthy: bool,
    /// Transport health flags.
    wifi_healthy: bool,
    ethernet_healthy: bool,
    cellular_healthy: bool,
    ble_healthy: bool,
    /// Cloud failover count (times cloud fell back to SCADA-only).
    cloud_failover_count: u32,
    /// SCADA failover count (times SCADA recovered from error).
    scada_recovery_count: u32,
    /// Uptime when mode was last changed (ms).
    mode_changed_at_ms: u64,
}

impl ModeManager {
    pub fn new(mode: OperatingMode) -> Self {
        Self {
            mode,
            cloud_healthy: false,
            scada_healthy: false,
            wifi_healthy: false,
            ethernet_healthy: false,
            cellular_healthy: false,
            ble_healthy: false,
            cloud_failover_count: 0,
            scada_recovery_count: 0,
            mode_changed_at_ms: 0,
        }
    }

    /// Get the current operating mode.
    pub fn mode(&self) -> OperatingMode {
        self.mode
    }

    /// Set the operating mode (e.g., from Modbus write or cloud command).
    pub fn set_mode(&mut self, mode: OperatingMode, now_ms: u64) {
        if self.mode != mode {
            log::info!(
                "Operating mode changed: {} → {}",
                self.mode.as_str(),
                mode.as_str()
            );
            self.mode = mode;
            self.mode_changed_at_ms = now_ms;
        }
    }

    /// Whether cloud communication should be active.
    pub fn should_run_cloud(&self) -> bool {
        self.mode.cloud_enabled()
    }

    /// Whether SCADA/Modbus communication should be active.
    pub fn should_run_scada(&self) -> bool {
        self.mode.scada_enabled()
    }

    /// Report cloud subsystem health.
    pub fn report_cloud_health(&mut self, healthy: bool) {
        if self.cloud_healthy && !healthy {
            self.cloud_failover_count += 1;
            log::warn!(
                "Cloud subsystem unhealthy (failover #{})",
                self.cloud_failover_count
            );
        }
        self.cloud_healthy = healthy;
    }

    /// Report SCADA subsystem health.
    pub fn report_scada_health(&mut self, healthy: bool) {
        if !self.scada_healthy && healthy {
            self.scada_recovery_count += 1;
            log::info!(
                "SCADA subsystem recovered (recovery #{})",
                self.scada_recovery_count
            );
        }
        self.scada_healthy = healthy;
    }

    /// Report transport health for a specific channel.
    pub fn report_wifi_health(&mut self, healthy: bool) {
        self.wifi_healthy = healthy;
        // WiFi health implies cloud health when in cloud/hybrid mode
        if self.mode.cloud_enabled() {
            self.report_cloud_health(self.wifi_healthy || self.ethernet_healthy || self.cellular_healthy);
        }
    }

    /// Report Ethernet transport health.
    pub fn report_ethernet_health(&mut self, healthy: bool) {
        self.ethernet_healthy = healthy;
        if self.mode.cloud_enabled() {
            self.report_cloud_health(self.wifi_healthy || self.ethernet_healthy || self.cellular_healthy);
        }
    }

    /// Report cellular transport health.
    pub fn report_cellular_health(&mut self, healthy: bool) {
        self.cellular_healthy = healthy;
        if self.mode.cloud_enabled() {
            self.report_cloud_health(self.wifi_healthy || self.ethernet_healthy || self.cellular_healthy);
        }
    }

    /// Report BLE transport health.
    pub fn report_ble_health(&mut self, healthy: bool) {
        self.ble_healthy = healthy;
    }

    /// Return the best available cloud transport.
    pub fn best_cloud_transport(&self) -> Option<&'static str> {
        if self.ethernet_healthy {
            Some("ethernet")
        } else if self.wifi_healthy {
            Some("wifi")
        } else if self.cellular_healthy {
            Some("cellular")
        } else {
            None
        }
    }

    /// Whether at least one communication channel is healthy.
    pub fn any_channel_healthy(&self) -> bool {
        match self.mode {
            OperatingMode::Cloud => self.wifi_healthy || self.ethernet_healthy || self.cellular_healthy,
            OperatingMode::Scada => self.scada_healthy,
            OperatingMode::Hybrid => {
                self.scada_healthy || self.wifi_healthy || self.ethernet_healthy || self.cellular_healthy
            }
        }
    }

    /// Get mode diagnostics for status reporting.
    pub fn diagnostics(&self) -> ModeDiagnostics {
        ModeDiagnostics {
            mode: self.mode,
            cloud_healthy: self.cloud_healthy,
            scada_healthy: self.scada_healthy,
            wifi_healthy: self.wifi_healthy,
            ethernet_healthy: self.ethernet_healthy,
            cellular_healthy: self.cellular_healthy,
            ble_healthy: self.ble_healthy,
            cloud_failovers: self.cloud_failover_count,
            scada_recoveries: self.scada_recovery_count,
            mode_changed_at_ms: self.mode_changed_at_ms,
        }
    }
}

/// Diagnostics snapshot for status reporting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeDiagnostics {
    pub mode: OperatingMode,
    pub cloud_healthy: bool,
    pub scada_healthy: bool,
    pub wifi_healthy: bool,
    pub ethernet_healthy: bool,
    pub cellular_healthy: bool,
    pub ble_healthy: bool,
    pub cloud_failovers: u32,
    pub scada_recoveries: u32,
    pub mode_changed_at_ms: u64,
}
