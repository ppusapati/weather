# Agriculture Industry Module

## Overview

The agriculture module extends the base weather station with crop-focused
sensors and analytics for precision farming. Enabled with `--features agriculture`.

```bash
cargo build --release --features agriculture
```

## Additional Sensors

| # | Component | Part Number | Interface | Purpose |
|---|-----------|-------------|-----------|---------|
| 1 | Capacitive soil moisture (shallow) | v2.0 | ADC (GPIO7) | Root zone moisture (10–15 cm) |
| 2 | Capacitive soil moisture (deep) | v2.0 | ADC (GPIO18) | Root zone moisture (30–40 cm) |
| 3 | DS18B20 waterproof probe | DS18B20 | 1-Wire (GPIO16) | Soil temperature |
| 4 | Leaf wetness sensor | SEN-LWS | ADC (GPIO17) | Surface moisture detection |

### Pin Mapping (Agriculture Extension)

```
STM32F407 Additional Pins (Agriculture)
═══════════════════════════════════════════════════════════

ADC (Soil Moisture):
  PC2 (ADC1_CH12) ──── Soil Moisture Shallow (capacitive)
  PC3 (ADC1_CH13) ──── Soil Moisture Deep (capacitive)

1-Wire (Soil Temperature):
  GPIO 16 ──── DATA ──── DS18B20 (4.7kΩ pull-up to 3.3V)

ADC (Leaf Wetness):
  GPIO 17 ──── ADC ──── Leaf Wetness Sensor
```

### Circuit Diagram (Agriculture Extension)

```
         3.3V
          │
          ├──── 4.7kΩ ────┐
          │                │
          │         ┌──────┴──────┐
          ├────────►│ DS18B20     │
          │    VCC  │ Waterproof  │
          │         │ Probe       │
   GPIO16 ◄────────┤ DATA        │
          │         │ (1-Wire)    │
     GND ─┤────────►│ GND         │
          │         └─────────────┘
          │
          │     ┌─────────────────────┐
          ├────►│ Capacitive Soil     │
          │ VCC │ Moisture v2.0       │
   GPIO7  ◄────┤ AOUT (shallow)      │
     GND ─┤────►│ GND                │
          │     └─────────────────────┘
          │
          │     ┌─────────────────────┐
          ├────►│ Capacitive Soil     │
          │ VCC │ Moisture v2.0       │
   GPIO18 ◄────┤ AOUT (deep)         │
     GND ─┤────►│ GND                │
          │     └─────────────────────┘
          │
          │     ┌─────────────────────┐
          ├────►│ Leaf Wetness        │
          │ VCC │ Sensor (resistive)  │
   GPIO17 ◄────┤ AOUT                │
     GND ──────►│ GND                │
                └─────────────────────┘
```

## Analytics Features

### 1. Evapotranspiration (ET₀)

Reference evapotranspiration calculated using the FAO Penman-Monteith equation.
Falls back to Hargreaves-Samani when solar radiation data is unavailable.

**Inputs**: Air temperature, humidity, wind speed, solar radiation, pressure
**Output**: ET₀ in mm/day

```
MQTT topic: weather/{device_id}/agriculture
Field: "et0_mm_day"
```

### 2. Growing Degree Days (GDD)

Accumulated thermal units for crop phenology staging.

```
GDD_daily = max(0, (T_max + T_min) / 2 - T_base)
```

Supported crop base temperatures:
| Crop | T_base (°C) |
|------|-------------|
| Corn | 10.0 |
| Wheat | 0.0 |
| Rice | 10.0 |
| Soybean | 10.0 |
| Cotton | 15.6 |

Configurable via `gdd_base_temp_c` in RuntimeConfig or BLE.

### 3. Frost Alerts

Multi-level frost risk assessment considering temperature and humidity:

| Level | Condition | Action |
|-------|-----------|--------|
| **None** | T > 5°C | Normal operation |
| **Watch** | T approaching frost threshold | Advisory notification |
| **Warning** | T ≤ 2°C (adjusted for humidity) | MQTT + BLE alert |
| **Critical** | T ≤ 0°C | High-priority alert |

Low humidity (< 50%) increases radiative frost risk by adding a 1.5°C offset.

### 4. Irrigation Scheduling

Soil moisture-based irrigation recommendations:

```
┌─────────────────────────────────────────────────────────┐
│ Soil Moisture Profile                                   │
│                                                         │
│ 100% ─── Saturation ──────────────────────────────────  │
│  35% ─── Field Capacity ──── Irrigation Stop ────────  │
│  23% ─── Management Allowable Depletion ── Monitor ──  │
│  15% ─── Wilting Point ──── Irrigate Immediately ────  │
│   0% ─── Oven Dry ───────────────────────────────────  │
└─────────────────────────────────────────────────────────┘
```

| Status | Moisture Range | Recommendation |
|--------|---------------|----------------|
| FieldCapacity | ≥ 35% | No irrigation needed |
| Adequate | 23–35% | Normal |
| MonitorClosely | 19–23% | Plan irrigation |
| IrrigateRecommended | 15–19% | Start irrigation |
| IrrigateCritical | ≤ 15% | Irrigate immediately |

### 5. Disease Risk Index

Smith period-based fungal disease prediction model (0–100 scale):

- **Temperature factor**: Bell curve centered at 20°C (10–30°C range)
- **Wetness factor**: Risk escalates after 6+ hours continuous leaf wetness
- Risk ≥ 80 triggers a warning alert

### 6. Spray Window Assessment

Evaluates suitability for pesticide/herbicide application:

| Condition | Assessment |
|-----------|------------|
| Rain > 0.5 mm/hr | Poor |
| Wind > 20 km/h | Poor |
| Temp < 5°C + calm | Poor (inversion) |
| Wind 12–20 km/h | Marginal |
| Wind < 12 km/h, no rain | Good |

## MQTT Topics

```
weather/{device_id}/agriculture       — Agriculture sensor data + analytics (JSON)
weather/{device_id}/alerts/industry   — Frost, irrigation, disease alerts
```

### Agriculture Payload Example

```json
{
  "soil_moisture_shallow_pct": 28.5,
  "soil_moisture_deep_pct": 35.2,
  "soil_temp_c": 18.3,
  "leaf_wetness_pct": 45.0,
  "leaf_wet_duration_min": 120,
  "et0_mm_day": 4.2,
  "gdd_accumulated": 1250.5,
  "gdd_today": 12.3,
  "frost_risk": "None",
  "irrigation": "Adequate",
  "disease_risk_index": 25,
  "spray_window": "Good"
}
```

## Power Budget Addition

| Component | Active (mA) | Sleep (µA) |
|-----------|------------|-----------|
| Soil moisture sensor ×2 | 8.0 | 0 (switched) |
| DS18B20 | 1.5 | 0.75 |
| Leaf wetness sensor | 0.2 | 0 (switched) |
| **Subtotal** | **~10 mA** | **~1 µA** |
