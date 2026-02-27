# Weather Station Firmware

## ESP32-S3 Weather Station — Embedded Rust Firmware

A production-grade weather station firmware written in Rust targeting the **ESP32-S3** MCU.
It collects environmental data from multiple sensors and transmits readings over
**WiFi, BLE, LoRa, UART, MQTT, and HTTP** — covering every practical communication
channel available on the platform.

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    APPLICATION LAYER                        │
│  ┌──────────┐ ┌────────────┐ ┌──────────┐ ┌────────────┐  │
│  │Scheduler │ │Data Pipeline│ │  Power   │ │    OTA     │  │
│  │          │ │  & Fusion  │ │  Mgmt    │ │  Updater   │  │
│  └──────────┘ └────────────┘ └──────────┘ └────────────┘  │
├─────────────────────────────────────────────────────────────┤
│                  COMMUNICATION LAYER                        │
│  ┌──────┐ ┌─────┐ ┌──────┐ ┌──────┐ ┌──────┐ ┌────────┐  │
│  │ WiFi │ │ BLE │ │ LoRa │ │ UART │ │ MQTT │ │  HTTP  │  │
│  └──────┘ └─────┘ └──────┘ └──────┘ └──────┘ └────────┘  │
├─────────────────────────────────────────────────────────────┤
│                     DRIVER LAYER                            │
│  ┌───────┐ ┌──────────┐ ┌──────┐ ┌──────┐ ┌───────────┐  │
│  │BME280 │ │Wind Speed│ │ Rain │ │  UV  │ │   Light   │  │
│  │Temp/  │ │& Dir     │ │Gauge │ │Index │ │  (BH1750) │  │
│  │Hum/Pr │ │(AS5600)  │ │(Tip) │ │SI1145│ │           │  │
│  └───────┘ └──────────┘ └──────┘ └──────┘ └───────────┘  │
├─────────────────────────────────────────────────────────────┤
│                 HARDWARE ABSTRACTION                        │
│  ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐ ┌───────┐  │
│  │ I2C  │ │ SPI  │ │ ADC  │ │ GPIO │ │ UART │ │ Timer │  │
│  └──────┘ └──────┘ └──────┘ └──────┘ └──────┘ └───────┘  │
├─────────────────────────────────────────────────────────────┤
│                    ESP32-S3 HARDWARE                        │
│  Xtensa LX7 Dual-Core · 512KB SRAM · WiFi · BLE 5.0      │
└─────────────────────────────────────────────────────────────┘
```

## MCU Selection — ESP32-S3

| Criterion | ESP32-S3 |
|---|---|
| CPU | Dual-core Xtensa LX7 @ 240 MHz |
| RAM | 512 KB SRAM + 8 MB PSRAM |
| Flash | 16 MB (quad SPI) |
| WiFi | 802.11 b/g/n |
| Bluetooth | BLE 5.0 |
| ADC | 2x 12-bit SAR, 20 channels |
| I2C / SPI / UART | 2 / 4 / 3 |
| GPIO | 45 programmable |
| Deep sleep | ~10 uA |
| Rust support | `esp-hal` + `esp-idf-hal` (mature) |

**Why ESP32-S3?** Best-in-class Rust embedded support via `esp-rs`, built-in WiFi + BLE
eliminates external modules, sufficient ADC channels for all analog sensors, and SPI
available for LoRa (SX1276). The dual-core architecture lets us dedicate one core to
sensor acquisition and the other to communication.

## Sensors

| Sensor | Measures | Interface | Address / Pin |
|---|---|---|---|
| BME280 | Temperature, humidity, pressure | I2C | 0x76 |
| AS5600 | Wind direction (magnetic encoder) | I2C | 0x36 |
| Anemometer | Wind speed (pulse count) | GPIO interrupt | GPIO 4 |
| Rain gauge | Rainfall (tipping bucket) | GPIO interrupt | GPIO 5 |
| SI1145 | UV index | I2C | 0x60 |
| BH1750 | Ambient light (lux) | I2C | 0x23 |

## Communication Channels

| Channel | Protocol | Use Case |
|---|---|---|
| **WiFi** | 802.11n | Primary uplink to cloud/MQTT broker |
| **BLE 5.0** | GATT | Mobile app config & live readings |
| **LoRa** | LoRaWAN (SX1276 via SPI) | Long-range low-power telemetry |
| **UART** | Serial 115200 | Debug console & wired data export |
| **MQTT** | MQTT v3.1.1 over TCP/TLS | Cloud telemetry (over WiFi) |
| **HTTP** | REST JSON API | Local web dashboard + OTA updates |

## Industry Modules

The firmware supports optional industry-specific extensions via Cargo features:

| Feature | Industry | Additional Sensors | Analytics |
|---------|----------|-------------------|-----------|
| `agriculture` | Precision Farming | Soil moisture (×2), soil temp, leaf wetness | ET₀, GDD, frost alerts, irrigation scheduling, disease risk, spray windows |
| `solar` | Photovoltaic Monitoring | Pyranometer, panel temp (×2), power meter | Irradiance, yield estimation, peak sun hours, performance ratio, soiling loss, cloud transients |
| `all-industries` | Both | All above | All above |

```bash
# Build with agriculture module
cargo build --release --features agriculture

# Build with solar module
cargo build --release --features solar

# Build with all industry modules
cargo build --release --features all-industries
```

See [docs/INDUSTRY_AGRICULTURE.md](docs/INDUSTRY_AGRICULTURE.md) and
[docs/INDUSTRY_SOLAR.md](docs/INDUSTRY_SOLAR.md) for detailed documentation.

## Building

```bash
# Install Rust + ESP toolchain
rustup install nightly
cargo install espup
espup install

# Build (base weather station)
cd firmware
cargo build --release --target xtensa-esp32s3-none-elf

# Flash
espflash flash target/xtensa-esp32s3-none-elf/release/weather-station
```

## Project Structure

```
weather/
├── README.md
├── docs/
│   ├── ARCHITECTURE.md
│   ├── REQUIREMENTS.md
│   ├── API.md
│   ├── HARDWARE.md
│   └── diagrams/
│       ├── system_architecture.md
│       ├── circuit_diagram.md
│       ├── communication_flow.md
│       ├── state_machine.md
│       ├── data_pipeline.md
│       └── sequence_diagrams.md
├── firmware/
│   ├── Cargo.toml
│   ├── build.rs
│   ├── .cargo/config.toml
│   ├── memory.x
│   └── src/
│       ├── main.rs
│       ├── config.rs
│       ├── error.rs
│       ├── drivers/
│       │   ├── mod.rs
│       │   ├── bme280.rs
│       │   ├── wind.rs
│       │   ├── rain.rs
│       │   ├── uv.rs
│       │   ├── light.rs
│       │   ├── soil_moisture.rs  (agriculture)
│       │   ├── soil_temp.rs      (agriculture/solar)
│       │   ├── leaf_wetness.rs   (agriculture)
│       │   └── pyranometer.rs    (solar)
│       ├── comms/
│       │   ├── mod.rs
│       │   ├── wifi.rs
│       │   ├── ble.rs
│       │   ├── lora.rs
│       │   ├── uart_console.rs
│       │   ├── mqtt.rs
│       │   └── http.rs
│       ├── core/
│       │   ├── mod.rs
│       │   ├── scheduler.rs
│       │   ├── data_pipeline.rs
│       │   ├── power.rs
│       │   └── ota.rs
│       ├── industry/
│       │   ├── mod.rs
│       │   ├── agriculture.rs    (agriculture)
│       │   └── solar.rs          (solar)
│       ├── storage/
│       │   ├── mod.rs
│       │   └── flash.rs
│       └── utils/
│           ├── mod.rs
│           ├── fmt.rs
│           ├── ring_buffer.rs
│           └── crc.rs
└── tests/
    └── integration.rs
```

## License

MIT
