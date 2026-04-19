/// Agriculture industry analytics module.
///
/// Provides crop-focused derived values and alerts:
/// - **Evapotranspiration (ET₀)** — FAO Penman-Monteith reference ET
/// - **Growing Degree Days (GDD)** — Accumulated thermal units for crop staging
/// - **Frost alerts** — Temperature-based frost/freeze warnings
/// - **Irrigation scheduling** — Soil moisture depletion-based recommendations
/// - **Disease risk indices** — Leaf wetness duration + temperature models
/// - **Spray window detection** — Wind + rain conditions for pesticide application

use crate::config;
use crate::industry::{AlertSeverity, IndustryAlert};
use serde::{Deserialize, Serialize};

// ──────────────────────────────────────────────────
// Data Structures
// ──────────────────────────────────────────────────

/// Agriculture-specific sensor readings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgricultureReading {
    /// Shallow soil moisture (%).
    pub soil_moisture_shallow_pct: Option<f32>,
    /// Deep soil moisture (%).
    pub soil_moisture_deep_pct: Option<f32>,
    /// Soil temperature at probe depth (°C).
    pub soil_temp_c: Option<f32>,
    /// Leaf wetness percentage (0=dry, 100=saturated).
    pub leaf_wetness_pct: Option<f32>,
    /// Leaf wetness duration (minutes in current wet period).
    pub leaf_wet_duration_min: u32,

    // Derived values
    /// Reference evapotranspiration (mm/day).
    pub et0_mm_day: Option<f32>,
    /// Accumulated Growing Degree Days this season.
    pub gdd_accumulated: f32,
    /// Today's GDD contribution.
    pub gdd_today: f32,
    /// Frost risk level.
    pub frost_risk: FrostRisk,
    /// Irrigation recommendation.
    pub irrigation: IrrigationStatus,
    /// Disease risk index (0–100).
    pub disease_risk_index: u8,
    /// Spray window assessment.
    pub spray_window: SprayWindow,
}

/// Frost risk classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum FrostRisk {
    #[default]
    None,
    /// Temperature approaching frost threshold.
    Watch,
    /// Frost likely within hours.
    Warning,
    /// Freezing conditions active.
    Critical,
}

/// Irrigation scheduling recommendation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum IrrigationStatus {
    #[default]
    Adequate,
    /// Soil moisture dropping, plan irrigation.
    MonitorClosely,
    /// Soil moisture at management allowable depletion.
    IrrigateRecommended,
    /// Soil moisture at or below wilting point.
    IrrigateCritical,
    /// Soil is at or above field capacity.
    FieldCapacity,
}

/// Spray window assessment for pesticide/herbicide application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SprayWindow {
    #[default]
    Unknown,
    /// Conditions suitable for spraying.
    Good,
    /// Marginal conditions — proceed with caution.
    Marginal,
    /// Do not spray — wind, rain, or inversion.
    Poor,
}

// ──────────────────────────────────────────────────
// Analytics Engine
// ──────────────────────────────────────────────────

/// Agriculture analytics engine.
pub struct AgricultureAnalytics {
    /// Accumulated GDD for the growing season.
    gdd_accumulated: f32,
    /// Today's min/max for GDD calculation.
    today_temp_min_c: f32,
    today_temp_max_c: f32,
    /// Whether any temperature readings were received today.
    today_has_readings: bool,
    /// Base temperature for GDD (crop-specific).
    gdd_base_temp_c: f32,
    /// Consecutive hours of leaf wetness (for disease models).
    leaf_wet_hours: f32,
    /// Last frost alert sent (to avoid duplicates).
    last_frost_alert: FrostRisk,
    /// Last irrigation alert sent (to avoid duplicates).
    last_irrigation_alert: IrrigationStatus,
    /// Last disease alert state (to avoid duplicates).
    last_disease_alert_sent: bool,
}

impl AgricultureAnalytics {
    pub fn new(gdd_base_temp_c: f32) -> Self {
        Self {
            gdd_accumulated: 0.0,
            today_temp_min_c: f32::MAX,
            today_temp_max_c: f32::MIN,
            today_has_readings: false,
            gdd_base_temp_c,
            leaf_wet_hours: 0.0,
            last_frost_alert: FrostRisk::None,
            last_irrigation_alert: IrrigationStatus::Adequate,
            last_disease_alert_sent: false,
        }
    }

    /// Process weather + agriculture sensor data and produce an AgricultureReading.
    pub fn process(
        &mut self,
        air_temp_c: Option<f32>,
        humidity_pct: Option<f32>,
        wind_speed_kmh: Option<f32>,
        pressure_hpa: Option<f32>,
        rain_rate_mm_hr: Option<f32>,
        solar_radiation_w_m2: Option<f32>,
        soil_moisture_shallow: Option<f32>,
        soil_moisture_deep: Option<f32>,
        soil_temp: Option<f32>,
        leaf_wetness_pct: Option<f32>,
        leaf_wet_duration_min: u32,
    ) -> AgricultureReading {
        let mut reading = AgricultureReading {
            soil_moisture_shallow_pct: soil_moisture_shallow,
            soil_moisture_deep_pct: soil_moisture_deep,
            soil_temp_c: soil_temp,
            leaf_wetness_pct,
            leaf_wet_duration_min,
            ..AgricultureReading::default()
        };

        // Update daily min/max
        if let Some(temp) = air_temp_c {
            self.today_has_readings = true;
            if temp < self.today_temp_min_c {
                self.today_temp_min_c = temp;
            }
            if temp > self.today_temp_max_c {
                self.today_temp_max_c = temp;
            }
        }

        // Evapotranspiration (simplified Penman-Monteith)
        reading.et0_mm_day = self.compute_et0(
            air_temp_c,
            humidity_pct,
            wind_speed_kmh,
            solar_radiation_w_m2,
            pressure_hpa,
        );

        // Growing Degree Days
        reading.gdd_today = self.compute_daily_gdd();
        reading.gdd_accumulated = self.gdd_accumulated;

        // Frost risk
        reading.frost_risk = self.evaluate_frost_risk(air_temp_c, humidity_pct);

        // Irrigation
        reading.irrigation = self.evaluate_irrigation(soil_moisture_shallow);

        // Disease risk
        self.leaf_wet_hours = leaf_wet_duration_min as f32 / 60.0;
        reading.disease_risk_index = self.compute_disease_risk(air_temp_c);

        // Spray window
        reading.spray_window = self.evaluate_spray_window(
            wind_speed_kmh,
            rain_rate_mm_hr,
            air_temp_c,
        );

        reading
    }

    /// Generate alerts based on current conditions.
    pub fn check_alerts(
        &mut self,
        reading: &AgricultureReading,
        timestamp_ms: u64,
    ) -> heapless::Vec<IndustryAlert, 4> {
        let mut alerts = heapless::Vec::new();

        // Frost alert (only send on state change)
        if reading.frost_risk != self.last_frost_alert {
            self.last_frost_alert = reading.frost_risk;
            match reading.frost_risk {
                FrostRisk::Warning => {
                    let _ = alerts.push(IndustryAlert {
                        severity: AlertSeverity::Warning,
                        category: heapless::String::try_from("frost").unwrap_or_default(),
                        message: heapless::String::try_from(
                            "Frost watch: temperature approaching freezing",
                        )
                        .unwrap_or_default(),
                        timestamp_ms,
                    });
                }
                FrostRisk::Critical => {
                    let _ = alerts.push(IndustryAlert {
                        severity: AlertSeverity::Critical,
                        category: heapless::String::try_from("frost").unwrap_or_default(),
                        message: heapless::String::try_from(
                            "FROST ALERT: freezing conditions detected",
                        )
                        .unwrap_or_default(),
                        timestamp_ms,
                    });
                }
                _ => {}
            }
        }

        // Irrigation alert (only on state change to avoid flooding)
        if reading.irrigation != self.last_irrigation_alert {
            self.last_irrigation_alert = reading.irrigation;
            if reading.irrigation == IrrigationStatus::IrrigateCritical {
                let _ = alerts.push(IndustryAlert {
                    severity: AlertSeverity::Critical,
                    category: heapless::String::try_from("irrigation").unwrap_or_default(),
                    message: heapless::String::try_from(
                        "Critical: soil moisture at wilting point, irrigate immediately",
                    )
                    .unwrap_or_default(),
                    timestamp_ms,
                });
            }
        }

        // Disease risk alert (only on transition to high risk)
        let disease_high = reading.disease_risk_index >= 80;
        if disease_high && !self.last_disease_alert_sent {
            self.last_disease_alert_sent = true;
            let _ = alerts.push(IndustryAlert {
                severity: AlertSeverity::Warning,
                category: heapless::String::try_from("disease").unwrap_or_default(),
                message: heapless::String::try_from(
                    "High disease risk: extended leaf wetness + warm temperatures",
                )
                .unwrap_or_default(),
                timestamp_ms,
            });
        } else if !disease_high {
            self.last_disease_alert_sent = false;
        }

        alerts
    }

    /// End-of-day: finalize GDD and reset daily min/max.
    pub fn end_of_day(&mut self) {
        let daily_gdd = if self.today_has_readings {
            self.compute_daily_gdd()
        } else {
            0.0
        };
        self.gdd_accumulated += daily_gdd;
        self.today_temp_min_c = f32::MAX;
        self.today_temp_max_c = f32::MIN;
        self.today_has_readings = false;
        log::info!(
            "Agriculture: day ended, GDD today={:.1}, total={:.1}",
            daily_gdd,
            self.gdd_accumulated
        );
    }

    /// Reset GDD accumulator (start of growing season).
    pub fn reset_season(&mut self) {
        self.gdd_accumulated = 0.0;
        log::info!("Agriculture: growing season reset");
    }

    // ── Private computation methods ──────────────────

    /// Simplified FAO Penman-Monteith ET₀ (mm/day).
    ///
    /// Full equation requires net radiation, soil heat flux, and
    /// psychrometric constant. This simplified version uses
    /// Hargreaves-Samani when full inputs are unavailable.
    fn compute_et0(
        &self,
        temp_c: Option<f32>,
        humidity_pct: Option<f32>,
        wind_kmh: Option<f32>,
        radiation_w_m2: Option<f32>,
        pressure_hpa: Option<f32>,
    ) -> Option<f32> {
        let t = temp_c?;
        let rh = humidity_pct?;

        // Guard: Tetens formula denominator (t + 237.3) must not be zero
        if (t + 237.3).abs() < 1.0 {
            return None;
        }

        // Saturation vapor pressure (kPa) — Tetens formula
        let es = 0.6108 * libm::expf(17.27 * t / (t + 237.3));
        // Actual vapor pressure
        let ea = es * (rh / 100.0);
        // Vapor pressure deficit
        let vpd = es - ea;

        if let Some(rs) = radiation_w_m2 {
            // Full Penman-Monteith (simplified)
            let wind_m_s = wind_kmh.unwrap_or(2.0) / 3.6;
            let pressure = pressure_hpa.unwrap_or(1013.25) / 10.0; // hPa to kPa
            let gamma = config::ET_PSYCHROMETRIC_CONST * pressure / 100.0;

            // Slope of saturation vapor pressure curve
            let denom_sq = (t + 237.3) * (t + 237.3);
            let delta = 4098.0 * es / denom_sq;

            // Net radiation (MJ/m²/day) — rough conversion from instantaneous W/m²
            let rn = rs * 0.0864 * 0.77; // ×0.0864 for daily, ×0.77 net ratio

            // Guard: (t + 273) must not be zero for temperature term
            if (t + 273.0).abs() < 0.5 {
                return None;
            }

            let numerator = 0.408 * delta * rn + gamma * (900.0 / (t + 273.0)) * wind_m_s * vpd;
            let denominator = delta + gamma * (1.0 + 0.34 * wind_m_s);

            if denominator.abs() < 0.0001 {
                None
            } else {
                Some((numerator / denominator).max(0.0))
            }
        } else {
            // Hargreaves-Samani (temperature-only fallback)
            if self.today_temp_max_c > self.today_temp_min_c {
                let t_range = self.today_temp_max_c - self.today_temp_min_c;
                let t_mean = (self.today_temp_max_c + self.today_temp_min_c) / 2.0;
                // Extraterrestrial radiation estimate (≈ 15 MJ/m²/day mid-latitude)
                let ra = 15.0;
                let et0 = 0.0023 * ra * libm::sqrtf(t_range) * (t_mean + 17.8);
                Some(et0.max(0.0))
            } else {
                None
            }
        }
    }

    /// Compute daily GDD using average of daily min/max.
    fn compute_daily_gdd(&self) -> f32 {
        if self.today_temp_min_c >= f32::MAX || self.today_temp_max_c <= f32::MIN {
            return 0.0;
        }
        let t_avg = (self.today_temp_min_c + self.today_temp_max_c) / 2.0;
        (t_avg - self.gdd_base_temp_c).max(0.0)
    }

    /// Evaluate frost risk from current temperature and humidity.
    fn evaluate_frost_risk(
        &self,
        temp_c: Option<f32>,
        humidity_pct: Option<f32>,
    ) -> FrostRisk {
        let temp = match temp_c {
            Some(t) => t,
            None => return FrostRisk::None,
        };

        // Low humidity increases radiative cooling risk
        let humidity_factor = match humidity_pct {
            Some(h) if h < 50.0 => 1.5, // dry air → faster radiative frost
            _ => 0.0,
        };

        let effective_temp = temp - humidity_factor;

        if effective_temp <= config::FROST_CRITICAL_THRESHOLD_C {
            FrostRisk::Critical
        } else if effective_temp <= config::FROST_ALERT_THRESHOLD_C {
            FrostRisk::Warning
        } else if effective_temp <= config::FROST_ALERT_THRESHOLD_C + 3.0 {
            FrostRisk::Watch
        } else {
            FrostRisk::None
        }
    }

    /// Evaluate irrigation need based on soil moisture.
    fn evaluate_irrigation(&self, soil_moisture_pct: Option<f32>) -> IrrigationStatus {
        let moisture = match soil_moisture_pct {
            Some(m) => m,
            None => return IrrigationStatus::Adequate,
        };

        if moisture >= config::SOIL_FIELD_CAPACITY_PCT {
            IrrigationStatus::FieldCapacity
        } else if moisture <= config::SOIL_WILTING_POINT_PCT {
            IrrigationStatus::IrrigateCritical
        } else if moisture <= config::SOIL_WILTING_POINT_PCT
            + (config::SOIL_FIELD_CAPACITY_PCT - config::SOIL_WILTING_POINT_PCT) * 0.4
        {
            IrrigationStatus::IrrigateRecommended
        } else if moisture
            <= config::SOIL_WILTING_POINT_PCT
                + (config::SOIL_FIELD_CAPACITY_PCT - config::SOIL_WILTING_POINT_PCT) * 0.6
        {
            IrrigationStatus::MonitorClosely
        } else {
            IrrigationStatus::Adequate
        }
    }

    /// Compute disease risk index (0–100) using leaf wetness duration × temperature.
    ///
    /// Based on simplified Smith period model for fungal disease prediction.
    /// Risk increases with prolonged leaf wetness at temperatures 15–25°C.
    fn compute_disease_risk(&self, temp_c: Option<f32>) -> u8 {
        let temp = match temp_c {
            Some(t) => t,
            None => return 0,
        };

        // Temperature suitability for fungal growth (bell curve around 20°C)
        let temp_factor = if temp >= 10.0 && temp <= 30.0 {
            let deviation = (temp - 20.0).abs();
            1.0 - (deviation / 10.0)
        } else {
            0.0
        };

        // Wetness duration factor (risk escalates after 6 hours)
        let wetness_factor = if self.leaf_wet_hours > 12.0 {
            1.0
        } else if self.leaf_wet_hours > 6.0 {
            (self.leaf_wet_hours - 6.0) / 6.0
        } else {
            0.0
        };

        let risk = (temp_factor * wetness_factor * 100.0).clamp(0.0, 100.0);
        // Round to avoid truncation discontinuities (99.9 → 100, not 99)
        libm::roundf(risk) as u8
    }

    /// Evaluate spray window suitability.
    fn evaluate_spray_window(
        &self,
        wind_kmh: Option<f32>,
        rain_rate: Option<f32>,
        temp_c: Option<f32>,
    ) -> SprayWindow {
        // Rain active → no spray
        if let Some(rain) = rain_rate {
            if rain > 0.5 {
                return SprayWindow::Poor;
            }
        }

        // Wind check
        let wind = wind_kmh.unwrap_or(0.0);
        if wind > 20.0 {
            return SprayWindow::Poor;
        }

        // Temperature inversion risk (very calm + cold)
        if let Some(temp) = temp_c {
            if temp < 5.0 && wind < 3.0 {
                return SprayWindow::Poor; // inversion likely
            }
        }

        if wind > 12.0 {
            SprayWindow::Marginal
        } else {
            SprayWindow::Good
        }
    }
}
