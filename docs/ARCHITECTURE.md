# System Architecture

## 1. Overview

The weather station firmware follows a **layered architecture** with strict
dependency rules: upper layers depend on lower layers, never the reverse.

```
┌─────────────────────────────────────────────────────────────────────┐
│ LAYER 4 — APPLICATION                                               │
│                                                                     │
│  Scheduler ──► Data Pipeline ──► Power Manager ──► OTA Updater     │
│       │              │                │                              │
│       ▼              ▼                ▼                              │
├─────────────────────────────────────────────────────────────────────┤
│ LAYER 3 — COMMUNICATION                                             │
│                                                                     │
│  WiFi Manager ──► MQTT Client ──► HTTP Server                      │
│  BLE GATT Server                                                    │
│  LoRa Transceiver (SX1276)                                         │
│  UART Console                                                       │
├─────────────────────────────────────────────────────────────────────┤
│ LAYER 2 — DRIVERS                                                   │
│                                                                     │
│  BME280 │ AS5600 │ Anemometer │ Rain Gauge │ SI1145 │ BH1750      │
├─────────────────────────────────────────────────────────────────────┤
│ LAYER 1 — HAL (stm32f4xx-hal)                                       │
│                                                                     │
│  I2C │ SPI │ ADC │ GPIO │ UART │ Timer │ RTC │ NVS                │
├─────────────────────────────────────────────────────────────────────┤
│ LAYER 0 — HARDWARE                                                  │
│                                                                     │
│  STM32F407VGT6 — ARM Cortex-M4F @ 168 MHz                          │
│  192 KB SRAM (128 KB + 64 KB CCM) + 1 MB Flash                    │
│  ATWINC1500 WiFi (SPI3) · RN4870 BLE (USART3)                     │
└─────────────────────────────────────────────────────────────────────┘
```

## 2. Single-Core Cooperative Scheduling

The STM32F407 runs a cooperative task scheduler on a single Cortex-M4F core.
Tasks are time-sliced by priority, with ISRs handling time-critical pulse counting.

| Task Group | Responsibility |
|------------|----------------|
| **Sensor tasks** | Sensor sampling, data fusion, calibration, alerts |
| **Comms tasks** | WiFi poll, BLE poll, LoRa TX/RX, MQTT, HTTP, UART, Modbus RTU |
| **System tasks** | Power management, OTA updates, watchdog feed, status reporting |

All tasks share a single execution context with cooperative yielding.

## 3. Data Flow

```
Sensors ──► Raw ADC / I2C ──► Driver Layer ──► Calibration & Filtering
    │                                                │
    ▼                                                ▼
GPIO ISRs ──► Pulse Counters ──────────────► Data Pipeline
                                                     │
                                         ┌───────────┼───────────┐
                                         ▼           ▼           ▼
                                      Flash       MQTT/HTTP    BLE/LoRa
                                     Storage      (WiFi)      Broadcast
```

### 3.1 Sampling Schedule

| Sensor | Interval | Method |
|--------|----------|--------|
| BME280 (T/H/P) | 10 s | Polled (I2C) |
| Wind speed | Continuous | GPIO ISR pulse count, computed every 5 s |
| Wind direction | 5 s | Polled (I2C — AS5600 angle register) |
| Rain gauge | Continuous | GPIO ISR tip count, accumulated |
| UV index | 30 s | Polled (I2C) |
| Ambient light | 30 s | Polled (I2C) |

### 3.2 Data Fusion

Raw sensor values go through a pipeline:

1. **Calibration** — Apply per-sensor offset/gain from NVS
2. **Filtering** — Exponential moving average (configurable α)
3. **Validation** — Range checks, stuck-sensor detection
4. **Packaging** — Structured `WeatherReading` with timestamp

## 4. Communication Architecture

### 4.1 Channel Priority & Fallback

```
PRIMARY:   WiFi ──► MQTT Broker ──► Cloud Dashboard
                │
                └──► HTTP REST API (local access)

SECONDARY: BLE GATT ──► Mobile App (config + live data)

TERTIARY:  LoRa ──► LoRaWAN Gateway ──► Network Server

DEBUG:     UART ──► Serial Console (always active)
```

If WiFi is unavailable, data is buffered to flash and LoRa becomes the primary
uplink. BLE remains available for local configuration regardless of WiFi state.

### 4.2 MQTT Topic Structure

```
weather/{device_id}/telemetry      — periodic readings (JSON)
weather/{device_id}/status         — heartbeat, battery, RSSI
weather/{device_id}/alerts         — threshold-triggered alerts
weather/{device_id}/cmd            — remote commands (subscribe)
weather/{device_id}/ota            — OTA control channel
```

### 4.3 BLE GATT Services

| Service UUID | Characteristic | Properties |
|---|---|---|
| `0x181A` (Env Sensing) | Temperature | Read, Notify |
| | Humidity | Read, Notify |
| | Pressure | Read, Notify |
| `0xFFE0` (Custom) | Wind Speed | Read, Notify |
| | Wind Direction | Read, Notify |
| | Rain Accumulation | Read, Notify |
| | UV Index | Read, Notify |
| | Light Level | Read, Notify |
| `0xFFE1` (Config) | Sampling Interval | Read, Write |
| | WiFi SSID | Write |
| | WiFi Password | Write |
| | Device Name | Read, Write |

## 5. Power Management

### 5.1 Operating Modes

| Mode | Current Draw | Duration | Trigger |
|------|-------------|----------|---------|
| **Active** | ~170 mA | During sampling + TX | Scheduler wake |
| **Sleep (WFI)** | ~20 mA | Between tasks | Auto after TX |
| **Stop** | ~20 µA | Idle periods | Configurable |
| **Standby** | ~2 µA | Extended idle | Battery < 20% |

### 5.2 Battery Monitoring

- ADC channel monitors battery voltage via resistive divider
- Low-battery threshold triggers deep sleep with LoRa-only wake cycle
- Critical battery (<3.3V) triggers graceful shutdown after final status message

## 6. OTA Update Flow

```
1. Device checks /ota endpoint on boot + every 6 hours
2. Server responds with firmware version + SHA256 hash
3. If newer version → download binary over HTTPS
4. Verify SHA256 → write to OTA partition
5. Set boot partition → reboot
6. New firmware validates → mark as stable
7. Failure → rollback to previous partition
```

## 7. Error Handling Strategy

- **Sensor failure**: Mark channel as degraded, report NaN, continue other sensors
- **WiFi failure**: Buffer data to flash, retry with exponential backoff
- **MQTT failure**: Queue messages (up to 1000), replay on reconnect
- **LoRa failure**: Fall back to store-and-forward
- **Flash full**: Overwrite oldest records (circular buffer)
- **Watchdog**: Hardware WDT resets system if main loop stalls > 30 s
