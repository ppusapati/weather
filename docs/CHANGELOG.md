# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [2.1.0] - 2026-03-15

### Added — SD Card Storage
- **SD card data logging** (`--features sdcard`): micro-SD card on SPI2 shared bus (CS=PD10, DET=PD11)
- FAT32 CSV logger with daily file rotation (`/weather/YYYY-MM-DD.csv`)
- 512-byte sector-aligned write buffer with 60s periodic flush
- Automatic fallback to internal flash when SD card is not inserted
- New `FlushSdCard` scheduler task for periodic buffer flushing
- Updated BOM with micro-SD card holder (Molex 5031821852), decoupling cap, detect pull-up
- `sdcard` feature added to `all-comms` and `full-system` feature groups

## [2.0.0] - 2026-03-15

### Changed — Single-MCU Architecture
- **BREAKING**: Migrated from dual-MCU (ESP32-S3 + STM32F407) to single-MCU (STM32F407VGT6 only)
- Replaced ESP32-S3 built-in WiFi with **ATWINC1500** module on SPI3
- Replaced ESP32-S3 built-in BLE with **RN4870** module on USART3
- Added optional **W5500 Ethernet** on SPI2 (`--features ethernet`)
- Added optional **SIM7600E-H Cellular** on UART4 (`--features cellular`)
- Removed ESP32-S3 bridge protocol (USART3 now used for RN4870 BLE)
- Removed `stm32` feature flag — STM32F407 is now the only target
- Changed toolchain: `esp-hal` → `stm32f4xx-hal`, `esp-alloc` → `embedded-alloc`
- Changed target: `xtensa-esp32s3-none-elf` → `thumbv7em-none-eabihf`
- Changed flashing: `esptool` → `probe-rs` / `cargo-flash` via SWD
- Changed power modes: Modem/Light/Deep sleep → Sleep (WFI) / Stop / Standby
- Reduced heap from 384 KB to 48 KB (STM32 SRAM constraint)
- Updated KiCad schematics, BOM, and symbol library for new modules
- Updated all documentation to reflect single-MCU architecture

## [0.2.0] - 2026-02-27

### Added — Industry Modules
- **Agriculture module** (`--features agriculture`):
  - Sensor drivers: capacitive soil moisture (dual-depth), DS18B20 soil temperature, leaf wetness
  - Evapotranspiration (ET₀) — FAO Penman-Monteith with Hargreaves-Samani fallback
  - Growing Degree Days (GDD) — configurable base temperature for 5 crop types
  - Frost alerts — multi-level (Watch/Warning/Critical) with humidity-adjusted radiative frost
  - Irrigation scheduling — soil moisture depletion model (field capacity to wilting point)
  - Disease risk index — Smith period model (leaf wetness duration × temperature)
  - Spray window assessment — wind, rain, and temperature inversion detection
- **Solar module** (`--features solar`):
  - Sensor drivers: ML8511 pyranometer, DS18B20 panel temperature (front/back)
  - Solar irradiance tracking (GHI) with ADC-to-W/m² calibration
  - Temperature derating — panel power loss from overheating (-0.35%/°C above STC)
  - Estimated power output — incorporates irradiance, derating, soiling, inverter efficiency
  - Daily energy yield integration (Wh) and peak sun hours (PSH) accumulation
  - Performance ratio — actual vs theoretical power ratio with daily averaging
  - Soiling loss estimation — days-since-rain accumulation model, rain auto-reset
  - Cloud transient detection — rapid irradiance change alerting
  - Sky condition classification (Clear/PartlyCloudy/Overcast/Night)
  - Daily summary reports (total yield, peak power, cloud transients)
- Cargo feature flags: `agriculture`, `solar`, `all-industries`
- Industry-specific MQTT topics and alert publishing
- Industry sensor status tracking in `SensorStatusMap`
- Industry hardware documentation with circuit diagrams and pin mappings
- Scheduler dynamically adds industry tasks based on enabled features

## [0.1.0] - 2026-02-27

### Added
- Initial firmware implementation for weather station
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
