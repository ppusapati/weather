# STM32F407 + SCADA/Cloud/Hybrid Operating Modes

## Overview

The weather station uses a single-MCU architecture with the **STM32F407VGT6**
handling all sensor acquisition, SCADA (Modbus RS485), and cloud connectivity
via external modules: ATWINC1500 (WiFi), RN4870 (BLE), W5500 (Ethernet),
SIM7600E-H (Cellular).

```bash
# SCADA + all industry modules
cargo build --release --features all-industries

# Full system (all features + all comms)
cargo build --release --features full-system
```

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                    STM32F407VGT6 (Single MCU)                       │
│              ARM Cortex-M4F @ 168 MHz, -40/+105°C                   │
│              1 MB Flash / 192 KB SRAM / Hardware FPU                │
├─────────────────────────────────────────────────────────────────────┤
│ SPI1 ─── SX1276 LoRa (PA4-PA7, PC4, PC5)                           │
│ SPI2 ─── W5500 Ethernet (PB12-PB15, PD3, PD4) [optional]           │
│ SPI3 ─── ATWINC1500 WiFi (PB3-PB5, PE3-PE6)                        │
│ USART2 ── MAX3485 RS485 Modbus (PA2, PA3, PA1)                     │
│ USART3 ── RN4870 BLE (PB10, PB11, PD8, PD9)                       │
│ UART4 ── SIM7600E-H Cellular (PC10, PC11) [optional]               │
│ USART1 ── Debug Console (PA9, PA10)                                 │
│ I2C1 ─── Sensors: BME280, AS5600, SI1145, BH1750 (PB6, PB7)       │
└─────────┬───────────────────────────────────────────────────────────┘
          │
          │ USART2 (RS485)
          │
    ┌─────┴─────┐         ┌───────────────┐
    │  MAX3485   │─── A ───┤               │
    │  RS485     │─── B ───┤  SCADA / PLC  │
    │  Xcvr      │─── GND ─┤  Master       │
    └───────────┘         └───────────────┘
```

## Operating Modes

Three operating modes selectable via DIP switch or software configuration:

| Mode | DIP (PE1:PE0) | Cloud (MQTT/HTTP) | SCADA (Modbus RS485) | Use Case |
|------|:----------:|:-----------------:|:--------------------:|----------|
| **Cloud** | 0:0 | Yes | — | Standard IoT deployment |
| **SCADA** | 0:1 | — | Yes | Air-gapped industrial |
| **Hybrid** | 1:0 | Yes | Yes | Full capability |

### Cloud Mode
- ATWINC1500 handles WiFi, MQTT, HTTP connectivity
- RN4870 provides BLE for mobile app configuration
- LoRa available for long-range backup
- Standard IoT deployment

### SCADA Mode
- Modbus RTU slave on RS485 bus
- All weather data exposed as Modbus registers
- No internet/cloud connectivity — fully air-gapped
- Ideal for integration with existing SCADA/PLC systems
- Up to 32 devices on single RS485 bus

### Hybrid Mode (Default)
- Both cloud and SCADA active simultaneously
- ATWINC1500/W5500 publishes to MQTT/HTTP
- Modbus RTU responds to SCADA master queries
- Maximum observability and integration flexibility

## Modbus Register Map

### Input Registers (Read-Only, FC 0x04)

| Address | Description | Unit | Scale |
|---------|-------------|------|-------|
| 30001 | Temperature | °C × 100 | i16 |
| 30002 | Humidity | % × 100 | u16 |
| 30003 | Pressure (high) | hPa × 100 | u32 |
| 30004 | Pressure (low) | | |
| 30005 | Wind Speed | km/h × 100 | u16 |
| 30006 | Wind Direction | degrees | u16 |
| 30007 | Rain Rate | mm/h × 100 | u16 |
| 30008 | Rain Accumulation | mm × 100 | u16 |
| 30009 | UV Index | index × 100 | u16 |
| 30010 | Light (high) | lux | u32 |
| 30011 | Light (low) | | |
| 30012 | Heat Index | °C × 100 | i16 |
| 30013 | Dew Point | °C × 100 | i16 |
| 30014 | Wind Chill | °C × 100 | i16 |
| 30015 | Battery Voltage | mV | u16 |
| 30016 | Battery Percentage | % | u16 |
| 30017 | PM2.5 (India) | µg/m³ × 10 | u16 |
| 30018 | AQI NAQI (India) | index | u16 |
| 30019 | Soil Moisture Shallow | % × 100 | u16 |
| 30020 | Soil Moisture Deep | % × 100 | u16 |
| 30021 | Soil Temperature | °C × 100 | i16 |
| 30022 | Solar Irradiance | W/m² × 10 | u16 |
| 30023 | Panel Temperature | °C × 100 | i16 |
| 30024 | Est. Solar Power | W × 10 | u16 |

### Holding Registers (R/W, FC 0x03/0x06/0x10)

| Address | Description | Range | Default |
|---------|-------------|-------|---------|
| 40001 | Slave Address | 1–247 | 1 |
| 40002 | Baud Rate Code | 0=2400, 1=9600, 2=19200, 3=38400 | 1 |
| 40003 | Sensor Read Interval | 1–3600 s | 10 |
| 40004 | Operating Mode | 0=Cloud, 1=SCADA, 2=Hybrid | 2 |
| 40005 | Temp Cal Offset | °C × 100 | 0 |
| 40006 | Humidity Cal Offset | % × 100 | 0 |
| 40007 | Pressure Cal Offset | hPa × 100 | 0 |
| 40008 | GDD Base Temp | °C × 100 | 1000 |
| 40009 | India Region | 0=Plains, 1=Coastal, 2=Hill | 0 |
| 40010 | Device ID | 0–65535 | 1 |

### Discrete Inputs (Read-Only, FC 0x02)

| Address | Description |
|---------|-------------|
| 10001 | Heat Wave Active |
| 10002 | Severe Heat Wave |
| 10003 | Cyclone Watch |
| 10004 | Cyclone Warning |
| 10005 | Cyclone Severe |
| 10006 | AQI Severe |
| 10007 | Frost Warning |
| 10008 | Sensor Fault |

## STM32F407 Pin Assignment

| Pin | Function | Connected To |
|-----|----------|-------------|
| PB6 | I2C1_SCL | BME280, AS5600, SI1145, BH1750 |
| PB7 | I2C1_SDA | (shared bus) |
| PA5 | SPI1_SCK | SX1276 LoRa |
| PA6 | SPI1_MISO | SX1276 LoRa |
| PA7 | SPI1_MOSI | SX1276 LoRa |
| PA4 | SPI1_NSS | SX1276 CS |
| PC4 | GPIO | SX1276 RST |
| PC5 | GPIO (EXTI) | SX1276 DIO0 |
| PA2 | USART2_TX | MAX3485 DI |
| PA3 | USART2_RX | MAX3485 RO |
| PA1 | GPIO | MAX3485 DE/RE |
| PA9 | USART1_TX | Debug header |
| PA10 | USART1_RX | Debug header |
| PB10 | USART3_TX | RN4870 RX |
| PB11 | USART3_RX | RN4870 TX |
| PA0 | ADC1_CH0 | Battery divider |
| PC1 | ADC1_CH11 | PM2.5 sensor |
| PB0 | TIM3_CH3 | Wind speed pulse |
| PB1 | TIM3_CH4 | Rain gauge pulse |
| PE0 | GPIO (input) | Mode DIP bit 0 |
| PE1 | GPIO (input) | Mode DIP bit 1 |
| PD0 | GPIO (output) | Status LED (green) |
| PD1 | GPIO (output) | Error LED (red) |
| PD2 | GPIO (output) | SCADA LED (amber) |

## RS485 Bus Wiring

```
Station A          Station B          SCADA Master
┌────────┐         ┌────────┐         ┌────────┐
│ MAX3485│         │ MAX3485│         │ RS485  │
│    A ──┼────┬────┼── A    │    ┌────┼── A    │
│    B ──┼────┼────┼── B    │    │    ┼── B    │
│   GND ─┼────┼────┼── GND  │    │    ┼── GND  │
└────────┘    │    └────────┘    │    └────────┘
         [120Ω]                 [120Ω]
```

- **Cable**: Twisted pair (Cat5e or dedicated RS485 cable)
- **Termination**: 120Ω at each end of bus
- **Max length**: 1200m at 9600 baud
- **Max devices**: 32 on single bus
- **Shield**: Connect at one end only (SCADA master)

## Hardware Components

| # | Component | Part Number | Package | Temp Range |
|---|-----------|-------------|---------|------------|
| 1 | MCU | STM32F407VGT6 | LQFP-100 | -40°C to +105°C |
| 2 | RS485 Transceiver | MAX3485ESA+ | SOIC-8 | -40°C to +85°C |
| 3 | HSE Crystal | ABM3B-8.000MHz | SMD HC49 | -40°C to +85°C |
| 4 | RS485 TVS | SMBJ6.0CA | SMB | -40°C to +125°C |
| 5 | RS485 Connector | Phoenix MC 1844210 | 4-pin plug | -40°C to +105°C |
| 6 | Mode DIP Switch | C&K SDA02H1SBD | 2-pos SMD | -20°C to +70°C |
| 7 | ATWINC1500 WiFi | ATWINC1500-MR210PB | QFN-28 | -40°C to +85°C |
| 8 | RN4870 BLE | RN4870-I/RM | 11.5×8mm | -40°C to +85°C |

## Power Budget Addition

| Component | Active (mA) | Sleep (µA) |
|-----------|------------|-----------|
| STM32F407VGT6 @ 168 MHz | 93 | 2.4 (Stop mode) |
| MAX3485ESA+ | 1.0 | 0.1 (shutdown) |
| Mode DIP switch | 0 | 0 |
| LEDs (3×) | 30 | 0 |
| **Subtotal** | **~124 mA** | **~2.5 µA** |

## KiCad Files

| File | Description |
|------|-------------|
| `stm32_scada.kicad_sch` | STM32F407 + MAX3485 schematic |
| `BOM.csv` | Updated with 18 new components |

## SCADA Integration Examples

### Reading Temperature with pymodbus (Python)

```python
from pymodbus.client import ModbusSerialClient

client = ModbusSerialClient(port='/dev/ttyUSB0', baudrate=9600, parity='N')
client.connect()

# Read temperature (register 30001, address 0)
result = client.read_input_registers(address=0, count=1, slave=1)
temp_c = result.registers[0] / 100.0  # Convert from i16 × 100
print(f"Temperature: {temp_c}°C")

# Read all weather data (registers 30001–30024)
result = client.read_input_registers(address=0, count=24, slave=1)
```

### Integration with PLC (IEC 61131-3)

```
(* Read weather station via Modbus RTU *)
MODBUS_READ(
    SlaveAddr := 1,
    FunctionCode := 4,   (* Read Input Registers *)
    StartReg := 0,
    NumRegs := 24,
    Data => weather_data
);

temperature := INT_TO_REAL(weather_data[0]) / 100.0;
humidity := UINT_TO_REAL(weather_data[1]) / 100.0;
wind_speed := UINT_TO_REAL(weather_data[4]) / 100.0;
```
