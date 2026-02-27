/// Power management module.
///
/// Manages operating modes (active, modem sleep, light sleep, deep sleep),
/// battery monitoring, and power-saving transitions.

use crate::config;
use crate::error::Result;

/// Power operating mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerMode {
    /// All systems active. ~160 mA.
    Active,
    /// WiFi in duty-cycle mode. ~20 mA.
    ModemSleep,
    /// CPU paused, RAM retained. ~0.8 mA.
    LightSleep,
    /// Full power-off, RTC only. ~10 µA.
    DeepSleep,
}

/// Battery status.
#[derive(Debug, Clone, Copy)]
pub struct BatteryStatus {
    /// Battery voltage in volts.
    pub voltage_v: f32,
    /// Battery percentage (0-100).
    pub percentage: u8,
    /// Whether the battery is charging (if detectable).
    pub charging: bool,
    /// Whether the battery is below low threshold.
    pub is_low: bool,
    /// Whether the battery is critically low.
    pub is_critical: bool,
}

/// Power manager.
pub struct PowerManager {
    mode: PowerMode,
    battery: BatteryStatus,
    idle_start_ms: Option<u64>,
    deep_sleep_requested: bool,
}

impl PowerManager {
    pub fn new() -> Self {
        Self {
            mode: PowerMode::Active,
            battery: BatteryStatus {
                voltage_v: 4.2,
                percentage: 100,
                charging: false,
                is_low: false,
                is_critical: false,
            },
            idle_start_ms: None,
            deep_sleep_requested: false,
        }
    }

    /// Update battery status from ADC reading.
    /// `adc_raw` is the raw 12-bit ADC value from the battery voltage divider.
    pub fn update_battery(&mut self, adc_raw: u16) {
        let adc_voltage = (adc_raw as f32 / config::ADC_RESOLUTION as f32) * config::ADC_VREF;
        let battery_voltage = adc_voltage * config::BATTERY_DIVIDER_RATIO;

        let percentage = voltage_to_percentage(battery_voltage);

        self.battery = BatteryStatus {
            voltage_v: battery_voltage,
            percentage,
            charging: false, // would need a charging pin to detect
            is_low: percentage <= config::BATTERY_LOW_THRESHOLD_PCT,
            is_critical: battery_voltage <= config::BATTERY_CRITICAL_V,
        };

        if self.battery.is_critical {
            log::error!(
                "CRITICAL: battery voltage {:.2}V — initiating shutdown",
                battery_voltage
            );
            self.deep_sleep_requested = true;
        } else if self.battery.is_low {
            log::warn!(
                "Battery low: {:.2}V ({}%)",
                battery_voltage,
                percentage
            );
        }
    }

    /// Get current battery status.
    pub fn battery(&self) -> &BatteryStatus {
        &self.battery
    }

    /// Get current power mode.
    pub fn mode(&self) -> PowerMode {
        self.mode
    }

    /// Signal that the system is idle (no pending work).
    /// `now_ms` is the current system time.
    pub fn signal_idle(&mut self, now_ms: u64) {
        if self.idle_start_ms.is_none() {
            self.idle_start_ms = Some(now_ms);
        }
    }

    /// Signal that the system is busy (work pending).
    pub fn signal_busy(&mut self) {
        self.idle_start_ms = None;
        if self.mode != PowerMode::Active {
            self.transition_to(PowerMode::Active);
        }
    }

    /// Evaluate whether to transition to a lower power mode.
    /// Call this periodically from the main loop.
    pub fn evaluate_sleep(&mut self, now_ms: u64, time_until_next_task_ms: u64) {
        // Check if deep sleep requested (critical battery)
        if self.deep_sleep_requested {
            self.transition_to(PowerMode::DeepSleep);
            return;
        }

        // Check idle duration
        let idle_duration = self
            .idle_start_ms
            .map(|start| now_ms.saturating_sub(start))
            .unwrap_or(0);

        // Decision tree for sleep mode
        if self.battery.is_low && idle_duration > 5000 {
            // Low battery: prefer light sleep
            self.transition_to(PowerMode::LightSleep);
        } else if time_until_next_task_ms > 30_000 && idle_duration > 30_000 {
            // Long idle with no imminent tasks: light sleep
            self.transition_to(PowerMode::LightSleep);
        } else if time_until_next_task_ms > 5000 && idle_duration > 5000 {
            // Moderate idle: modem sleep
            self.transition_to(PowerMode::ModemSleep);
        }
    }

    /// Request deep sleep for a specific duration.
    pub fn request_deep_sleep(&mut self, duration_s: u64) {
        log::info!("Power: deep sleep requested for {}s", duration_s);
        self.deep_sleep_requested = true;
    }

    /// Transition to a new power mode.
    fn transition_to(&mut self, new_mode: PowerMode) {
        if self.mode == new_mode {
            return;
        }

        log::info!("Power: {:?} -> {:?}", self.mode, new_mode);

        match new_mode {
            PowerMode::Active => {
                // In real firmware: restore full clock, enable WiFi
            }
            PowerMode::ModemSleep => {
                // In real firmware: WiFi goes to duty-cycle mode
            }
            PowerMode::LightSleep => {
                // In real firmware: CPU paused, wake on timer or GPIO
                // esp_hal::sleep::light_sleep(wake_sources)
            }
            PowerMode::DeepSleep => {
                // In real firmware: configure RTC wake timer, enter deep sleep
                // This will not return — device reboots on wake.
                log::info!(
                    "Power: entering deep sleep for {}s",
                    config::DEEP_SLEEP_DURATION_S
                );
                // esp_hal::sleep::deep_sleep(duration)
            }
        }

        self.mode = new_mode;
    }

    /// Whether deep sleep has been requested.
    pub fn deep_sleep_pending(&self) -> bool {
        self.deep_sleep_requested
    }
}

/// Convert battery voltage to percentage using a LiPo discharge curve.
fn voltage_to_percentage(voltage: f32) -> u8 {
    // Simplified LiPo discharge curve approximation
    let pct = if voltage >= 4.2 {
        100.0
    } else if voltage >= 4.06 {
        90.0 + (voltage - 4.06) / (4.2 - 4.06) * 10.0
    } else if voltage >= 3.98 {
        80.0 + (voltage - 3.98) / (4.06 - 3.98) * 10.0
    } else if voltage >= 3.92 {
        70.0 + (voltage - 3.92) / (3.98 - 3.92) * 10.0
    } else if voltage >= 3.87 {
        60.0 + (voltage - 3.87) / (3.92 - 3.87) * 10.0
    } else if voltage >= 3.82 {
        50.0 + (voltage - 3.82) / (3.87 - 3.82) * 10.0
    } else if voltage >= 3.79 {
        40.0 + (voltage - 3.79) / (3.82 - 3.79) * 10.0
    } else if voltage >= 3.77 {
        30.0 + (voltage - 3.77) / (3.79 - 3.77) * 10.0
    } else if voltage >= 3.74 {
        20.0 + (voltage - 3.74) / (3.77 - 3.74) * 10.0
    } else if voltage >= 3.68 {
        10.0 + (voltage - 3.68) / (3.74 - 3.68) * 10.0
    } else if voltage >= 3.45 {
        (voltage - 3.45) / (3.68 - 3.45) * 10.0
    } else {
        0.0
    };

    pct.clamp(0.0, 100.0) as u8
}
