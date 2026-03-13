/// Firmware configuration constants and runtime config.
///
/// All hardware pin assignments, I2C addresses, timing intervals,
/// calibration constants, and validation ranges are defined here
/// as named constants. Runtime-configurable values are stored in
/// [`RuntimeConfig`] and persisted in NVS flash.
///
/// # Industrial-Grade Component Selection
///
/// All components are selected for the industrial temperature range
/// (-40°C to +85°C) and harsh outdoor environments:
///
/// | Component | Industrial Part | Temp Range | Notes |
/// |-----------|----------------|------------|-------|
/// | MCU | ESP32-S3-WROOM-1-N16R8**I** | -40°C to +85°C | Industrial suffix "I" |
/// | Temp/Hum/Pressure | BME280 (Bosch) | -40°C to +85°C | Automotive-qualified available |
/// | Wind Direction | AS5600-ASOM | -40°C to +85°C | Industrial magnetic encoder |
/// | UV Sensor | SI1145-A10-GMR | -40°C to +85°C | Industrial UV/ALS/proximity |
/// | Light Sensor | BH1750FVI-TR | -40°C to +85°C | Industrial ALS |
/// | PM2.5 Sensor | GP2Y1014AU0F | -10°C to +65°C | Upgraded from GP2Y1010AU0F |
/// | LoRa Transceiver | SX1276 (Semtech) | -40°C to +85°C | Industrial ISM radio |
/// | Soil Temp | DS18B20**Z** | -55°C to +125°C | Industrial 1-Wire |
/// | Soil Moisture | Capacitive (SEN0193) | -40°C to +85°C | No corrosion |
/// | Voltage Regulator | TPS63020 | -40°C to +85°C | Wide input buck-boost |
/// | TVS/ESD Protection | TPD4E05U06 | -40°C to +125°C | On all external lines |
/// | Conformal Coating | Dow Corning 1-2577 | -65°C to +200°C | Moisture/salt spray |
///
/// IP65/IP67-rated enclosure with UV-stabilized polycarbonate recommended.

use serde::{Deserialize, Serialize};

// ---------- Hardware Pin Assignments ----------

/// I2C bus pins
pub const I2C_SDA_PIN: u8 = 8;
pub const I2C_SCL_PIN: u8 = 9;
pub const I2C_FREQ_HZ: u32 = 400_000;

/// SPI bus pins (LoRa SX1276)
pub const SPI_MOSI_PIN: u8 = 10;
pub const SPI_MISO_PIN: u8 = 11;
pub const SPI_SCK_PIN: u8 = 12;
pub const LORA_CS_PIN: u8 = 13;
pub const LORA_RST_PIN: u8 = 14;
pub const LORA_DIO0_PIN: u8 = 15;

/// GPIO interrupt pins
pub const WIND_SPEED_PIN: u8 = 4;
pub const RAIN_GAUGE_PIN: u8 = 5;

/// ADC pins
pub const BATTERY_ADC_PIN: u8 = 6;

/// UART pins
pub const UART_TX_PIN: u8 = 43;
pub const UART_RX_PIN: u8 = 44;
pub const UART_BAUD_RATE: u32 = 115_200;

/// Status LED
pub const LED_PIN: u8 = 2;

// ---------- I2C Addresses ----------

pub const BME280_ADDR: u8 = 0x76;
pub const AS5600_ADDR: u8 = 0x36;
pub const SI1145_ADDR: u8 = 0x60;
pub const BH1750_ADDR: u8 = 0x23;

// ---------- Sensor Calibration ----------

/// Wind speed: pulses per revolution
pub const WIND_PULSES_PER_REV: f32 = 1.0;
/// Wind speed factor: km/h per pulse per second
pub const WIND_SPEED_FACTOR: f32 = 2.4;
/// Rain gauge: mm per tip
pub const RAIN_MM_PER_TIP: f32 = 0.2;

// ---------- Sensor Validation Ranges ----------

pub const TEMP_MIN_C: f32 = -40.0;
pub const TEMP_MAX_C: f32 = 85.0;
pub const HUMIDITY_MIN_PCT: f32 = 0.0;
pub const HUMIDITY_MAX_PCT: f32 = 100.0;
pub const PRESSURE_MIN_HPA: f32 = 300.0;
pub const PRESSURE_MAX_HPA: f32 = 1100.0;
pub const WIND_SPEED_MAX_KMH: f32 = 200.0;
pub const RAIN_MAX_MM_HR: f32 = 500.0;
pub const UV_INDEX_MAX: f32 = 15.0;
pub const LIGHT_MAX_LUX: f32 = 120_000.0;

// ---------- EMA Filter Coefficients ----------

pub const EMA_ALPHA_TEMPERATURE: f32 = 0.3;
pub const EMA_ALPHA_HUMIDITY: f32 = 0.3;
pub const EMA_ALPHA_PRESSURE: f32 = 0.2;
pub const EMA_ALPHA_WIND_SPEED: f32 = 0.7;
pub const EMA_ALPHA_WIND_DIR: f32 = 0.5;
pub const EMA_ALPHA_UV: f32 = 0.4;
pub const EMA_ALPHA_LIGHT: f32 = 0.4;

// ---------- Stuck Sensor Detection ----------

pub const STUCK_SENSOR_THRESHOLD: u32 = 10;

// ---------- Power Management ----------

pub const BATTERY_FULL_V: f32 = 4.2;
pub const BATTERY_EMPTY_V: f32 = 3.0;
pub const BATTERY_LOW_THRESHOLD_PCT: u8 = 20;
pub const BATTERY_CRITICAL_V: f32 = 3.3;
pub const ADC_VREF: f32 = 3.3;
pub const ADC_RESOLUTION: u16 = 4095;
pub const BATTERY_DIVIDER_RATIO: f32 = 2.0;

// ---------- LoRa Configuration ----------

pub const LORA_FREQUENCY_HZ: u32 = 868_000_000; // EU868
pub const LORA_SPREADING_FACTOR: u8 = 7;
pub const LORA_BANDWIDTH_HZ: u32 = 125_000;
pub const LORA_TX_POWER_DBM: i8 = 14;
pub const LORA_CODING_RATE: u8 = 5; // 4/5

// ---------- Timing ----------

pub const SENSOR_READ_INTERVAL_MS: u64 = 10_000;
pub const WIND_READ_INTERVAL_MS: u64 = 5_000;
pub const UV_LIGHT_READ_INTERVAL_MS: u64 = 30_000;
pub const MQTT_PUBLISH_INTERVAL_MS: u64 = 30_000;
pub const STATUS_REPORT_INTERVAL_MS: u64 = 60_000;
pub const OTA_CHECK_INTERVAL_MS: u64 = 6 * 3600 * 1000;
pub const WATCHDOG_TIMEOUT_MS: u64 = 30_000;
pub const WIFI_CONNECT_TIMEOUT_MS: u64 = 15_000;
pub const WIFI_RETRY_MAX: u8 = 5;
pub const DEEP_SLEEP_DURATION_S: u64 = 300;

// ---------- MQTT Defaults ----------

pub const MQTT_PORT_DEFAULT: u16 = 1883;
pub const MQTT_TLS_PORT_DEFAULT: u16 = 8883;
pub const MQTT_KEEPALIVE_S: u16 = 60;
pub const MQTT_BUFFER_SIZE: usize = 1000;

// ---------- HTTP ----------

pub const HTTP_PORT: u16 = 80;
pub const HTTP_MAX_CONNECTIONS: usize = 4;

// ---------- Flash Storage ----------

pub const FLASH_DATA_START: u32 = 0x00819000;
pub const FLASH_DATA_SIZE: u32 = 0x007E7000; // ~7.9 MB
pub const FLASH_RECORD_SIZE: usize = 64;

/// Minimum interval between flash writes (ms) to limit wear.
pub const FLASH_WRITE_MIN_INTERVAL_MS: u64 = 30_000;

// ---------- Ring Buffer ----------

pub const READING_BUFFER_CAPACITY: usize = 64;

// ---------- Heap ----------

/// Heap allocator size in bytes (384 KB).
pub const HEAP_SIZE: usize = 384 * 1024;

// ---------- Main Loop ----------

/// Main loop tick interval in milliseconds.
pub const MAIN_LOOP_TICK_MS: u64 = 10;

// ---------- Power Thresholds (milliseconds) ----------

/// Idle threshold before entering modem sleep.
pub const IDLE_MODEM_SLEEP_MS: u64 = 5_000;
/// Idle threshold before entering light sleep.
pub const IDLE_LIGHT_SLEEP_MS: u64 = 30_000;
/// Initial WiFi reconnection backoff in milliseconds.
pub const WIFI_BACKOFF_INITIAL_MS: u64 = 2_000;
/// Maximum WiFi reconnection backoff in milliseconds.
pub const WIFI_BACKOFF_MAX_MS: u64 = 30_000;

// ---------- Stuck Sensor ----------

/// Epsilon for floating-point comparison in stuck sensor detection.
pub const STUCK_SENSOR_EPSILON: f32 = 0.001;

// ---------- Wind Direction ----------

/// Full circle in degrees for wind direction normalization.
pub const WIND_DIR_FULL_CIRCLE: f32 = 360.0;

// ========== AGRICULTURE INDUSTRY ==========

/// Soil moisture sensor ADC pin (capacitive sensor).
#[cfg(feature = "agriculture")]
pub const SOIL_MOISTURE_ADC_PIN: u8 = 7;
/// Soil temperature sensor pin (DS18B20 1-Wire).
#[cfg(feature = "agriculture")]
pub const SOIL_TEMP_PIN: u8 = 16;
/// Leaf wetness sensor ADC pin.
#[cfg(feature = "agriculture")]
pub const LEAF_WETNESS_ADC_PIN: u8 = 17;
/// Second soil moisture probe (deeper layer) ADC pin.
#[cfg(feature = "agriculture")]
pub const SOIL_MOISTURE_DEEP_ADC_PIN: u8 = 18;

/// Soil moisture range: completely dry (ADC reading).
#[cfg(feature = "agriculture")]
pub const SOIL_MOISTURE_DRY_ADC: u16 = 3500;
/// Soil moisture range: saturated (ADC reading).
#[cfg(feature = "agriculture")]
pub const SOIL_MOISTURE_WET_ADC: u16 = 1200;
/// Soil moisture reading interval.
#[cfg(feature = "agriculture")]
pub const SOIL_READ_INTERVAL_MS: u64 = 60_000;
/// Leaf wetness threshold (ADC) — above this means "wet".
#[cfg(feature = "agriculture")]
pub const LEAF_WETNESS_THRESHOLD: u16 = 2000;

/// Growing Degree Day base temperatures for common crops (°C).
#[cfg(feature = "agriculture")]
pub const GDD_BASE_TEMP_CORN: f32 = 10.0;
#[cfg(feature = "agriculture")]
pub const GDD_BASE_TEMP_WHEAT: f32 = 0.0;
#[cfg(feature = "agriculture")]
pub const GDD_BASE_TEMP_RICE: f32 = 10.0;
#[cfg(feature = "agriculture")]
pub const GDD_BASE_TEMP_SOYBEAN: f32 = 10.0;
#[cfg(feature = "agriculture")]
pub const GDD_BASE_TEMP_COTTON: f32 = 15.6;

/// Frost alert threshold (°C).
#[cfg(feature = "agriculture")]
pub const FROST_ALERT_THRESHOLD_C: f32 = 2.0;
/// Frost critical threshold (°C).
#[cfg(feature = "agriculture")]
pub const FROST_CRITICAL_THRESHOLD_C: f32 = 0.0;

/// Evapotranspiration psychrometric constant (kPa/°C).
#[cfg(feature = "agriculture")]
pub const ET_PSYCHROMETRIC_CONST: f32 = 0.0665;
/// Soil field capacity (volumetric %) — triggers irrigation stop.
#[cfg(feature = "agriculture")]
pub const SOIL_FIELD_CAPACITY_PCT: f32 = 35.0;
/// Soil wilting point (volumetric %) — triggers irrigation start.
#[cfg(feature = "agriculture")]
pub const SOIL_WILTING_POINT_PCT: f32 = 15.0;

/// EMA filter alpha for soil sensors.
#[cfg(feature = "agriculture")]
pub const EMA_ALPHA_SOIL_MOISTURE: f32 = 0.2;
#[cfg(feature = "agriculture")]
pub const EMA_ALPHA_SOIL_TEMP: f32 = 0.2;

// ========== SOLAR ENERGY INDUSTRY ==========

/// Pyranometer (ML8511) ADC pin — measures solar irradiance.
#[cfg(feature = "solar")]
pub const PYRANOMETER_ADC_PIN: u8 = 7;
/// Panel temperature sensor pin (DS18B20 1-Wire).
#[cfg(feature = "solar")]
pub const PANEL_TEMP_PIN: u8 = 16;
/// Second panel temperature sensor (back of panel).
#[cfg(feature = "solar")]
pub const PANEL_TEMP_BACK_PIN: u8 = 19;
/// AC power meter pulse input (grid-tie inverter output).
#[cfg(feature = "solar")]
pub const POWER_METER_PULSE_PIN: u8 = 20;

/// Solar irradiance read interval.
#[cfg(feature = "solar")]
pub const SOLAR_READ_INTERVAL_MS: u64 = 10_000;

/// ML8511 calibration: mV per (mW/cm²).
#[cfg(feature = "solar")]
pub const PYRANOMETER_MV_PER_UNIT: f32 = 12.67;
/// ML8511 baseline voltage (mV) at 0 mW/cm².
#[cfg(feature = "solar")]
pub const PYRANOMETER_BASELINE_MV: f32 = 1000.0;
/// Conversion: mW/cm² → W/m².
#[cfg(feature = "solar")]
pub const MW_CM2_TO_W_M2: f32 = 10.0;

/// Panel nominal power (watts-peak) — for yield calculation.
#[cfg(feature = "solar")]
pub const PANEL_NOMINAL_WP: f32 = 400.0;
/// Panel area in m².
#[cfg(feature = "solar")]
pub const PANEL_AREA_M2: f32 = 1.94;
/// Panel temperature coefficient (%/°C above 25°C STC).
#[cfg(feature = "solar")]
pub const PANEL_TEMP_COEFF_PCT_PER_C: f32 = -0.35;
/// Standard Test Conditions temperature.
#[cfg(feature = "solar")]
pub const STC_TEMP_C: f32 = 25.0;
/// Standard Test Conditions irradiance (W/m²).
#[cfg(feature = "solar")]
pub const STC_IRRADIANCE_W_M2: f32 = 1000.0;
/// Minimum irradiance to count as peak sun hour (W/m²).
#[cfg(feature = "solar")]
pub const PEAK_SUN_HOUR_THRESHOLD_W_M2: f32 = 1000.0;
/// Irradiance threshold for cloud detection (W/m²).
#[cfg(feature = "solar")]
pub const CLOUD_COVER_THRESHOLD_W_M2: f32 = 200.0;
/// Soiling loss per day without rain (%).
#[cfg(feature = "solar")]
pub const SOILING_LOSS_PCT_PER_DAY: f32 = 0.1;
/// Maximum soiling accumulation days.
#[cfg(feature = "solar")]
pub const SOILING_MAX_DAYS: u16 = 30;

/// Irradiance validation max (W/m²).
#[cfg(feature = "solar")]
pub const IRRADIANCE_MAX_W_M2: f32 = 1500.0;
/// EMA alpha for irradiance.
#[cfg(feature = "solar")]
pub const EMA_ALPHA_IRRADIANCE: f32 = 0.5;
/// EMA alpha for panel temperature.
#[cfg(feature = "solar")]
pub const EMA_ALPHA_PANEL_TEMP: f32 = 0.3;

// ========== INDIA REGIONAL ==========

/// PM2.5 particulate sensor ADC pin (optional, for AQI).
#[cfg(feature = "india")]
pub const PM25_SENSOR_ADC_PIN: u8 = 21;

/// India analytics processing interval (ms).
#[cfg(feature = "india")]
pub const INDIA_PROCESS_INTERVAL_MS: u64 = 30_000;

/// IST offset from UTC in milliseconds (5 hours 30 minutes).
#[cfg(feature = "india")]
pub const IST_OFFSET_MS: i64 = 5 * 3600 * 1000 + 30 * 60 * 1000;

/// LoRa frequency for India ISM band (IN865: 865.0–867.0 MHz).
#[cfg(feature = "india")]
pub const LORA_FREQUENCY_IN865_HZ: u32 = 865_062_500;

/// Heat wave thresholds — Plains (most of India).
#[cfg(feature = "india")]
pub const INDIA_HEAT_WAVE_PLAINS_C: f32 = 40.0;
#[cfg(feature = "india")]
pub const INDIA_SEVERE_HEAT_WAVE_PLAINS_C: f32 = 45.0;

/// Heat wave thresholds — Coastal regions.
#[cfg(feature = "india")]
pub const INDIA_HEAT_WAVE_COASTAL_C: f32 = 37.0;
#[cfg(feature = "india")]
pub const INDIA_SEVERE_HEAT_WAVE_COASTAL_C: f32 = 41.0;

/// Heat wave thresholds — Hill stations.
#[cfg(feature = "india")]
pub const INDIA_HEAT_WAVE_HILL_C: f32 = 30.0;
#[cfg(feature = "india")]
pub const INDIA_SEVERE_HEAT_WAVE_HILL_C: f32 = 34.0;

/// Monsoon onset detection: minimum daily rainfall (mm) to count as a rain day.
#[cfg(feature = "india")]
pub const INDIA_MONSOON_ONSET_RAIN_MM: f32 = 2.5;
/// Monsoon onset detection: consecutive rain days to declare onset.
#[cfg(feature = "india")]
pub const INDIA_MONSOON_ONSET_DAYS: u16 = 5;

/// Cyclone pressure drop thresholds (hPa over 3 hours).
#[cfg(feature = "india")]
pub const INDIA_CYCLONE_WATCH_DROP_HPA: f32 = 3.0;
#[cfg(feature = "india")]
pub const INDIA_CYCLONE_WARNING_DROP_HPA: f32 = 5.0;
#[cfg(feature = "india")]
pub const INDIA_CYCLONE_SEVERE_DROP_HPA: f32 = 8.0;

/// GDD base temperatures for Indian crops (°C).
#[cfg(feature = "india")]
pub const GDD_BASE_TEMP_RICE_INDIA: f32 = 10.0;
#[cfg(feature = "india")]
pub const GDD_BASE_TEMP_WHEAT_INDIA: f32 = 0.0;
#[cfg(feature = "india")]
pub const GDD_BASE_TEMP_SUGARCANE: f32 = 12.0;
#[cfg(feature = "india")]
pub const GDD_BASE_TEMP_TEA: f32 = 7.0;
#[cfg(feature = "india")]
pub const GDD_BASE_TEMP_JUTE: f32 = 15.0;

/// EMA alpha for PM2.5 sensor.
#[cfg(feature = "india")]
pub const EMA_ALPHA_PM25: f32 = 0.2;

/// PM2.5 sensor warm-up time (ms). GP2Y1014AU0F requires ~10s stabilization.
#[cfg(feature = "india")]
pub const PM25_WARMUP_MS: u64 = 10_000;

/// PM2.5 maximum valid reading (µg/m³). Above this indicates sensor fault.
#[cfg(feature = "india")]
pub const PM25_MAX_UGM3: f32 = 1000.0;

/// PM2.5 LED pulse width (µs). GP2Y1014AU0F datasheet: 320 µs.
#[cfg(feature = "india")]
pub const PM25_LED_PULSE_US: u32 = 320;

/// PM2.5 ADC sample delay within LED pulse (µs). Datasheet: 280 µs after LED on.
#[cfg(feature = "india")]
pub const PM25_SAMPLE_DELAY_US: u32 = 280;

/// Number of ADC samples to average per PM2.5 reading (noise rejection).
#[cfg(feature = "india")]
pub const PM25_SAMPLE_COUNT: u8 = 10;

// ---------- Firmware Info ----------

pub const FIRMWARE_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const DEVICE_NAME_DEFAULT: &str = "WeatherStation";

/// Runtime configuration stored in NVS.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub device_name: heapless::String<32>,
    pub device_id: u16,

    // WiFi
    pub wifi_ssid: heapless::String<32>,
    pub wifi_password: heapless::String<64>,

    // MQTT
    pub mqtt_broker: heapless::String<64>,
    pub mqtt_port: u16,
    pub mqtt_use_tls: bool,
    pub mqtt_username: heapless::String<32>,
    pub mqtt_password: heapless::String<64>,

    // Intervals (seconds)
    pub telemetry_interval_s: u16,
    pub status_interval_s: u16,

    // Alert thresholds
    pub alert_wind_speed_kmh: f32,
    pub alert_temp_high_c: f32,
    pub alert_temp_low_c: f32,
    pub alert_rain_mm_hr: f32,

    // LoRa
    pub lora_spreading_factor: u8,
    pub lora_tx_power_dbm: i8,

    // Calibration offsets
    pub cal_temp_offset: f32,
    pub cal_humidity_offset: f32,
    pub cal_pressure_offset: f32,
    pub cal_wind_dir_offset: f32,

    // Agriculture industry config
    #[cfg(feature = "agriculture")]
    pub gdd_base_temp_c: f32,
    #[cfg(feature = "agriculture")]
    pub irrigation_enable: bool,
    #[cfg(feature = "agriculture")]
    pub crop_type: heapless::String<16>,

    // Solar industry config
    #[cfg(feature = "solar")]
    pub panel_wp: f32,
    #[cfg(feature = "solar")]
    pub panel_area_m2: f32,
    #[cfg(feature = "solar")]
    pub panel_tilt_deg: f32,
    #[cfg(feature = "solar")]
    pub panel_azimuth_deg: f32,

    // India regional config
    #[cfg(feature = "india")]
    pub india_region: crate::industry::india::IndiaRegion,
    #[cfg(feature = "india")]
    pub india_gdd_base_temp_c: f32,
    #[cfg(feature = "india")]
    pub india_crop_type: heapless::String<16>,
}

impl RuntimeConfig {
    /// Validate configuration values, clamping or correcting out-of-range fields.
    /// Logs warnings for any values that were adjusted.
    pub fn validate(&mut self) {
        if self.telemetry_interval_s < 5 {
            log::warn!("Config: telemetry_interval_s too low ({}), clamping to 5", self.telemetry_interval_s);
            self.telemetry_interval_s = 5;
        }
        if self.status_interval_s < 10 {
            log::warn!("Config: status_interval_s too low ({}), clamping to 10", self.status_interval_s);
            self.status_interval_s = 10;
        }
        if !(1..=12).contains(&self.lora_spreading_factor) {
            log::warn!("Config: invalid SF {}, resetting to {}", self.lora_spreading_factor, LORA_SPREADING_FACTOR);
            self.lora_spreading_factor = LORA_SPREADING_FACTOR;
        }
        if self.lora_tx_power_dbm < 2 || self.lora_tx_power_dbm > 20 {
            log::warn!("Config: TX power {} out of range, resetting to {}", self.lora_tx_power_dbm, LORA_TX_POWER_DBM);
            self.lora_tx_power_dbm = LORA_TX_POWER_DBM;
        }
        if self.alert_wind_speed_kmh < 0.0 || self.alert_wind_speed_kmh > WIND_SPEED_MAX_KMH {
            self.alert_wind_speed_kmh = 90.0;
        }
        if self.cal_temp_offset.abs() > 10.0 {
            log::warn!("Config: temp cal offset {:.1} seems too large, clamping", self.cal_temp_offset);
            self.cal_temp_offset = self.cal_temp_offset.clamp(-10.0, 10.0);
        }
        #[cfg(feature = "solar")]
        {
            if self.panel_wp <= 0.0 || self.panel_wp > 2000.0 {
                log::warn!("Config: invalid panel_wp {}, resetting to {}", self.panel_wp, PANEL_NOMINAL_WP);
                self.panel_wp = PANEL_NOMINAL_WP;
            }
            if self.panel_area_m2 <= 0.0 || self.panel_area_m2 > 20.0 {
                log::warn!("Config: invalid panel_area_m2 {}, resetting to {}", self.panel_area_m2, PANEL_AREA_M2);
                self.panel_area_m2 = PANEL_AREA_M2;
            }
        }
        #[cfg(feature = "agriculture")]
        {
            if self.gdd_base_temp_c < -10.0 || self.gdd_base_temp_c > 30.0 {
                log::warn!("Config: invalid gdd_base_temp {}, resetting to 10.0", self.gdd_base_temp_c);
                self.gdd_base_temp_c = GDD_BASE_TEMP_CORN;
            }
        }
        #[cfg(feature = "india")]
        {
            if self.india_gdd_base_temp_c < -10.0 || self.india_gdd_base_temp_c > 30.0 {
                log::warn!("Config: invalid india_gdd_base_temp {}, resetting to 10.0", self.india_gdd_base_temp_c);
                self.india_gdd_base_temp_c = GDD_BASE_TEMP_RICE_INDIA;
            }
        }
    }
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            // SAFETY: DEVICE_NAME_DEFAULT ("WeatherStation") is 14 chars, fits in String<32>.
            device_name: heapless::String::try_from(DEVICE_NAME_DEFAULT).unwrap_or_default(),
            device_id: 1,
            wifi_ssid: heapless::String::new(),
            wifi_password: heapless::String::new(),
            mqtt_broker: heapless::String::new(),
            mqtt_port: MQTT_PORT_DEFAULT,
            mqtt_use_tls: false,
            mqtt_username: heapless::String::new(),
            mqtt_password: heapless::String::new(),
            telemetry_interval_s: 30,
            status_interval_s: 60,
            alert_wind_speed_kmh: 90.0,
            alert_temp_high_c: 40.0,
            alert_temp_low_c: -10.0,
            alert_rain_mm_hr: 50.0,
            lora_spreading_factor: LORA_SPREADING_FACTOR,
            lora_tx_power_dbm: LORA_TX_POWER_DBM,
            cal_temp_offset: 0.0,
            cal_humidity_offset: 0.0,
            cal_pressure_offset: 0.0,
            cal_wind_dir_offset: 0.0,
            #[cfg(feature = "agriculture")]
            gdd_base_temp_c: GDD_BASE_TEMP_CORN,
            #[cfg(feature = "agriculture")]
            irrigation_enable: false,
            #[cfg(feature = "agriculture")]
            crop_type: heapless::String::new(),
            #[cfg(feature = "solar")]
            panel_wp: PANEL_NOMINAL_WP,
            #[cfg(feature = "solar")]
            panel_area_m2: PANEL_AREA_M2,
            #[cfg(feature = "solar")]
            panel_tilt_deg: 30.0,
            #[cfg(feature = "solar")]
            panel_azimuth_deg: 180.0,
            #[cfg(feature = "india")]
            india_region: crate::industry::india::IndiaRegion::Plains,
            #[cfg(feature = "india")]
            india_gdd_base_temp_c: GDD_BASE_TEMP_RICE_INDIA,
            #[cfg(feature = "india")]
            india_crop_type: heapless::String::new(),
        }
    }
}
