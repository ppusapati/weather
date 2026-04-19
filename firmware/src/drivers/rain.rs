/// Rain gauge driver — Tipping bucket with reed switch.
///
/// Each tip of the bucket generates a pulse on the GPIO interrupt pin.
/// Each tip corresponds to 0.2 mm of rainfall.

use crate::config;
use core::sync::atomic::{AtomicU32, Ordering};

/// Global tip counter incremented by GPIO ISR.
static RAIN_TIP_COUNT: AtomicU32 = AtomicU32::new(0);

/// Call this from the GPIO ISR for the rain gauge pin.
pub fn rain_isr_handler() {
    RAIN_TIP_COUNT.fetch_add(1, Ordering::Relaxed);
}

/// Rain gauge reading.
#[derive(Debug, Clone, Copy)]
pub struct RainReading {
    /// Total accumulated rainfall in mm since last reset.
    pub total_mm: f32,
    /// Rainfall rate in mm/hour (computed from recent tips).
    pub rate_mm_hr: f32,
    /// Number of tips since last read.
    pub tips_since_last: u32,
    /// Total tip count since boot.
    pub total_tips: u32,
}

/// Rain gauge driver.
pub struct RainGauge {
    last_tip_count: u32,
    last_read_ms: u32,
    total_tips_at_reset: u32,
    /// Debounce: minimum ms between valid tips
    debounce_ms: u32,
}

impl RainGauge {
    pub fn new() -> Self {
        Self {
            last_tip_count: 0,
            last_read_ms: 0,
            total_tips_at_reset: 0,
            debounce_ms: 100,
        }
    }

    /// Initialize the rain gauge. Resets the tip counter.
    pub fn init(&mut self) {
        RAIN_TIP_COUNT.store(0, Ordering::Relaxed);
        self.last_tip_count = 0;
        self.total_tips_at_reset = 0;
        log::info!("Rain gauge initialized on GPIO {}", config::RAIN_GAUGE_PIN);
    }

    /// Read rainfall data. `current_time_ms` is the current system time.
    pub fn read(&mut self, current_time_ms: u32) -> RainReading {
        let current_tips = RAIN_TIP_COUNT.load(Ordering::Relaxed);
        let tips_since_last = current_tips.wrapping_sub(self.last_tip_count);
        let period_ms = current_time_ms.wrapping_sub(self.last_read_ms);

        // Calculate rate in mm/hour
        let rate_mm_hr = if period_ms > 0 {
            let tips_per_ms = tips_since_last as f32 / period_ms as f32;
            tips_per_ms * config::RAIN_MM_PER_TIP * 3_600_000.0
        } else {
            0.0
        };

        let total_tips = current_tips.wrapping_sub(self.total_tips_at_reset);
        let total_mm = total_tips as f32 * config::RAIN_MM_PER_TIP;

        self.last_tip_count = current_tips;
        self.last_read_ms = current_time_ms;

        RainReading {
            total_mm,
            rate_mm_hr,
            tips_since_last,
            total_tips,
        }
    }

    /// Reset the daily accumulation counter.
    pub fn reset_accumulation(&mut self) {
        self.total_tips_at_reset = RAIN_TIP_COUNT.load(Ordering::Relaxed);
    }

    /// Get total tips since boot (for diagnostics).
    pub fn total_tips_since_boot(&self) -> u32 {
        RAIN_TIP_COUNT.load(Ordering::Relaxed)
    }
}
