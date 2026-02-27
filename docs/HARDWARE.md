# Hardware Design

## 1. Bill of Materials (BOM)

| # | Component | Part Number | Qty | Interface | Notes |
|---|-----------|-------------|-----|-----------|-------|
| 1 | ESP32-S3-WROOM-1 | ESP32-S3-WROOM-1-N16R8 | 1 | — | 16MB flash, 8MB PSRAM |
| 2 | BME280 breakout | GY-BME280 | 1 | I2C | Temp/humidity/pressure |
| 3 | AS5600 magnetic encoder | AS5600-ASOM | 1 | I2C | Wind direction |
| 4 | Anemometer | SEN-15901 | 1 | GPIO | Reed switch, 1 pulse/rev |
| 5 | Rain gauge (tipping bucket) | SEN-15902 | 1 | GPIO | 0.2 mm/tip |
| 6 | SI1145 UV sensor | SI1145-A10-GMR | 1 | I2C | UV/Vis/IR |
| 7 | BH1750 light sensor | BH1750FVI | 1 | I2C | Ambient light |
| 8 | SX1276 LoRa module | RFM95W | 1 | SPI | 868/915 MHz |
| 9 | LiPo battery | — | 1 | — | 3.7V 3000mAh |
| 10 | Solar panel | — | 1 | — | 6V 2W |
| 11 | TP4056 charge controller | TP4056 | 1 | — | USB-C + solar input |
| 12 | AMS1117-3.3 regulator | AMS1117-3.3 | 1 | — | 3.3V LDO |
| 13 | Diode magnet (wind vane) | N52 6mm | 1 | — | For AS5600 |
| 14 | Pull-up resistors | — | 4 | — | 4.7kΩ for I2C |
| 15 | Decoupling caps | — | 6 | — | 100nF ceramic |
| 16 | Voltage divider resistors | — | 2 | — | 100kΩ + 100kΩ for batt |

## 2. Pin Mapping

```
ESP32-S3 Pin Allocation
═══════════════════════════════════════════════════════════

I2C Bus (shared):
  GPIO 8  ──── SDA ──── BME280, AS5600, SI1145, BH1750
  GPIO 9  ──── SCL ──── BME280, AS5600, SI1145, BH1750

SPI Bus (LoRa SX1276):
  GPIO 10 ──── MOSI ──── SX1276 MOSI
  GPIO 11 ──── MISO ──── SX1276 MISO
  GPIO 12 ──── SCK  ──── SX1276 SCK
  GPIO 13 ──── CS   ──── SX1276 NSS
  GPIO 14 ──── RST  ──── SX1276 RESET
  GPIO 15 ──── DIO0 ──── SX1276 DIO0 (interrupt)

GPIO Interrupts:
  GPIO 4  ──── WIND ──── Anemometer reed switch (pull-up)
  GPIO 5  ──── RAIN ──── Rain gauge reed switch (pull-up)

ADC:
  GPIO 6  ──── VBAT ──── Battery voltage (via divider)

UART (debug console):
  GPIO 43 ──── TX   ──── USB-UART / debug header
  GPIO 44 ──── RX   ──── USB-UART / debug header

Status LED:
  GPIO 2  ──── LED  ──── Onboard LED (active low)

Boot / Reset:
  GPIO 0  ──── BOOT ──── Boot button
  EN      ──── RST  ──── Reset button
```

## 3. I2C Address Map

| Device | Address | Bus |
|--------|---------|-----|
| BME280 | 0x76 | I2C0 |
| AS5600 | 0x36 | I2C0 |
| SI1145 | 0x60 | I2C0 |
| BH1750 | 0x23 | I2C0 |

All four devices on a single I2C bus at 400 kHz (fast mode).
4.7 kΩ pull-ups on SDA and SCL.

## 4. Circuit Schematic (Block Level)

```
                        ┌─────────────────────────────────────┐
                        │          ESP32-S3-WROOM-1           │
                        │                                     │
   ┌─────────┐   I2C   │  GPIO8 (SDA) ◄──────────────────┐  │
   │ BME280  │◄────────►│  GPIO9 (SCL) ◄──────────────┐   │  │
   │ 0x76    │          │                              │   │  │
   └─────────┘          │  GPIO10 (MOSI) ────────┐    │   │  │
                        │  GPIO11 (MISO) ◄───┐   │    │   │  │
   ┌─────────┐   I2C   │  GPIO12 (SCK)  ─┐  │   │    │   │  │
   │ AS5600  │◄────────►│  GPIO13 (CS)  ─┐│  │   │    │   │  │
   │ 0x36    │          │  GPIO14 (RST) ─┐││  │   │    │   │  │
   └─────────┘          │  GPIO15 (IRQ) ◄┐│││  │   │    │   │  │
                        │                ││││  │   │    │   │  │
   ┌─────────┐   I2C   │                ││││  │   │    │   │  │
   │ SI1145  │◄────────►│                ▼▼▼▼  ▼   ▼    ▼   ▼  │
   │ 0x60    │          │          ┌──────────────────────┐ │  │
   └─────────┘          │          │     SX1276 (LoRa)   │ │  │
                        │          │     RFM95W Module    │ │  │
   ┌─────────┐   I2C   │          └──────────────────────┘ │  │
   │ BH1750  │◄────────►│                                   │  │
   │ 0x23    │          │  GPIO4 ◄──── Anemometer (pulse)  │  │
   └─────────┘          │  GPIO5 ◄──── Rain gauge (pulse)  │  │
                        │  GPIO6 ◄──── Battery ADC         │  │
                        │                                   │  │
                        │  GPIO43 ────► UART TX (debug)    │  │
                        │  GPIO44 ◄──── UART RX (debug)    │  │
                        │                                   │  │
                        │  GPIO2  ────► Status LED          │  │
                        │                                   │  │
                        │  WiFi Antenna (onboard)           │  │
                        │  BLE Antenna (shared)             │  │
                        └─────────────────────────────────────┘

Power Supply:
  Solar Panel (6V) ──► TP4056 ──► LiPo 3.7V ──► AMS1117 ──► 3.3V rail
                        │
                     USB-C input
```

## 5. Power Budget

| Component | Active (mA) | Sleep (µA) |
|-----------|------------|-----------|
| ESP32-S3 (WiFi TX) | 120 | 10 |
| ESP32-S3 (BLE) | 30 | 10 |
| BME280 | 0.35 | 0.1 |
| AS5600 | 6.5 | 1.5 |
| SI1145 | 5.0 | 0.5 |
| BH1750 | 0.12 | 0.01 |
| SX1276 (TX) | 120 | 0.2 |
| SX1276 (RX) | 12 | 0.2 |
| **Total (active)** | **~170 mA** | — |
| **Total (deep sleep)** | — | **~12 µA** |

## 6. PCB Design Notes

- 4-layer PCB recommended (signal, ground, power, signal)
- Keep SX1276 antenna trace as 50Ω impedance-matched microstrip
- I2C pull-ups close to ESP32-S3
- Decoupling caps within 5mm of each IC VCC pin
- Ground pour on all layers
- Keep BME280 thermally isolated from heat-generating components
- Conformal coating for outdoor weather resistance
- IP65-rated enclosure minimum
