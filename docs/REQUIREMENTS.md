# Requirements Specification

## 1. Functional Requirements

### FR-1: Sensor Data Acquisition
- **FR-1.1**: Read temperature (±0.5°C), humidity (±3%), barometric pressure (±1 hPa) from BME280
- **FR-1.2**: Measure wind speed (0–160 km/h, ±2 km/h) via pulse-counting anemometer
- **FR-1.3**: Measure wind direction (0–359°, ±3°) via AS5600 magnetic encoder
- **FR-1.4**: Accumulate rainfall (0.2 mm per tip) via tipping-bucket gauge
- **FR-1.5**: Read UV index (0–15) from SI1145
- **FR-1.6**: Read ambient light (1–65535 lux) from BH1750
- **FR-1.7**: All sensor readings timestamped with RTC (synced via NTP when WiFi available)

### FR-2: Data Processing
- **FR-2.1**: Apply calibration offsets stored in NVS flash
- **FR-2.2**: Filter readings with configurable exponential moving average
- **FR-2.3**: Validate readings against physical range limits
- **FR-2.4**: Compute derived values: heat index, wind chill, dew point
- **FR-2.5**: Generate alerts when readings exceed configurable thresholds

### FR-3: WiFi Communication
- **FR-3.1**: Connect to WPA2 network with credentials stored in NVS
- **FR-3.2**: Auto-reconnect with exponential backoff on disconnection
- **FR-3.3**: Support provisioning via BLE when no WiFi credentials stored

### FR-4: MQTT Telemetry
- **FR-4.1**: Publish readings to MQTT broker at configurable interval (default 30 s)
- **FR-4.2**: Support TLS-encrypted connections
- **FR-4.3**: Buffer messages during disconnection (up to 1000 messages)
- **FR-4.4**: Subscribe to command topic for remote configuration

### FR-5: HTTP REST API
- **FR-5.1**: Serve current readings at `GET /api/v1/current`
- **FR-5.2**: Serve historical data at `GET /api/v1/history?hours=N`
- **FR-5.3**: Accept configuration changes at `POST /api/v1/config`
- **FR-5.4**: Serve OTA firmware upload at `POST /api/v1/ota`
- **FR-5.5**: Serve a minimal status page at `GET /`

### FR-6: BLE Interface
- **FR-6.1**: Advertise Environmental Sensing service (0x181A)
- **FR-6.2**: Support GATT read + notify for all sensor values
- **FR-6.3**: Support GATT write for configuration parameters
- **FR-6.4**: WiFi provisioning via custom BLE characteristic

### FR-7: LoRa Communication
- **FR-7.1**: Transmit compressed readings via SX1276 (SPI)
- **FR-7.2**: Support LoRaWAN Class A operation
- **FR-7.3**: Configurable spreading factor (SF7–SF12)
- **FR-7.4**: Serve as fallback when WiFi unavailable

### FR-8: UART Console
- **FR-8.1**: Output structured log messages at 115200 baud
- **FR-8.2**: Accept diagnostic commands (status, reset, calibrate)
- **FR-8.3**: Support raw data dump mode for debugging

### FR-9: Data Storage
- **FR-9.1**: Store readings in flash as circular buffer (min 7 days at 30 s interval)
- **FR-9.2**: Store configuration in NVS partition
- **FR-9.3**: Store calibration data in NVS partition

### FR-10: OTA Updates
- **FR-10.1**: Check for updates on boot and periodically (6-hour interval)
- **FR-10.2**: Download and verify firmware image (SHA-256)
- **FR-10.3**: Automatic rollback on boot failure

## 2. Non-Functional Requirements

### NFR-1: Performance
- Sensor read cycle completes within 500 ms
- MQTT publish latency < 2 s from reading to broker
- HTTP API response time < 200 ms
- BLE notification latency < 500 ms

### NFR-2: Reliability
- Mean time between failures > 30 days continuous operation
- Graceful degradation: individual sensor failure does not halt system
- Watchdog timer (30 s) prevents permanent hangs
- Data preserved across unexpected reboots

### NFR-3: Power Efficiency
- Active mode current < 200 mA average
- Deep sleep current < 50 µA
- Support solar panel + LiPo battery operation
- Minimum 48-hour operation on 3000 mAh battery (no solar)

### NFR-4: Security
- All WiFi traffic encrypted (WPA2)
- MQTT over TLS (optional, configurable)
- BLE pairing required for write operations
- OTA images cryptographically verified
- No plaintext credentials in firmware binary

### NFR-5: Maintainability
- Modular driver architecture — add sensor without modifying core
- All magic numbers defined as named constants
- Comprehensive error types with context
- Structured logging with severity levels
