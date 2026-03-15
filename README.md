# Weather Station Firmware

## STM32F407 Weather Station — Embedded Rust Firmware

A production-grade weather station firmware written in Rust targeting the **STM32F407VGT6** MCU.
It collects environmental data from multiple sensors and transmits readings over
**WiFi (ATWINC1500), BLE (RN4870), LoRa, UART, MQTT, and HTTP** — with optional
Ethernet (W5500) and Cellular (SIM7600E-H) connectivity.

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
│                   STM32F407 HARDWARE                        │
│  ARM Cortex-M4F @ 168 MHz · 192KB SRAM · 1MB Flash        │
└─────────────────────────────────────────────────────────────┘
```

## MCU Selection — STM32F407VGT6

| Criterion | STM32F407VGT6 |
|---|---|
| CPU | ARM Cortex-M4F @ 168 MHz (with FPU) |
| RAM | 192 KB SRAM (128 KB + 64 KB CCM) |
| Flash | 1 MB internal |
| WiFi | ATWINC1500 module (SPI3) |
| Bluetooth | RN4870 BLE 5.0 module (USART3) |
| ADC | 3x 12-bit SAR, 16 channels |
| I2C / SPI / UART | 3 / 3 / 4 |
| GPIO | 82 programmable |
| Standby | ~2 µA |
| Temp range | -40°C to +105°C (Industrial) |
| Rust support | `stm32f4xx-hal` + `cortex-m-rt` (mature) |

**Why STM32F407?** Industrial temperature range for outdoor deployment, hardware FPU for
sensor math, three SPI buses for concurrent LoRa/Ethernet/WiFi, dedicated UART for each
communication module, and excellent Rust embedded-hal support. Single-MCU architecture
simplifies firmware (one codebase, one toolchain).

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
| `india` | Indian Regional | PM2.5 particulate sensor | Monsoon tracking, IMD heat wave alerts, cyclone detection, NAQI air quality, IST timezone, IN865 LoRa |
| `all-industries` | All | All above | All above |
| `cellular` | Remote Sites | SIM7600E-H LTE modem | Cellular uplink for remote stations |
| `ethernet` | Industrial | W5500 wired Ethernet | Reliable wired connectivity + Modbus TCP |
| `all-comms` | All Comms | Cellular + Ethernet | All communication channels |
| `full-system` | Everything | All sensors + all comms | All analytics + SCADA + Cloud |

### Operating Modes

| Mode | Cloud (MQTT/HTTP) | SCADA (Modbus RS485) | Selection |
|------|:-----------------:|:--------------------:|-----------|
| **Cloud** | Yes | — | DIP 0:0 |
| **SCADA** | Yes | Yes | DIP 0:1 |
| **Hybrid** | Yes | Yes | DIP 1:0 (default) |

```bash
# Build with agriculture module
cargo build --release --features agriculture

# Build with solar module
cargo build --release --features solar

# Build with India regional module
cargo build --release --features india

# Build with all industry modules
cargo build --release --features all-industries

# Build with cellular modem support
cargo build --release --features cellular

# Build with Ethernet support
cargo build --release --features ethernet

# Full system: all industries + all comms
cargo build --release --features full-system
```

See [docs/INDUSTRY_AGRICULTURE.md](docs/INDUSTRY_AGRICULTURE.md),
[docs/INDUSTRY_SOLAR.md](docs/INDUSTRY_SOLAR.md),
[docs/INDUSTRY_INDIA.md](docs/INDUSTRY_INDIA.md), and
[docs/STM32_SCADA.md](docs/STM32_SCADA.md) for detailed documentation.

## Building

```bash
# Install Rust + ARM toolchain
rustup install nightly
rustup target add thumbv7em-none-eabihf
cargo install probe-rs-tools

# Build (base weather station)
cd firmware
cargo build --release --target thumbv7em-none-eabihf

# Flash via SWD (ST-Link V2)
probe-rs run --chip STM32F407VGTx target/thumbv7em-none-eabihf/release/weather-station
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
│   ├── INDUSTRY_INDIA.md
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
│       │   ├── pyranometer.rs    (solar)
│       │   ├── pm25.rs           (india)
│       │   ├── stm32f407.rs     (MCU HAL + pin map)
│       │   ├── atwinc1500.rs   (WiFi SPI driver)
│       │   └── rn4870.rs       (BLE UART driver)
│       ├── comms/
│       │   ├── mod.rs
│       │   ├── wifi.rs
│       │   ├── ble.rs
│       │   ├── lora.rs
│       │   ├── uart_console.rs
│       │   ├── mqtt.rs
│       │   ├── http.rs
│       │   └── modbus_rtu.rs    (Modbus RS485 slave)
│       ├── core/
│       │   ├── mod.rs
│       │   ├── scheduler.rs
│       │   ├── data_pipeline.rs
│       │   ├── power.rs
│       │   ├── ota.rs
│       │   └── mode_manager.rs  (SCADA/Cloud/Hybrid)
│       ├── industry/
│       │   ├── mod.rs
│       │   ├── agriculture.rs    (agriculture)
│       │   ├── solar.rs          (solar)
│       │   └── india.rs          (india)
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
