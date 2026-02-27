# API Reference

## 1. MQTT Topics

### 1.1 Telemetry (Publish)

**Topic**: `weather/{device_id}/telemetry`
**QoS**: 1
**Interval**: Configurable (default 30 s)

```json
{
  "timestamp": "2026-02-26T14:30:00Z",
  "temperature_c": 23.5,
  "humidity_pct": 65.2,
  "pressure_hpa": 1013.25,
  "wind_speed_kmh": 12.3,
  "wind_direction_deg": 225,
  "rain_mm_hour": 0.4,
  "rain_mm_total": 12.6,
  "uv_index": 3.2,
  "light_lux": 45000,
  "heat_index_c": 24.1,
  "dew_point_c": 16.8,
  "wind_chill_c": null
}
```

### 1.2 Status (Publish)

**Topic**: `weather/{device_id}/status`
**QoS**: 1
**Interval**: 60 s

```json
{
  "timestamp": "2026-02-26T14:30:00Z",
  "uptime_s": 86400,
  "battery_v": 3.85,
  "battery_pct": 72,
  "wifi_rssi_dbm": -45,
  "free_heap_bytes": 180000,
  "flash_used_pct": 34,
  "firmware_version": "1.0.0",
  "sensor_status": {
    "bme280": "ok",
    "wind": "ok",
    "rain": "ok",
    "uv": "ok",
    "light": "degraded"
  }
}
```

### 1.3 Alerts (Publish)

**Topic**: `weather/{device_id}/alerts`
**QoS**: 2

```json
{
  "timestamp": "2026-02-26T14:30:00Z",
  "alert_type": "threshold_exceeded",
  "sensor": "wind_speed",
  "value": 95.2,
  "threshold": 90.0,
  "unit": "km/h",
  "message": "High wind speed alert"
}
```

### 1.4 Commands (Subscribe)

**Topic**: `weather/{device_id}/cmd`
**QoS**: 1

```json
{
  "command": "set_interval",
  "params": {
    "telemetry_interval_s": 15
  }
}
```

Supported commands:
| Command | Description |
|---|---|
| `set_interval` | Change telemetry interval |
| `reboot` | Restart device |
| `calibrate` | Trigger sensor calibration |
| `status` | Request immediate status report |
| `sleep` | Enter deep sleep for N seconds |
| `factory_reset` | Erase NVS and reboot |

## 2. HTTP REST API

### 2.1 GET /api/v1/current

Returns the latest sensor readings.

**Response** (200):
```json
{
  "timestamp": "2026-02-26T14:30:00Z",
  "temperature_c": 23.5,
  "humidity_pct": 65.2,
  "pressure_hpa": 1013.25,
  "wind_speed_kmh": 12.3,
  "wind_direction_deg": 225,
  "rain_mm_hour": 0.4,
  "rain_mm_total": 12.6,
  "uv_index": 3.2,
  "light_lux": 45000
}
```

### 2.2 GET /api/v1/history

Returns historical readings.

**Query params**:
- `hours` (optional, default=1): Number of hours of history
- `resolution` (optional, default=auto): `raw`, `1min`, `5min`, `15min`, `1hour`

**Response** (200):
```json
{
  "from": "2026-02-26T13:30:00Z",
  "to": "2026-02-26T14:30:00Z",
  "resolution": "5min",
  "count": 12,
  "readings": [ ... ]
}
```

### 2.3 GET /api/v1/status

Returns device status.

### 2.4 POST /api/v1/config

Update device configuration.

**Request body**:
```json
{
  "telemetry_interval_s": 15,
  "mqtt_broker": "mqtt.example.com",
  "mqtt_port": 8883,
  "alerts": {
    "wind_speed_kmh": 90.0,
    "temperature_high_c": 40.0,
    "temperature_low_c": -10.0
  }
}
```

### 2.5 POST /api/v1/ota

Upload firmware binary for OTA update.

**Content-Type**: `application/octet-stream`
**Header**: `X-Firmware-SHA256: <hex digest>`

**Response** (202):
```json
{
  "status": "accepted",
  "message": "Firmware update in progress, device will reboot"
}
```

### 2.6 GET /

Minimal HTML status page showing current readings and device info.

## 3. BLE GATT Profile

### 3.1 Environmental Sensing Service (UUID: 0x181A)

| Characteristic | UUID | Type | Properties |
|---|---|---|---|
| Temperature | 0x2A6E | sint16 (0.01°C) | Read, Notify |
| Humidity | 0x2A6F | uint16 (0.01%) | Read, Notify |
| Pressure | 0x2A6D | uint32 (0.1 Pa) | Read, Notify |

### 3.2 Custom Weather Service (UUID: 0000FFE0-0000-1000-8000-00805F9B34FB)

| Characteristic | UUID | Type | Properties |
|---|---|---|---|
| Wind Speed | FFE0-0001 | uint16 (0.1 km/h) | Read, Notify |
| Wind Direction | FFE0-0002 | uint16 (degrees) | Read, Notify |
| Rain Accum | FFE0-0003 | uint16 (0.1 mm) | Read, Notify |
| UV Index | FFE0-0004 | uint16 (0.1) | Read, Notify |
| Light Level | FFE0-0005 | uint32 (lux) | Read, Notify |
| All Readings | FFE0-00FF | JSON blob | Read, Notify |

### 3.3 Configuration Service (UUID: 0000FFE1-0000-1000-8000-00805F9B34FB)

| Characteristic | UUID | Type | Properties |
|---|---|---|---|
| Sampling Rate | FFE1-0001 | uint16 (seconds) | Read, Write |
| WiFi SSID | FFE1-0010 | UTF-8 string | Write |
| WiFi Password | FFE1-0011 | UTF-8 string | Write |
| Device Name | FFE1-0020 | UTF-8 string | Read, Write |
| WiFi Status | FFE1-0030 | uint8 (enum) | Read, Notify |

## 4. UART Console Commands

Baud: 115200, 8N1

| Command | Description | Example |
|---|---|---|
| `status` | Print device status | `> status` |
| `reading` | Print latest readings | `> reading` |
| `config get <key>` | Read config value | `> config get interval` |
| `config set <key> <val>` | Write config value | `> config set interval 15` |
| `calibrate <sensor>` | Run calibration | `> calibrate bme280` |
| `reset` | Soft reset | `> reset` |
| `factory-reset` | Erase NVS + reset | `> factory-reset` |
| `dump <sensor> <count>` | Raw data dump | `> dump bme280 100` |
| `lora send <msg>` | Manual LoRa transmit | `> lora send test` |
| `wifi scan` | Scan WiFi networks | `> wifi scan` |
| `help` | Show command list | `> help` |

## 5. LoRa Packet Format

Compact binary format to minimize airtime:

```
Byte 0:     Packet type (0x01 = telemetry)
Byte 1-2:   Device ID (uint16)
Byte 3-6:   Timestamp (uint32, epoch seconds)
Byte 7-8:   Temperature (int16, ×100 °C)
Byte 9-10:  Humidity (uint16, ×100 %)
Byte 11-14: Pressure (uint32, ×100 Pa)
Byte 15-16: Wind speed (uint16, ×10 km/h)
Byte 17-18: Wind direction (uint16, degrees)
Byte 19-20: Rain total (uint16, ×10 mm)
Byte 21:    UV index (uint8, ×10)
Byte 22-23: Light (uint16, lux, capped 65535)
Byte 24:    Battery % (uint8)
Byte 25-26: CRC16
Total: 27 bytes
```
