# India Regional Module

## Overview

The India module extends the base weather station with region-specific analytics
tailored to the Indian subcontinent. It provides IMD-aligned seasonal classification,
monsoon onset/withdrawal tracking, heat wave alerts using official IMD criteria,
tropical cyclone detection, NAQI-based air quality indexing, and IST timezone support
for daily aggregations. Enabled with `--features india`.

```bash
cargo build --release --features india
```

## Features

| Feature | Description |
|---------|-------------|
| **Monsoon tracking** | Onset/break/withdrawal detection using consecutive rain-day analysis |
| **IMD season classification** | Winter, Pre-Monsoon, SW Monsoon, Post-Monsoon |
| **Heat wave alerts** | IMD criteria for plains, coastal, and hill regions |
| **Apparent temperature** | Steadman's feels-like temperature model |
| **Cyclone risk detection** | 3-hour pressure tendency + wind speed analysis |
| **NAQI air quality** | PM2.5-based National Air Quality Index (CPCB breakpoints) |
| **Indian crop GDD** | Rice, wheat, sugarcane, tea, jute base temperatures |
| **IST timezone** | UTC+05:30 day boundary for daily aggregations |
| **IN865 LoRa band** | 865–867 MHz ISM band compliance for India |

## Industrial-Grade Component Specifications

All components are selected for industrial temperature range (-40°C to +85°C)
and harsh outdoor deployment with IP65+ enclosure, conformal coating, and TVS
protection on all external I/O lines.

### Base Station Components

| # | Component | Industrial Part Number | Temp Range | Notes |
|---|-----------|----------------------|------------|-------|
| 1 | MCU | ESP32-S3-WROOM-1-N16R8**I** | -40°C to +85°C | Industrial "I" suffix |
| 2 | Temp/Hum/Press | Bosch BME280 | -40°C to +85°C | Automotive-qualified avail. |
| 3 | Wind Direction | AS5600-ASOM | -40°C to +85°C | Industrial magnetic encoder |
| 4 | UV Sensor | SI1145-A10-GMR | -40°C to +85°C | UV/ALS/proximity |
| 5 | Light Sensor | BH1750FVI-TR | -40°C to +85°C | Ambient light sensor |
| 6 | LoRa Radio | Semtech SX1276 | -40°C to +85°C | ISM transceiver |
| 7 | Voltage Regulator | TPS63020DSJR | -40°C to +85°C | Wide-input buck-boost |
| 8 | TVS Protection | TPD4E05U06DQAR | -40°C to +125°C | All external I/O |
| 9 | Conformal Coating | Dow Corning 1-2577 | -65°C to +200°C | Salt-spray resistant |

### Enclosure & Environmental Protection

| Specification | Value |
|---------------|-------|
| IP Rating | IP65 (minimum), IP67 recommended |
| Material | UV-stabilized polycarbonate |
| Mounting | Stainless steel 304 pole mount |
| Cable Glands | IP68 rated, M12 connectors |
| Lightning | MOV surge protector on antenna + power |
| Grounding | Dedicated earth lug, bonded to mast |
| Operating Temp | -40°C to +85°C (electronics) |
| Humidity | 0–100% RH (conformal coated) |

## Additional Sensors (India Extension)

| # | Component | Part Number | Interface | Purpose | Temp Range |
|---|-----------|-------------|-----------|---------|------------|
| 1 | PM2.5 particulate sensor | GP2Y1014AU0F | ADC (GPIO21) | Air quality monitoring | -10°C to +65°C |

### Pin Mapping (India Extension)

```
ESP32-S3 Additional Pins (India)
═══════════════════════════════════════════════════════════

ADC (Air Quality):
  GPIO 21 ──── ADC ──── PM2.5 Sensor (analog output)
```

### Circuit Diagram (India Extension)

```
         3.3V
          │
          ├──── 150Ω ────┐
          │               │
          │        ┌──────┴──────────┐
          ├───────►│ GP2Y1014AU0F   │
          │   VCC  │ PM2.5 Dust     │
          │        │ Sensor          │
   GPIO21 ◄───────┤ Vo (analog)     │
          │        │                 │
          │   LED ─┤ LED drive       │
     GND ─┤───────►│ GND            │
          │        └─────────────────┘
          │
          ├──── 220µF cap to GND (LED pulse smoothing)
```

### PM2.5 Driver — Industrial Features

The PM2.5 driver implements production-grade signal acquisition:

| Feature | Description |
|---------|-------------|
| **LED pulse timing** | 320 µs pulse, ADC sample at 280 µs per GP2Y1014AU0F datasheet |
| **Multi-sample averaging** | 10 samples with trimmed-mean outlier rejection |
| **Warm-up enforcement** | 10-second minimum after power-on before readings accepted |
| **NaN/infinity guards** | All inputs and outputs validated for finite values |
| **Fault detection** | 5 consecutive out-of-range readings triggers sensor error |
| **EMA filtering** | Exponential moving average (α=0.2) for trend smoothing |
| **Calibration API** | Runtime-adjustable baseline and sensitivity from NVS |

## Analytics Features

### 1. IMD Season Classification

Automatic season detection based on month:

| Season | Months | Description |
|--------|--------|-------------|
| Winter | Jan–Feb | Cold, dry season |
| Pre-Monsoon | Mar–May | Hot, dry with thunderstorms |
| SW Monsoon | Jun–Sep | Primary rainy season |
| Post-Monsoon | Oct–Dec | Retreating monsoon, northeast winds |

### 2. Monsoon Tracking

Monitors rainfall patterns to detect monsoon lifecycle phases:

| Phase | Detection Criteria |
|-------|--------------------|
| **PreMonsoon** | Before onset (< 5 consecutive rain days in Jun–Sep) |
| **Active** | ≥ 5 consecutive days with rainfall ≥ 2.5 mm |
| **Break** | Dry spell after established onset |
| **Retreat** | October after active monsoon |
| **Off** | Outside monsoon season |

**Tracked metrics:**
- Consecutive rain days (threshold: 5 days at ≥ 2.5 mm/day)
- Accumulated monsoon rainfall (mm)
- Daily rainfall total (reset at IST midnight)

### 3. Heat Wave Alerts (IMD Criteria)

Region-specific thresholds per India Meteorological Department:

| Region | Heat Wave (°C) | Severe Heat Wave (°C) |
|--------|---------------|----------------------|
| Plains | ≥ 40.0 | ≥ 45.0 |
| Coastal | ≥ 37.0 | ≥ 41.0 |
| Hill | ≥ 30.0 | ≥ 34.0 |

Configurable via `india_region` in RuntimeConfig.

**Additional heat metrics:**
- **Apparent Temperature** — Steadman's model combining temperature, humidity, and wind
- **Discomfort Index** — Thom's DI for thermal comfort assessment

### 4. Tropical Cyclone Risk Detection

3-hour pressure tendency monitoring for early cyclone warning:

| Level | Pressure Drop (3h) | Wind Speed | Action |
|-------|-------------------|------------|--------|
| **None** | < 3 hPa | — | Normal |
| **Watch** | ≥ 3 hPa | Any | Advisory |
| **Warning** | ≥ 5 hPa | ≥ 60 km/h | MQTT + BLE alert |
| **Severe** | ≥ 8 hPa | ≥ 90 km/h | Critical alert |

Pressure is sampled every processing interval into an 18-slot circular buffer
covering a 3-hour window.

### 5. NAQI Air Quality Index

PM2.5-based NAQI calculation using CPCB (Central Pollution Control Board) breakpoints:

| Category | PM2.5 (µg/m³) | AQI Range |
|----------|--------------|-----------|
| Good | 0–30 | 0–50 |
| Satisfactory | 31–60 | 51–100 |
| Moderate | 61–90 | 101–200 |
| Poor | 91–120 | 201–300 |
| Very Poor | 121–250 | 301–400 |
| Severe | 250+ | 401–500 |

AQI ≥ 401 (Severe) triggers a critical alert.

### 6. Indian Crop GDD

Growing Degree Day accumulation with Indian crop base temperatures:

| Crop | T_base (°C) |
|------|-------------|
| Rice (Kharif) | 10.0 |
| Wheat (Rabi) | 0.0 |
| Sugarcane | 12.0 |
| Tea | 7.0 |
| Jute | 15.0 |

Configurable via `india_gdd_base_temp_c` in RuntimeConfig.

### 7. IST Timezone Support

Daily aggregations (rainfall, GDD, min/max temperature) reset at IST midnight
(UTC+05:30) rather than UTC midnight, matching Indian agricultural and
meteorological conventions.

```
IST offset: +05:30 (19800000 ms from UTC)
```

### 8. LoRa IN865 Band

For LoRaWAN deployments in India, the module configures the IN865 ISM band:

| Parameter | Value |
|-----------|-------|
| Frequency | 865.0625 MHz |
| Band | 865–867 MHz |
| Max EIRP | 36 dBm (per WPC India) |

## MQTT Topics

```
weather/{device_id}/india              — India regional data + analytics (JSON)
weather/{device_id}/alerts/industry    — Heat wave, cyclone, AQI, monsoon alerts
```

### India Payload Example

```json
{
  "season": "SouthwestMonsoon",
  "monsoon_phase": "Active",
  "consecutive_rain_days": 8,
  "monsoon_rainfall_mm": 342.5,
  "daily_rainfall_mm": 28.3,
  "heat_wave": "None",
  "apparent_temp_c": 34.2,
  "discomfort_index": 29.8,
  "cyclone_risk": "None",
  "pressure_tendency_3h_hpa": -0.5,
  "aqi_naqi": 156,
  "aqi_category": "Moderate",
  "gdd_accumulated": 850.0,
  "gdd_today": 15.2
}
```

### Alert Examples

```json
{
  "severity": "Critical",
  "category": "heat_wave",
  "message": "SEVERE HEAT WAVE: dangerous conditions, take precautions",
  "timestamp_ms": 1719500000000
}
```

```json
{
  "severity": "Warning",
  "category": "cyclone",
  "message": "Cyclone warning: rapid pressure drop with high winds",
  "timestamp_ms": 1719500000000
}
```

```json
{
  "severity": "Info",
  "category": "monsoon",
  "message": "Monsoon onset detected: sustained rainfall pattern identified",
  "timestamp_ms": 1719500000000
}
```

## Configuration

### Runtime Config Fields (NVS-persisted)

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `india_region` | IndiaRegion | Plains | Region type for heat wave thresholds |
| `india_gdd_base_temp_c` | f32 | 10.0 | GDD base temperature for crop |
| `india_crop_type` | String | "" | Crop name (for display) |

### Compile-time Constants

| Constant | Value | Description |
|----------|-------|-------------|
| `PM25_SENSOR_ADC_PIN` | GPIO 21 | PM2.5 sensor analog input |
| `INDIA_PROCESS_INTERVAL_MS` | 30,000 | Analytics processing interval |
| `IST_OFFSET_MS` | 19,800,000 | UTC+05:30 in milliseconds |
| `LORA_FREQUENCY_IN865_HZ` | 865,062,500 | IN865 LoRa center frequency |
| `INDIA_MONSOON_ONSET_RAIN_MM` | 2.5 | Min daily rainfall for rain-day |
| `INDIA_MONSOON_ONSET_DAYS` | 5 | Consecutive rain days for onset |

## Power Budget Addition

| Component | Active (mA) | Sleep (µA) |
|-----------|------------|-----------|
| PM2.5 sensor (GP2Y1014AU0F) | 20.0 | 0 (switched) |
| **Subtotal** | **~20 mA** | **~0 µA** |

Note: The PM2.5 sensor is power-switched and only activated during sampling
(~10 ms per reading) to minimize average power consumption.
