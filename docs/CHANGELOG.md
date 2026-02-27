# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] - 2026-02-27

### Added
- Initial firmware implementation for ESP32-S3 weather station
- **Sensors**: BME280 (temperature/humidity/pressure), AS5600 (wind direction), anemometer (wind speed), tipping bucket rain gauge, SI1145 (UV), BH1750 (ambient light)
- **Communication channels**: WiFi, BLE 5.0 GATT, LoRa SX1276, MQTT v3.1.1, HTTP REST API, UART serial console
- **Data pipeline**: 7-stage processing (acquisition, calibration, EMA filtering, validation, stuck sensor detection, derived values, packaging)
- **Derived values**: Heat index (Rothfusz regression), dew point (Magnus formula), wind chill
- **Power management**: 4 modes (active, modem sleep, light sleep, deep sleep) with LiPo battery monitoring and solar charging support
- **OTA updates**: Dual A/B partition scheme with SHA-256 verification and automatic rollback
- **Flash storage**: Circular buffer with CRC-16 integrity checks for offline data buffering
- **BLE provisioning**: WiFi credential configuration via BLE characteristic writes
- **UART console**: Interactive debug commands (status, reading, config, calibrate, dump, reset)
- **Documentation**: Architecture guide, hardware design, API reference, requirements, circuit diagrams, state machines, sequence diagrams, data pipeline diagrams
- **Tests**: Host-side unit tests for CRC, data validation, derived values, EMA filtering, battery voltage

### Quality Improvements
- Extracted shared `HeaplessWriter` formatter to `utils/fmt.rs` with `format_heapless!` macro
- Replaced all silent `let _ = Result` patterns with explicit error logging
- Added SAFETY documentation for all unsafe blocks
- Replaced 20+ magic numbers with named constants in `config.rs`
- Fixed `unwrap()` calls with `unwrap_or_default()` for robustness
- Aligned power consumption figures between architecture and hardware docs
