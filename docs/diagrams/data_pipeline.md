# Data Pipeline Diagram

## 1. Complete Data Pipeline

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                           DATA PIPELINE                                         │
│                                                                                 │
│  STAGE 1: ACQUISITION                                                           │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌─────────┐ │
│  │ BME280   │ │ AS5600   │ │Anemometer│ │  Rain    │ │ SI1145   │ │ BH1750  │ │
│  │          │ │          │ │          │ │  Gauge   │ │          │ │         │ │
│  │ I2C Read │ │ I2C Read │ │ GPIO ISR │ │ GPIO ISR │ │ I2C Read │ │I2C Read │ │
│  │ 10s poll │ │ 5s poll  │ │ Cont.    │ │ Cont.    │ │ 30s poll │ │30s poll │ │
│  └────┬─────┘ └────┬─────┘ └────┬─────┘ └────┬─────┘ └────┬─────┘ └────┬────┘ │
│       │             │            │             │            │            │      │
│       ▼             ▼            ▼             ▼            ▼            ▼      │
│  ┌──────────────────────────────────────────────────────────────────────────┐   │
│  │ RawReading                                                               │   │
│  │ {                                                                        │   │
│  │   temp_raw: i32,         // BME280 ADC value                            │   │
│  │   humidity_raw: i32,     // BME280 ADC value                            │   │
│  │   pressure_raw: i32,     // BME280 ADC value                            │   │
│  │   wind_pulses: u32,      // count since last read                       │   │
│  │   wind_angle_raw: u16,   // AS5600 12-bit angle                         │   │
│  │   rain_tips: u32,        // count since last read                       │   │
│  │   uv_raw: u16,           // SI1145 raw                                  │   │
│  │   light_raw: u16,        // BH1750 raw                                  │   │
│  │   timestamp: u64,        // millis since boot                           │   │
│  │ }                                                                        │   │
│  └─────────────────────────────────────┬────────────────────────────────────┘   │
│                                        │                                        │
│  STAGE 2: CALIBRATION                  ▼                                        │
│  ┌──────────────────────────────────────────────────────────────────────────┐   │
│  │ Apply calibration coefficients from NVS                                  │   │
│  │                                                                          │   │
│  │ Temperature:  T = (raw × gain_t) + offset_t     [°C]                   │   │
│  │ Humidity:     H = (raw × gain_h) + offset_h     [%]                    │   │
│  │ Pressure:     P = compensate(raw, cal_data)      [hPa]                 │   │
│  │ Wind Speed:   S = pulses × 2.4 / period          [km/h]               │   │
│  │ Wind Dir:     D = (raw × 360 / 4096) + offset_d  [degrees]            │   │
│  │ Rain:         R = tips × 0.2                      [mm]                 │   │
│  │ UV Index:     U = raw / 100.0                     [index]              │   │
│  │ Light:        L = raw / 1.2                       [lux]                │   │
│  └─────────────────────────────────────┬────────────────────────────────────┘   │
│                                        │                                        │
│  STAGE 3: FILTERING                   ▼                                        │
│  ┌──────────────────────────────────────────────────────────────────────────┐   │
│  │ Exponential Moving Average (EMA)                                         │   │
│  │                                                                          │   │
│  │ filtered = α × new_value + (1 - α) × previous_filtered                 │   │
│  │                                                                          │   │
│  │ Sensor-specific α values:                                                │   │
│  │   Temperature:  α = 0.3  (slow response, stable)                        │   │
│  │   Humidity:     α = 0.3                                                  │   │
│  │   Pressure:     α = 0.2  (very stable)                                  │   │
│  │   Wind Speed:   α = 0.7  (fast response, gusty)                         │   │
│  │   Wind Dir:     α = 0.5  (circular average)                              │   │
│  │   UV/Light:     α = 0.4                                                  │   │
│  └─────────────────────────────────────┬────────────────────────────────────┘   │
│                                        │                                        │
│  STAGE 4: VALIDATION                  ▼                                        │
│  ┌──────────────────────────────────────────────────────────────────────────┐   │
│  │ Range checks:                                                            │   │
│  │   Temperature:  -40°C  to  85°C                                         │   │
│  │   Humidity:       0%   to 100%                                           │   │
│  │   Pressure:     300    to 1100 hPa                                      │   │
│  │   Wind Speed:     0    to 200 km/h                                      │   │
│  │   Wind Dir:       0    to 359°                                           │   │
│  │   Rain:           0    to 500 mm/h                                      │   │
│  │   UV Index:       0    to  15                                            │   │
│  │   Light:          0    to 120000 lux                                    │   │
│  │                                                                          │   │
│  │ Stuck sensor detection:                                                  │   │
│  │   If value unchanged for > 10 consecutive readings → mark degraded      │   │
│  │                                                                          │   │
│  │ Out-of-range → set value to NaN, mark sensor as ERROR                   │   │
│  └─────────────────────────────────────┬────────────────────────────────────┘   │
│                                        │                                        │
│  STAGE 5: DERIVED VALUES              ▼                                        │
│  ┌──────────────────────────────────────────────────────────────────────────┐   │
│  │ Heat Index:                                                              │   │
│  │   HI = -8.785 + 1.611T + 2.339H - 0.146TH - 0.013T² - 0.016H²        │   │
│  │        + 0.002T²H + 0.001TH² - 0.000004T²H²                           │   │
│  │                                                                          │   │
│  │ Dew Point:                                                               │   │
│  │   γ = ln(H/100) + (17.67T)/(243.5+T)                                   │   │
│  │   Td = (243.5 × γ) / (17.67 - γ)                                       │   │
│  │                                                                          │   │
│  │ Wind Chill (if T < 10°C and wind > 4.8 km/h):                          │   │
│  │   WC = 13.12 + 0.6215T - 11.37V^0.16 + 0.3965TV^0.16                  │   │
│  └─────────────────────────────────────┬────────────────────────────────────┘   │
│                                        │                                        │
│  STAGE 6: PACKAGING                   ▼                                        │
│  ┌──────────────────────────────────────────────────────────────────────────┐   │
│  │ WeatherReading {                                                         │   │
│  │   timestamp:        DateTime<Utc>,                                       │   │
│  │   temperature_c:    Option<f32>,                                         │   │
│  │   humidity_pct:     Option<f32>,                                         │   │
│  │   pressure_hpa:     Option<f32>,                                         │   │
│  │   wind_speed_kmh:   Option<f32>,                                         │   │
│  │   wind_dir_deg:     Option<u16>,                                         │   │
│  │   rain_mm:          f32,                                                 │   │
│  │   uv_index:         Option<f32>,                                         │   │
│  │   light_lux:        Option<f32>,                                         │   │
│  │   heat_index_c:     Option<f32>,                                         │   │
│  │   dew_point_c:      Option<f32>,                                         │   │
│  │   wind_chill_c:     Option<f32>,                                         │   │
│  │   sensor_status:    SensorStatusMap,                                     │   │
│  │ }                                                                        │   │
│  └─────────────────────────────────────┬────────────────────────────────────┘   │
│                                        │                                        │
│  STAGE 7: DISTRIBUTION                ▼                                        │
│  ┌──────────────────────────────────────────────────────────────────────────┐   │
│  │                                                                          │   │
│  │   ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌─────────┐  │   │
│  │   │  MQTT    │  │   HTTP   │  │   BLE    │  │   LoRa   │  │  Flash  │  │   │
│  │   │ Publish  │  │  Cache   │  │  Notify  │  │ Transmit │  │  Store  │  │   │
│  │   │          │  │          │  │          │  │          │  │         │  │   │
│  │   │ JSON     │  │ JSON     │  │ Binary   │  │ Binary   │  │ Binary  │  │   │
│  │   │ ~200B    │  │ ~200B    │  │ GATT     │  │ 27 bytes │  │ Compact │  │   │
│  │   └──────────┘  └──────────┘  └──────────┘  └──────────┘  └─────────┘  │   │
│  │                                                                          │   │
│  └──────────────────────────────────────────────────────────────────────────┘   │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

## 2. Alert Processing Pipeline

```
    WeatherReading
         │
         ▼
    ┌────────────────┐
    │  Threshold     │
    │  Comparator    │
    │                │
    │  For each      │
    │  sensor value: │
    │  value > max?  │──── YES ──►┌───────────────┐
    │  value < min?  │            │ Generate Alert│
    │                │            │               │
    └────────────────┘            │ AlertEvent {  │
                                  │  sensor,      │
                                  │  value,       │
                                  │  threshold,   │
                                  │  type,        │
                                  │  timestamp    │
                                  │ }             │
                                  └───────┬───────┘
                                          │
                                ┌─────────┼─────────┐
                                ▼         ▼         ▼
                          ┌──────────┐ ┌──────┐ ┌──────┐
                          │MQTT Alert│ │ BLE  │ │ UART │
                          │Topic     │ │Notify│ │ Log  │
                          │(QoS 2)   │ │      │ │      │
                          └──────────┘ └──────┘ └──────┘
```
