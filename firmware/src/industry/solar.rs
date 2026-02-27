/// Solar energy industry analytics module.
///
/// Provides photovoltaic (PV) performance monitoring and analytics:
/// - **Solar irradiance tracking** — GHI measurement and integration
/// - **Panel temperature monitoring** — Front/back of module temperatures
/// - **Yield estimation** — Expected vs actual energy production
/// - **Performance ratio** — Actual vs theoretical output comparison
/// - **Peak sun hours** — Daily equivalent sun hours at 1000 W/m²
/// - **Temperature derating** — Power loss from panel overheating
/// - **Soiling loss estimation** — Dust accumulation tracking
/// - **Cloud transient detection** — Rapid irradiance fluctuation alerting

use crate::config;
use crate::industry::{AlertSeverity, IndustryAlert};
use serde::{Deserialize, Serialize};

// ──────────────────────────────────────────────────
// Data Structures
// ──────────────────────────────────────────────────

/// Solar energy-specific reading.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SolarReading {
    /// Global Horizontal Irradiance (W/m²).
    pub irradiance_w_m2: Option<f32>,
    /// Panel front surface temperature (°C).
    pub panel_temp_front_c: Option<f32>,
    /// Panel back surface temperature (°C).
    pub panel_temp_back_c: Option<f32>,

    // Derived analytics
    /// Estimated instantaneous power output (watts).
    pub estimated_power_w: Option<f32>,
    /// Daily energy yield so far (watt-hours).
    pub daily_yield_wh: f32,
    /// Accumulated peak sun hours today.
    pub peak_sun_hours: f32,
    /// Temperature derating factor (0.0–1.0, 1.0 = no loss).
    pub temp_derating_factor: Option<f32>,
    /// Estimated soiling loss (%).
    pub soiling_loss_pct: f32,
    /// Performance ratio (actual vs theoretical, 0.0–1.0).
    pub performance_ratio: Option<f32>,
    /// Sky condition.
    pub sky_condition: SkyCondition,
    /// Inverter efficiency estimate (0.0–1.0).
    pub inverter_efficiency: f32,
}

/// Sky condition derived from irradiance patterns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SkyCondition {
    #[default]
    Unknown,
    Clear,
    PartlyCloudy,
    Overcast,
    Night,
}

/// Daily summary for solar performance reporting.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DailySolarSummary {
    /// Total energy yield (watt-hours).
    pub total_yield_wh: f32,
    /// Peak power observed (watts).
    pub peak_power_w: f32,
    /// Peak irradiance observed (W/m²).
    pub peak_irradiance_w_m2: f32,
    /// Total peak sun hours.
    pub peak_sun_hours: f32,
    /// Average performance ratio.
    pub avg_performance_ratio: f32,
    /// Maximum panel temperature (°C).
    pub max_panel_temp_c: f32,
    /// Energy lost to temperature derating (Wh).
    pub temp_loss_wh: f32,
    /// Energy lost to soiling (Wh).
    pub soiling_loss_wh: f32,
    /// Number of cloud transient events.
    pub cloud_transients: u16,
}

// ──────────────────────────────────────────────────
// Analytics Engine
// ──────────────────────────────────────────────────

/// Solar energy analytics engine.
pub struct SolarAnalytics {
    /// Panel rated power at STC (watts-peak).
    panel_wp: f32,
    /// Panel area (m²).
    panel_area_m2: f32,

    /// Running daily energy accumulator (Wh).
    daily_yield_wh: f32,
    /// Running peak sun hours accumulator.
    peak_sun_hours: f32,
    /// Days since last rain (for soiling model).
    days_since_rain: u16,
    /// Daily summary accumulator.
    daily_summary: DailySolarSummary,
    /// Previous irradiance sample (for transient detection).
    prev_irradiance: Option<f32>,
    /// Timestamp of last sample (ms).
    last_sample_ms: u64,
    /// Performance ratio running sum and count (for daily average).
    pr_sum: f32,
    pr_count: u32,
    /// Inverter efficiency (configurable, default 0.96).
    inverter_efficiency: f32,
}

impl SolarAnalytics {
    pub fn new(panel_wp: f32, panel_area_m2: f32) -> Self {
        Self {
            panel_wp,
            panel_area_m2,
            daily_yield_wh: 0.0,
            peak_sun_hours: 0.0,
            days_since_rain: 0,
            daily_summary: DailySolarSummary::default(),
            prev_irradiance: None,
            last_sample_ms: 0,
            pr_sum: 0.0,
            pr_count: 0,
            inverter_efficiency: 0.96,
        }
    }

    /// Process sensor readings and produce a SolarReading with analytics.
    pub fn process(
        &mut self,
        irradiance_w_m2: Option<f32>,
        panel_temp_front_c: Option<f32>,
        panel_temp_back_c: Option<f32>,
        ambient_temp_c: Option<f32>,
        now_ms: u64,
    ) -> SolarReading {
        let mut reading = SolarReading {
            irradiance_w_m2,
            panel_temp_front_c,
            panel_temp_back_c,
            inverter_efficiency: self.inverter_efficiency,
            ..SolarReading::default()
        };

        let dt_hours = if self.last_sample_ms > 0 && now_ms > self.last_sample_ms {
            (now_ms - self.last_sample_ms) as f32 / 3_600_000.0
        } else {
            0.0
        };
        self.last_sample_ms = now_ms;

        // Temperature derating
        let panel_temp = panel_temp_front_c.or(panel_temp_back_c).or(ambient_temp_c);
        reading.temp_derating_factor = panel_temp.map(|t| self.compute_temp_derating(t));

        // Soiling loss
        reading.soiling_loss_pct = self.compute_soiling_loss();

        // Estimated power
        if let Some(irr) = irradiance_w_m2 {
            let derating = reading.temp_derating_factor.unwrap_or(1.0);
            let soiling = 1.0 - (reading.soiling_loss_pct / 100.0);
            let power = self.panel_wp
                * (irr / config::STC_IRRADIANCE_W_M2)
                * derating
                * soiling
                * self.inverter_efficiency;
            reading.estimated_power_w = Some(power.max(0.0));

            // Accumulate energy
            if dt_hours > 0.0 {
                let energy_wh = power.max(0.0) * dt_hours;
                self.daily_yield_wh += energy_wh;
            }

            // Peak sun hours: integrate irradiance / 1000
            if dt_hours > 0.0 {
                self.peak_sun_hours += irr / config::PEAK_SUN_HOUR_THRESHOLD_W_M2 * dt_hours;
            }

            // Performance ratio
            if irr > 50.0 {
                // Only compute PR when there's meaningful irradiance
                let theoretical_power = self.panel_wp * (irr / config::STC_IRRADIANCE_W_M2);
                if theoretical_power > 0.0 {
                    let pr = power / theoretical_power;
                    reading.performance_ratio = Some(pr.clamp(0.0, 1.2));
                    self.pr_sum += pr;
                    self.pr_count += 1;
                }
            }

            // Update daily summary peaks
            if irr > self.daily_summary.peak_irradiance_w_m2 {
                self.daily_summary.peak_irradiance_w_m2 = irr;
            }
            if let Some(p) = reading.estimated_power_w {
                if p > self.daily_summary.peak_power_w {
                    self.daily_summary.peak_power_w = p;
                }
            }

            // Cloud transient detection
            self.detect_cloud_transient(irr);

            // Sky condition
            reading.sky_condition = self.classify_sky(irr);
        } else {
            reading.sky_condition = SkyCondition::Night;
        }

        // Track max panel temp
        if let Some(pt) = panel_temp {
            if pt > self.daily_summary.max_panel_temp_c {
                self.daily_summary.max_panel_temp_c = pt;
            }
        }

        reading.daily_yield_wh = self.daily_yield_wh;
        reading.peak_sun_hours = self.peak_sun_hours;

        reading
    }

    /// Signal rain event (resets soiling accumulator).
    pub fn rain_detected(&mut self) {
        self.days_since_rain = 0;
        log::info!("Solar: rain detected, soiling counter reset");
    }

    /// End-of-day: finalize daily summary and reset accumulators.
    pub fn end_of_day(&mut self) -> DailySolarSummary {
        self.daily_summary.total_yield_wh = self.daily_yield_wh;
        self.daily_summary.peak_sun_hours = self.peak_sun_hours;
        self.daily_summary.avg_performance_ratio = if self.pr_count > 0 {
            self.pr_sum / self.pr_count as f32
        } else {
            0.0
        };
        self.daily_summary.soiling_loss_wh =
            self.daily_yield_wh * (self.compute_soiling_loss() / 100.0);

        let summary = self.daily_summary.clone();

        // Reset daily accumulators
        self.daily_yield_wh = 0.0;
        self.peak_sun_hours = 0.0;
        self.pr_sum = 0.0;
        self.pr_count = 0;
        self.daily_summary = DailySolarSummary::default();
        self.prev_irradiance = None;

        // Increment soiling day counter
        if self.days_since_rain < config::SOILING_MAX_DAYS {
            self.days_since_rain += 1;
        }

        log::info!(
            "Solar: day ended, yield={:.0}Wh, PSH={:.2}, PR={:.1}%",
            summary.total_yield_wh,
            summary.peak_sun_hours,
            summary.avg_performance_ratio * 100.0
        );

        summary
    }

    /// Generate alerts based on current solar conditions.
    pub fn check_alerts(
        &self,
        reading: &SolarReading,
        timestamp_ms: u64,
    ) -> heapless::Vec<IndustryAlert, 4> {
        let mut alerts = heapless::Vec::new();

        // Panel overheating alert
        if let Some(temp) = reading.panel_temp_front_c {
            if temp > 75.0 {
                let _ = alerts.push(IndustryAlert {
                    severity: AlertSeverity::Warning,
                    category: heapless::String::try_from("panel_temp").unwrap_or_default(),
                    message: heapless::String::try_from(
                        "Panel temperature exceeds 75C — significant power derating",
                    )
                    .unwrap_or_default(),
                    timestamp_ms,
                });
            }
        }

        // Low performance ratio
        if let Some(pr) = reading.performance_ratio {
            if pr < 0.5 && reading.irradiance_w_m2.unwrap_or(0.0) > 200.0 {
                let _ = alerts.push(IndustryAlert {
                    severity: AlertSeverity::Warning,
                    category: heapless::String::try_from("performance").unwrap_or_default(),
                    message: heapless::String::try_from(
                        "Low performance ratio (<50%) — check for shading or soiling",
                    )
                    .unwrap_or_default(),
                    timestamp_ms,
                });
            }
        }

        // High soiling loss
        if reading.soiling_loss_pct > 2.0 {
            let _ = alerts.push(IndustryAlert {
                severity: AlertSeverity::Info,
                category: heapless::String::try_from("soiling").unwrap_or_default(),
                message: heapless::String::try_from(
                    "Estimated soiling loss >2% — panel cleaning recommended",
                )
                .unwrap_or_default(),
                timestamp_ms,
            });
        }

        alerts
    }

    // ── Private computation methods ──────────────────

    /// Compute temperature derating factor.
    /// At STC (25°C), factor = 1.0. Above STC, power decreases by temp coefficient.
    fn compute_temp_derating(&self, panel_temp_c: f32) -> f32 {
        let delta_t = panel_temp_c - config::STC_TEMP_C;
        if delta_t <= 0.0 {
            1.0 // No derating below STC
        } else {
            let loss_pct = delta_t * config::PANEL_TEMP_COEFF_PCT_PER_C.abs();
            (1.0 - loss_pct / 100.0).max(0.0)
        }
    }

    /// Estimate soiling loss based on days since rain.
    fn compute_soiling_loss(&self) -> f32 {
        (self.days_since_rain as f32 * config::SOILING_LOSS_PCT_PER_DAY)
            .min(config::SOILING_MAX_DAYS as f32 * config::SOILING_LOSS_PCT_PER_DAY)
    }

    /// Detect rapid irradiance changes (cloud transients).
    fn detect_cloud_transient(&mut self, current_irradiance: f32) {
        if let Some(prev) = self.prev_irradiance {
            let delta = (current_irradiance - prev).abs();
            // > 200 W/m² change between samples = cloud transient
            if delta > 200.0 && prev > 100.0 {
                self.daily_summary.cloud_transients += 1;
                log::debug!(
                    "Solar: cloud transient detected, delta={:.0} W/m²",
                    delta
                );
            }
        }
        self.prev_irradiance = Some(current_irradiance);
    }

    /// Classify sky condition from absolute irradiance.
    fn classify_sky(&self, irradiance: f32) -> SkyCondition {
        if irradiance < 10.0 {
            SkyCondition::Night
        } else if irradiance < config::CLOUD_COVER_THRESHOLD_W_M2 {
            SkyCondition::Overcast
        } else if irradiance < 600.0 {
            SkyCondition::PartlyCloudy
        } else {
            SkyCondition::Clear
        }
    }
}
