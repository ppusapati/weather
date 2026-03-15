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
/// # Architecture (Hybrid Mode)
///
/// ```text
/// ┌─────────────────────────┐     USART3 (921.6k)     ┌────────────────────────┐
/// │     STM32F407VGT6       │◄══════════════════════════►│    ESP32-S3-WROOM-1   │
/// │  (Sensor Acquisition)   │    Bridge Protocol         │  (Cloud Connectivity)  │
/// │                         │                            │                        │
/// │  • I2C sensor bus       │                            │  • WiFi 802.11n        │
/// │  • ADC channels         │                            │  • BLE 5.0             │
/// │  • Modbus RS485 slave   │                            │  • MQTT client         │
/// │  • Data pipeline        │                            │  • HTTP REST API       │
/// │  • Alert engine         │                            │  • LoRa SX1276         │
/// │  • Watchdog (IWDG)      │                            │  • OTA updates         │
/// │                         │                            │  • NTP time sync       │
/// └─────────┬───────────────┘                            └────────────────────────┘
///           │
///           │ USART2 (RS485)
///           │
///     ┌─────┴─────┐
///     │  MAX3485   │
///     │ RS485 Xcvr │
///     └─────┬─────┘
///           │
///     ══════╧══════  RS485 Bus (A/B differential pair)
///           │
///     ┌─────┴─────┐
///     │  SCADA/PLC │
///     │  Master    │
///     └───────────┘
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

    /// Whether at least one communication channel is healthy.
    pub fn any_channel_healthy(&self) -> bool {
        match self.mode {
            OperatingMode::Cloud => self.cloud_healthy,
            OperatingMode::Scada => self.scada_healthy,
            OperatingMode::Hybrid => self.cloud_healthy || self.scada_healthy,
        }
    }

    /// Get mode diagnostics for status reporting.
    pub fn diagnostics(&self) -> ModeDiagnostics {
        ModeDiagnostics {
            mode: self.mode,
            cloud_healthy: self.cloud_healthy,
            scada_healthy: self.scada_healthy,
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
    pub cloud_failovers: u32,
    pub scada_recoveries: u32,
    pub mode_changed_at_ms: u64,
}
