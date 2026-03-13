/// India regional weather analytics module.
///
/// Provides India-specific derived values and alerts:
/// - **Monsoon tracking** — onset/withdrawal detection, IMD-aligned season classification
/// - **Heat wave alerts** — IMD criteria (plains, coastal, hill) with Apparent Temperature
/// - **Tropical cyclone preparedness** — pressure-drop + wind surge detection
/// - **AQI estimation** — PM2.5-based air quality index (NAQI scale)
/// - **IST timezone** — UTC+05:30 day boundary for daily aggregations
/// - **Regional LoRa** — IN865 band (865–867 MHz) for Indian ISM band compliance

use crate::config;
use crate::industry::{AlertSeverity, IndustryAlert};
use serde::{Deserialize, Serialize};

// ──────────────────────────────────────────────────
// Data Structures
// ──────────────────────────────────────────────────

/// India-specific weather reading with regional analytics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IndiaReading {
    /// Current IMD season classification.
    pub season: ImdSeason,
    /// Monsoon phase (pre/active/retreat/off).
    pub monsoon_phase: MonsoonPhase,
    /// Consecutive days with rainfall ≥ 2.5 mm (monsoon onset indicator).
    pub consecutive_rain_days: u16,
    /// Accumulated monsoon rainfall this season (mm).
    pub monsoon_rainfall_mm: f32,
    /// Daily rainfall total (mm), reset at IST midnight.
    pub daily_rainfall_mm: f32,

    // Heat assessment
    /// Heat wave classification per IMD criteria.
    pub heat_wave: HeatWaveLevel,
    /// Apparent temperature (°C) — feels-like with humidity.
    pub apparent_temp_c: Option<f32>,
    /// Discomfort index (Thom, 0–100).
    pub discomfort_index: Option<f32>,

    // Cyclone indicators
    /// Cyclone risk level based on pressure drop + wind.
    pub cyclone_risk: CycloneRisk,
    /// 3-hour pressure tendency (hPa).
    pub pressure_tendency_3h_hpa: Option<f32>,

    // Air quality
    /// Estimated AQI (NAQI 0–500 scale) if PM2.5 sensor present.
    pub aqi_naqi: Option<u16>,
    /// AQI category string.
    pub aqi_category: AqiCategory,

    // Regional crops
    /// GDD accumulated for the configured Indian crop.
    pub gdd_accumulated: f32,
    /// Today's GDD contribution.
    pub gdd_today: f32,
}

/// IMD seasonal classification (India Meteorological Department).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ImdSeason {
    /// January–February.
    Winter,
    /// March–May.
    PreMonsoon,
    /// June–September.
    #[default]
    SouthwestMonsoon,
    /// October–December.
    PostMonsoon,
}

/// Monsoon phase tracking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum MonsoonPhase {
    /// Before monsoon onset.
    #[default]
    PreMonsoon,
    /// Monsoon has arrived — sustained rainfall.
    Active,
    /// Active break/dry spell within monsoon season.
    Break,
    /// Monsoon withdrawing.
    Retreat,
    /// Post-monsoon / dry season.
    Off,
}

/// Heat wave severity per IMD criteria.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum HeatWaveLevel {
    /// No heat wave.
    #[default]
    None,
    /// Temperature 4.5–6.4°C above normal or ≥ 40°C (plains).
    HeatWave,
    /// Temperature ≥ 6.5°C above normal or ≥ 45°C (plains).
    SevereHeatWave,
}

/// Tropical cyclone risk level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum CycloneRisk {
    #[default]
    None,
    /// Rapid pressure drop detected.
    Watch,
    /// Significant pressure drop + high wind.
    Warning,
    /// Extreme conditions — take shelter.
    Severe,
}

/// NAQI air quality categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AqiCategory {
    Good,
    Satisfactory,
    Moderate,
    Poor,
    VeryPoor,
    Severe,
    #[default]
    Unknown,
}

// ──────────────────────────────────────────────────
// Analytics Engine
// ──────────────────────────────────────────────────

/// India regional analytics engine.
pub struct IndiaAnalytics {
    // Monsoon tracking
    consecutive_rain_days: u16,
    monsoon_rainfall_mm: f32,
    daily_rainfall_mm: f32,
    current_phase: MonsoonPhase,

    // Heat tracking
    last_heat_wave: HeatWaveLevel,

    // Pressure history (3-hour window, sampled every 10 min = 18 slots)
    pressure_history: [f32; 18],
    pressure_idx: usize,
    pressure_count: u16,

    // Cyclone tracking
    last_cyclone_risk: CycloneRisk,

    // AQI alert deduplication
    last_aqi_severe: bool,

    // GDD
    gdd_accumulated: f32,
    today_temp_min_c: f32,
    today_temp_max_c: f32,
    today_has_readings: bool,
    gdd_base_temp_c: f32,

    // Day boundary tracking (IST = UTC+05:30)
    last_day_boundary_ms: u64,

    // Region type for heat wave thresholds
    region: IndiaRegion,
}

/// Indian geographic region for heat wave threshold selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum IndiaRegion {
    /// Plains (most of India): heat wave at 40°C.
    #[default]
    Plains,
    /// Coastal regions: heat wave at 37°C.
    Coastal,
    /// Hill stations: heat wave at 30°C.
    Hill,
}

impl IndiaAnalytics {
    pub fn new(gdd_base_temp_c: f32, region: IndiaRegion) -> Self {
        Self {
            consecutive_rain_days: 0,
            monsoon_rainfall_mm: 0.0,
            daily_rainfall_mm: 0.0,
            current_phase: MonsoonPhase::PreMonsoon,
            last_heat_wave: HeatWaveLevel::None,
            pressure_history: [0.0; 18],
            pressure_idx: 0,
            pressure_count: 0,
            last_cyclone_risk: CycloneRisk::None,
            last_aqi_severe: false,
            gdd_accumulated: 0.0,
            today_temp_min_c: f32::MAX,
            today_temp_max_c: f32::MIN,
            today_has_readings: false,
            gdd_base_temp_c,
            last_day_boundary_ms: 0,
            region,
        }
    }

    /// Process weather data and produce an IndiaReading.
    pub fn process(
        &mut self,
        air_temp_c: Option<f32>,
        humidity_pct: Option<f32>,
        wind_speed_kmh: Option<f32>,
        pressure_hpa: Option<f32>,
        rain_rate_mm_hr: Option<f32>,
        pm25_ugm3: Option<f32>,
        month: u8,
        uptime_ms: u64,
    ) -> IndiaReading {
        let mut reading = IndiaReading::default();

        // Sanitize inputs: reject NaN/infinity values
        let air_temp_c = air_temp_c.filter(|v| v.is_finite());
        let humidity_pct = humidity_pct.filter(|v| v.is_finite());
        let wind_speed_kmh = wind_speed_kmh.filter(|v| v.is_finite());
        let pressure_hpa = pressure_hpa.filter(|v| v.is_finite());
        let rain_rate_mm_hr = rain_rate_mm_hr.filter(|v| v.is_finite() && *v >= 0.0);
        let pm25_ugm3 = pm25_ugm3.filter(|v| v.is_finite() && *v >= 0.0);

        // Season classification
        reading.season = Self::classify_season(month);

        // Update daily min/max for GDD
        if let Some(temp) = air_temp_c {
            self.today_has_readings = true;
            if temp < self.today_temp_min_c {
                self.today_temp_min_c = temp;
            }
            if temp > self.today_temp_max_c {
                self.today_temp_max_c = temp;
            }
        }

        // Track rainfall
        if let Some(rate) = rain_rate_mm_hr {
            // Accumulate rainfall (called at ~10s intervals)
            let increment = rate * (config::INDIA_PROCESS_INTERVAL_MS as f32 / 3_600_000.0);
            self.daily_rainfall_mm += increment;
            if reading.season == ImdSeason::SouthwestMonsoon {
                self.monsoon_rainfall_mm += increment;
            }
        }
        reading.daily_rainfall_mm = self.daily_rainfall_mm;
        reading.monsoon_rainfall_mm = self.monsoon_rainfall_mm;

        // Monsoon phase
        reading.monsoon_phase = self.evaluate_monsoon_phase(month);
        reading.consecutive_rain_days = self.consecutive_rain_days;

        // Heat wave assessment
        reading.heat_wave = self.evaluate_heat_wave(air_temp_c);
        reading.apparent_temp_c = Self::compute_apparent_temp(air_temp_c, humidity_pct, wind_speed_kmh);
        reading.discomfort_index = Self::compute_discomfort_index(air_temp_c, humidity_pct);

        // Pressure tendency & cyclone risk
        if let Some(p) = pressure_hpa {
            self.record_pressure(p);
        }
        reading.pressure_tendency_3h_hpa = self.compute_pressure_tendency();
        reading.cyclone_risk = self.evaluate_cyclone_risk(
            reading.pressure_tendency_3h_hpa,
            wind_speed_kmh,
        );

        // AQI
        if let Some(pm25) = pm25_ugm3 {
            let (aqi, cat) = Self::compute_naqi(pm25);
            reading.aqi_naqi = Some(aqi);
            reading.aqi_category = cat;
        }

        // GDD
        reading.gdd_today = self.compute_daily_gdd();
        reading.gdd_accumulated = self.gdd_accumulated;

        reading
    }

    /// Generate alerts based on current conditions.
    pub fn check_alerts(
        &mut self,
        reading: &IndiaReading,
        timestamp_ms: u64,
    ) -> heapless::Vec<IndustryAlert, 4> {
        let mut alerts = heapless::Vec::new();

        // Heat wave alert (on state change)
        if reading.heat_wave != self.last_heat_wave {
            self.last_heat_wave = reading.heat_wave;
            match reading.heat_wave {
                HeatWaveLevel::HeatWave => {
                    let _ = alerts.push(IndustryAlert {
                        severity: AlertSeverity::Warning,
                        category: heapless::String::try_from("heat_wave").unwrap_or_default(),
                        message: heapless::String::try_from(
                            "IMD Heat Wave: extreme temperature conditions",
                        )
                        .unwrap_or_default(),
                        timestamp_ms,
                    });
                }
                HeatWaveLevel::SevereHeatWave => {
                    let _ = alerts.push(IndustryAlert {
                        severity: AlertSeverity::Critical,
                        category: heapless::String::try_from("heat_wave").unwrap_or_default(),
                        message: heapless::String::try_from(
                            "SEVERE HEAT WAVE: dangerous conditions, take precautions",
                        )
                        .unwrap_or_default(),
                        timestamp_ms,
                    });
                }
                _ => {}
            }
        }

        // Cyclone alert (on state change)
        if reading.cyclone_risk != self.last_cyclone_risk {
            self.last_cyclone_risk = reading.cyclone_risk;
            match reading.cyclone_risk {
                CycloneRisk::Warning => {
                    let _ = alerts.push(IndustryAlert {
                        severity: AlertSeverity::Warning,
                        category: heapless::String::try_from("cyclone").unwrap_or_default(),
                        message: heapless::String::try_from(
                            "Cyclone warning: rapid pressure drop with high winds",
                        )
                        .unwrap_or_default(),
                        timestamp_ms,
                    });
                }
                CycloneRisk::Severe => {
                    let _ = alerts.push(IndustryAlert {
                        severity: AlertSeverity::Critical,
                        category: heapless::String::try_from("cyclone").unwrap_or_default(),
                        message: heapless::String::try_from(
                            "CYCLONE ALERT: extreme pressure drop, take shelter immediately",
                        )
                        .unwrap_or_default(),
                        timestamp_ms,
                    });
                }
                _ => {}
            }
        }

        // AQI severe alert (on state change only — prevents alert flooding)
        let aqi_severe_now = reading.aqi_category == AqiCategory::Severe;
        if aqi_severe_now && !self.last_aqi_severe {
            let _ = alerts.push(IndustryAlert {
                severity: AlertSeverity::Critical,
                category: heapless::String::try_from("air_quality").unwrap_or_default(),
                message: heapless::String::try_from(
                    "Severe AQI: air quality hazardous, avoid outdoor exposure",
                )
                .unwrap_or_default(),
                timestamp_ms,
            });
        }
        self.last_aqi_severe = aqi_severe_now;

        // Monsoon onset detection
        if reading.monsoon_phase == MonsoonPhase::Active
            && self.current_phase == MonsoonPhase::PreMonsoon
        {
            let _ = alerts.push(IndustryAlert {
                severity: AlertSeverity::Info,
                category: heapless::String::try_from("monsoon").unwrap_or_default(),
                message: heapless::String::try_from(
                    "Monsoon onset detected: sustained rainfall pattern identified",
                )
                .unwrap_or_default(),
                timestamp_ms,
            });
        }

        self.current_phase = reading.monsoon_phase;
        alerts
    }

    /// End-of-day processing: finalize GDD, reset daily accumulators.
    /// Should be called at IST midnight (UTC+05:30).
    pub fn end_of_day(&mut self) {
        // Finalize GDD
        let daily_gdd = if self.today_has_readings {
            self.compute_daily_gdd()
        } else {
            0.0
        };
        self.gdd_accumulated += daily_gdd;

        // Track consecutive rain days for monsoon onset
        if self.daily_rainfall_mm >= config::INDIA_MONSOON_ONSET_RAIN_MM {
            self.consecutive_rain_days = self.consecutive_rain_days.saturating_add(1);
        } else {
            self.consecutive_rain_days = 0;
        }

        // Reset daily
        self.daily_rainfall_mm = 0.0;
        self.today_temp_min_c = f32::MAX;
        self.today_temp_max_c = f32::MIN;
        self.today_has_readings = false;

        log::info!(
            "India: day ended (IST), GDD today={:.1}, total={:.1}, rain_days={}",
            daily_gdd,
            self.gdd_accumulated,
            self.consecutive_rain_days
        );
    }

    /// Reset seasonal accumulators (start of Kharif/Rabi season).
    pub fn reset_season(&mut self) {
        self.gdd_accumulated = 0.0;
        self.monsoon_rainfall_mm = 0.0;
        self.consecutive_rain_days = 0;
        log::info!("India: seasonal accumulators reset");
    }

    // ── Private computation methods ──────────────────

    /// Classify IMD season by month.
    fn classify_season(month: u8) -> ImdSeason {
        match month {
            1 | 2 => ImdSeason::Winter,
            3..=5 => ImdSeason::PreMonsoon,
            6..=9 => ImdSeason::SouthwestMonsoon,
            10..=12 => ImdSeason::PostMonsoon,
            _ => ImdSeason::SouthwestMonsoon,
        }
    }

    /// Evaluate monsoon phase based on rainfall pattern and month.
    fn evaluate_monsoon_phase(&self, month: u8) -> MonsoonPhase {
        match month {
            // SW monsoon months
            6..=9 => {
                if self.consecutive_rain_days >= config::INDIA_MONSOON_ONSET_DAYS {
                    MonsoonPhase::Active
                } else if self.current_phase == MonsoonPhase::Active {
                    // Was active, now in a break
                    MonsoonPhase::Break
                } else {
                    MonsoonPhase::PreMonsoon
                }
            }
            // October: retreat phase
            10 => {
                if self.current_phase == MonsoonPhase::Active
                    || self.current_phase == MonsoonPhase::Break
                {
                    MonsoonPhase::Retreat
                } else {
                    MonsoonPhase::Off
                }
            }
            _ => MonsoonPhase::Off,
        }
    }

    /// Evaluate heat wave level per IMD criteria.
    ///
    /// IMD defines heat wave based on region:
    /// - Plains: ≥ 40°C (heat wave), ≥ 45°C (severe)
    /// - Coastal: ≥ 37°C (heat wave), ≥ 41°C (severe)
    /// - Hill: ≥ 30°C (heat wave), ≥ 34°C (severe)
    fn evaluate_heat_wave(&self, temp_c: Option<f32>) -> HeatWaveLevel {
        let temp = match temp_c {
            Some(t) => t,
            None => return HeatWaveLevel::None,
        };

        let (hw_threshold, severe_threshold) = match self.region {
            IndiaRegion::Plains => (
                config::INDIA_HEAT_WAVE_PLAINS_C,
                config::INDIA_SEVERE_HEAT_WAVE_PLAINS_C,
            ),
            IndiaRegion::Coastal => (
                config::INDIA_HEAT_WAVE_COASTAL_C,
                config::INDIA_SEVERE_HEAT_WAVE_COASTAL_C,
            ),
            IndiaRegion::Hill => (
                config::INDIA_HEAT_WAVE_HILL_C,
                config::INDIA_SEVERE_HEAT_WAVE_HILL_C,
            ),
        };

        if temp >= severe_threshold {
            HeatWaveLevel::SevereHeatWave
        } else if temp >= hw_threshold {
            HeatWaveLevel::HeatWave
        } else {
            HeatWaveLevel::None
        }
    }

    /// Compute apparent temperature (feels-like) using Steadman's model.
    fn compute_apparent_temp(
        temp_c: Option<f32>,
        humidity_pct: Option<f32>,
        wind_kmh: Option<f32>,
    ) -> Option<f32> {
        let t = temp_c?;
        let rh = humidity_pct?;
        let wind_ms = wind_kmh.unwrap_or(0.0) / 3.6;

        // Water vapor pressure (kPa)
        if (t + 237.3).abs() < 1.0 {
            return None;
        }
        let e = (rh / 100.0) * 0.6108 * libm::expf(17.27 * t / (t + 237.3));

        // Steadman apparent temperature
        let at = t + 0.33 * (e * 10.0) - 0.7 * wind_ms - 4.0;
        if at.is_finite() { Some(at) } else { None }
    }

    /// Compute Thom's discomfort index.
    /// DI = T - 0.55 * (1 - 0.01 * RH) * (T - 14.5)
    fn compute_discomfort_index(
        temp_c: Option<f32>,
        humidity_pct: Option<f32>,
    ) -> Option<f32> {
        let t = temp_c?;
        let rh = humidity_pct?;
        let di = t - 0.55 * (1.0 - 0.01 * rh) * (t - 14.5);
        if di.is_finite() { Some(di) } else { None }
    }

    /// Record pressure reading into circular history buffer.
    fn record_pressure(&mut self, pressure_hpa: f32) {
        if !pressure_hpa.is_finite() {
            return;
        }
        self.pressure_history[self.pressure_idx] = pressure_hpa;
        self.pressure_idx = (self.pressure_idx + 1) % self.pressure_history.len();
        if self.pressure_count < self.pressure_history.len() as u16 {
            self.pressure_count += 1;
        }
    }

    /// Compute 3-hour pressure tendency (newest - oldest in buffer).
    fn compute_pressure_tendency(&self) -> Option<f32> {
        if self.pressure_count < 2 {
            return None;
        }

        let newest_idx = if self.pressure_idx == 0 {
            self.pressure_history.len() - 1
        } else {
            self.pressure_idx - 1
        };

        let oldest_idx = if self.pressure_count >= self.pressure_history.len() as u16 {
            self.pressure_idx // wraps around to oldest
        } else {
            0
        };

        let tendency = self.pressure_history[newest_idx] - self.pressure_history[oldest_idx];
        Some(tendency)
    }

    /// Evaluate tropical cyclone risk from pressure tendency and wind.
    fn evaluate_cyclone_risk(
        &self,
        tendency: Option<f32>,
        wind_kmh: Option<f32>,
    ) -> CycloneRisk {
        let drop = match tendency {
            Some(t) => -t, // positive drop means pressure fell
            None => return CycloneRisk::None,
        };
        let wind = wind_kmh.unwrap_or(0.0);

        // Severe: ≥ 8 hPa drop in 3h with wind ≥ 90 km/h
        if drop >= config::INDIA_CYCLONE_SEVERE_DROP_HPA && wind >= 90.0 {
            CycloneRisk::Severe
        }
        // Warning: ≥ 5 hPa drop in 3h with wind ≥ 60 km/h
        else if drop >= config::INDIA_CYCLONE_WARNING_DROP_HPA && wind >= 60.0 {
            CycloneRisk::Warning
        }
        // Watch: ≥ 3 hPa drop in 3h
        else if drop >= config::INDIA_CYCLONE_WATCH_DROP_HPA {
            CycloneRisk::Watch
        } else {
            CycloneRisk::None
        }
    }

    /// Compute NAQI (National Air Quality Index) from PM2.5 concentration.
    ///
    /// CPCB breakpoints for PM2.5 (24-hr avg, µg/m³):
    /// Good:         0–30     → AQI 0–50
    /// Satisfactory: 31–60    → AQI 51–100
    /// Moderate:     61–90    → AQI 101–200
    /// Poor:         91–120   → AQI 201–300
    /// Very Poor:    121–250  → AQI 301–400
    /// Severe:       250+     → AQI 401–500
    fn compute_naqi(pm25: f32) -> (u16, AqiCategory) {
        if pm25 <= 30.0 {
            let aqi = Self::linear_interp(pm25, 0.0, 30.0, 0.0, 50.0);
            (aqi as u16, AqiCategory::Good)
        } else if pm25 <= 60.0 {
            let aqi = Self::linear_interp(pm25, 30.0, 60.0, 51.0, 100.0);
            (aqi as u16, AqiCategory::Satisfactory)
        } else if pm25 <= 90.0 {
            let aqi = Self::linear_interp(pm25, 60.0, 90.0, 101.0, 200.0);
            (aqi as u16, AqiCategory::Moderate)
        } else if pm25 <= 120.0 {
            let aqi = Self::linear_interp(pm25, 90.0, 120.0, 201.0, 300.0);
            (aqi as u16, AqiCategory::Poor)
        } else if pm25 <= 250.0 {
            let aqi = Self::linear_interp(pm25, 120.0, 250.0, 301.0, 400.0);
            (aqi as u16, AqiCategory::VeryPoor)
        } else {
            let aqi = Self::linear_interp(pm25, 250.0, 500.0, 401.0, 500.0).min(500.0);
            (aqi as u16, AqiCategory::Severe)
        }
    }

    /// Linear interpolation for AQI breakpoint calculation.
    fn linear_interp(value: f32, c_low: f32, c_high: f32, i_low: f32, i_high: f32) -> f32 {
        let denom = c_high - c_low;
        if denom.abs() < 0.001 {
            return i_low;
        }
        i_low + (value - c_low) * (i_high - i_low) / denom
    }

    /// Compute daily GDD using average of daily min/max.
    fn compute_daily_gdd(&self) -> f32 {
        if self.today_temp_min_c >= f32::MAX || self.today_temp_max_c <= f32::MIN {
            return 0.0;
        }
        let t_avg = (self.today_temp_min_c + self.today_temp_max_c) / 2.0;
        (t_avg - self.gdd_base_temp_c).max(0.0)
    }
}
