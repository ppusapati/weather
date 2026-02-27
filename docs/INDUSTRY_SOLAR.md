# Solar Energy Industry Module

## Overview

The solar module extends the base weather station with photovoltaic (PV)
performance monitoring and analytics. Enabled with `--features solar`.

```bash
cargo build --release --features solar
```

## Additional Sensors

| # | Component | Part Number | Interface | Purpose |
|---|-----------|-------------|-----------|---------|
| 1 | Pyranometer / Solar sensor | ML8511 / SP-110 | ADC (GPIO7) | Global Horizontal Irradiance |
| 2 | DS18B20 (panel front) | DS18B20 | 1-Wire (GPIO16) | Panel front surface temperature |
| 3 | DS18B20 (panel back) | DS18B20 | 1-Wire (GPIO19) | Panel back-of-module temperature |
| 4 | Power meter (optional) | — | GPIO20 (pulse) | Grid-tie inverter output |

### Pin Mapping (Solar Extension)

```
ESP32-S3 Additional Pins (Solar)
═══════════════════════════════════════════════════════════

ADC (Pyranometer):
  GPIO 7  ──── ADC ──── ML8511 / SP-110 Analog Output

1-Wire (Panel Temperature):
  GPIO 16 ──── DATA ──── DS18B20 #1 (panel front, 4.7kΩ pull-up)
  GPIO 19 ──── DATA ──── DS18B20 #2 (panel back, 4.7kΩ pull-up)

GPIO (Power Meter):
  GPIO 20 ──── PULSE ── Inverter kWh meter (10kΩ pull-up)
```

### Circuit Diagram (Solar Extension)

```
         3.3V
          │
          │     ┌─────────────────────┐
          ├────►│ ML8511 / SP-110     │
          │ VCC │ Pyranometer         │
   GPIO7  ◄────┤ OUT (analog)        │
          │     │ EN ─── 3.3V         │
     GND ─┤────►│ GND                │
          │     └─────────────────────┘
          │
          ├──── 4.7kΩ ────┐
          │                │
          │         ┌──────┴──────┐
          ├────────►│ DS18B20 #1  │
          │    VCC  │ Panel Front │
   GPIO16 ◄────────┤ DATA        │
     GND ─┤────────►│ GND         │
          │         └─────────────┘
          │
          ├──── 4.7kΩ ────┐
          │                │
          │         ┌──────┴──────┐
          ├────────►│ DS18B20 #2  │
          │    VCC  │ Panel Back  │
   GPIO19 ◄────────┤ DATA        │
     GND ─┤────────►│ GND         │
          │         └─────────────┘
          │
          ├──── 10kΩ ─────┐
          │                │
   GPIO20 ◄────────────────┤
          │         ┌──────┴──────┐
          │         │ kWh Meter   │
          │         │ Pulse Out   │
     GND ─┤────────►│ COM         │
          │         └─────────────┘
          │
```

### Pyranometer Mounting

```
         ☀️ Sun
          │
          │   ┌─────────────────┐
          │   │  ML8511 Sensor  │  ← Mount level, facing sky
          │   │  (upward facing)│     No shadows 360°
          │   └────────┬────────┘     Height: above panel plane
                       │
                 Mounting pole
```

## Analytics Features

### 1. Solar Irradiance Tracking

Global Horizontal Irradiance (GHI) measured by pyranometer:

```
ADC → Voltage (mV) → Irradiance (W/m²)

  voltage_mv = (raw / 4095) × 3300
  delta_mv   = voltage_mv - baseline_mv
  irradiance = (delta_mv / cal_mv_per_unit) × 10  [W/m²]
```

Range: 0–1500 W/m², EMA filtered (α = 0.5).

### 2. Temperature Derating

Panel power output decreases as panel temperature exceeds STC (25°C):

```
derating = 1.0 - (T_panel - 25°C) × |temp_coeff| / 100

Default temp_coeff: -0.35 %/°C (typical crystalline silicon)
```

| Panel Temp | Derating Factor | Power Loss |
|-----------|----------------|------------|
| 25°C (STC) | 1.000 | 0% |
| 35°C | 0.965 | 3.5% |
| 45°C | 0.930 | 7.0% |
| 55°C | 0.895 | 10.5% |
| 65°C | 0.860 | 14.0% |
| 75°C | 0.825 | 17.5% |

### 3. Estimated Power Output

```
P_est = P_rated × (GHI / 1000) × derating × (1 - soiling%) × η_inverter
```

Where:
- `P_rated`: Panel nameplate watts-peak (configurable)
- `GHI`: Current irradiance in W/m²
- `derating`: Temperature derating factor
- `soiling%`: Estimated dust/dirt loss
- `η_inverter`: Inverter efficiency (default 0.96)

### 4. Daily Energy Yield

Integrated power over time:

```
E_daily = Σ(P_est × Δt)  [Wh]
```

Accumulated every sample interval and reported via MQTT.

### 5. Peak Sun Hours (PSH)

Equivalent hours of 1000 W/m² irradiance:

```
PSH = Σ(GHI / 1000 × Δt)  [hours]
```

Typical values: 3–7 PSH/day depending on location and season.

### 6. Performance Ratio

Ratio of actual output to theoretical maximum:

```
PR = P_actual / (P_rated × GHI / 1000)
```

| PR Range | Interpretation |
|----------|---------------|
| 0.80–0.90 | Excellent (new/clean system) |
| 0.70–0.80 | Good (normal operation) |
| 0.50–0.70 | Degraded (investigate) |
| < 0.50 | Poor (shading/fault/soiling) |

### 7. Soiling Loss Estimation

Dust accumulation model based on days since last rain:

```
soiling_loss_pct = min(days_since_rain × 0.1%, 3.0%)
```

Rain > 1 mm/hr resets the counter (natural panel washing).

### 8. Cloud Transient Detection

Detects rapid irradiance changes (> 200 W/m² between samples) caused by
passing clouds. Logged for grid stability analysis.

### 9. Sky Condition Classification

| Irradiance | Classification |
|-----------|---------------|
| < 10 W/m² | Night |
| 10–200 W/m² | Overcast |
| 200–600 W/m² | Partly Cloudy |
| > 600 W/m² | Clear |

## MQTT Topics

```
weather/{device_id}/solar          — Real-time solar analytics (JSON)
weather/{device_id}/solar/daily    — Daily summary report (JSON)
weather/{device_id}/alerts/industry — Panel temp, performance, soiling alerts
```

### Solar Payload Example

```json
{
  "irradiance_w_m2": 850.0,
  "panel_temp_front_c": 42.5,
  "panel_temp_back_c": 38.2,
  "estimated_power_w": 312.8,
  "daily_yield_wh": 1850.5,
  "peak_sun_hours": 3.2,
  "temp_derating_factor": 0.939,
  "soiling_loss_pct": 0.5,
  "performance_ratio": 0.82,
  "sky_condition": "Clear",
  "inverter_efficiency": 0.96
}
```

### Daily Summary Payload

```json
{
  "total_yield_wh": 2850.0,
  "peak_power_w": 380.5,
  "peak_irradiance_w_m2": 1050.0,
  "peak_sun_hours": 5.2,
  "avg_performance_ratio": 0.78,
  "max_panel_temp_c": 55.3,
  "temp_loss_wh": 285.0,
  "soiling_loss_wh": 14.2,
  "cloud_transients": 12
}
```

## Alerts

| Alert | Severity | Condition |
|-------|----------|-----------|
| Panel overheating | Warning | Panel temp > 75°C |
| Low performance | Warning | PR < 0.5 at GHI > 200 W/m² |
| Soiling | Info | Estimated loss > 2% |

## Power Budget Addition

| Component | Active (mA) | Sleep (µA) |
|-----------|------------|-----------|
| ML8511 pyranometer | 0.3 | 0.1 |
| DS18B20 ×2 | 3.0 | 1.5 |
| Pulse counter | 0.01 | 0 |
| **Subtotal** | **~3.3 mA** | **~1.6 µA** |

## Installation Notes

- Mount pyranometer horizontally with unobstructed sky view (no shadows)
- DS18B20 probes: adhere front probe to panel glass, back probe behind backsheet
- Use weatherproof cable glands for all sensor wiring
- Keep ESP32-S3 module shaded (avoid self-heating from direct sun)
- For multi-string systems, deploy one weather station per orientation/tilt group
