# Hardware Design

## 1. Bill of Materials (BOM)

| # | Component | Part Number | Qty | Interface | Notes |
|---|-----------|-------------|-----|-----------|-------|
| 1 | STM32F407VGT6 | STM32F407VGT6 | 1 | — | ARM Cortex-M4F, 1MB Flash, 192KB SRAM |
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
| 17 | Micro-SD card holder | Molex 5031821852 | 1 | SPI2 | Push-push, CS=PD10, DET=PD11 |
| 18 | SD card decoupling cap | — | 1 | — | 100nF on VCC |
| 19 | SD card detect pull-up | — | 1 | — | 10kΩ pull-up on PD11 |

## 2. Pin Mapping

```
STM32F407 Pin Allocation
═══════════════════════════════════════════════════════════

I2C1 Bus (shared):
  PB7  ──── SDA ──── BME280, AS5600, SI1145, BH1750
  PB6  ──── SCL ──── BME280, AS5600, SI1145, BH1750

SPI1 (LoRa SX1276):
  PA7  ──── MOSI ──── SX1276 MOSI
  PA6  ──── MISO ──── SX1276 MISO
  PA5  ──── SCK  ──── SX1276 SCK
  PA4  ──── CS   ──── SX1276 NSS
  PC4  ──── RST  ──── SX1276 RESET
  PC5  ──── DIO0 ──── SX1276 DIO0 (interrupt)

SPI3 (ATWINC1500 WiFi):
  PB5  ──── MOSI ──── ATWINC1500 MOSI
  PB4  ──── MISO ──── ATWINC1500 MISO
  PB3  ──── SCK  ──── ATWINC1500 SCK
  PE3  ──── CS   ──── ATWINC1500 SSn
  PE4  ──── RST  ──── ATWINC1500 RESET
  PE5  ──── IRQ  ──── ATWINC1500 IRQn (EXTI)
  PE6  ──── EN   ──── ATWINC1500 CHIP_EN

USART3 (RN4870 BLE):
  PB10 ──── TX   ──── RN4870 RX
  PB11 ──── RX   ──── RN4870 TX
  PD8  ──── RST  ──── RN4870 RST
  PD9  ──── STATUS ── RN4870 STATUS

GPIO Interrupts (TIM3):
  PB0  ──── WIND ──── Anemometer pulse (TIM3_CH3)
  PB1  ──── RAIN ──── Rain gauge pulse (TIM3_CH4)

ADC:
  PA0  ──── VBAT ──── Battery voltage (via divider)

USART1 (debug console):
  PA9  ──── TX   ──── Debug header
  PA10 ──── RX   ──── Debug header

USART2 (RS485 Modbus):
  PA2  ──── TX   ──── MAX3485 DI
  PA3  ──── RX   ──── MAX3485 RO
  PA1  ──── DE/RE ─── MAX3485 DE/RE

Status LEDs:
  PD0  ──── LED  ──── Status (green)
  PD1  ──── LED  ──── Error (red)
  PD2  ──── LED  ──── SCADA (amber)

SD Card (SPI2 shared bus, optional):
  PD10 ──── CS   ──── SD card CS (active low)
  PD11 ──── DET  ──── SD card detect (active low = inserted)

Mode DIP Switch:
  PE0  ──── DIP0 ──── Mode bit 0
  PE1  ──── DIP1 ──── Mode bit 1
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
                        │          STM32F407VGT6              │
                        │                                     │
   ┌─────────┐   I2C   │  PB7 (SDA) ◄──────────────────────┐  │
   │ BME280  │◄────────►│  PB6 (SCL) ◄────────────────┐   │  │
   │ 0x76    │          │                              │   │  │
   └─────────┘          │  PA7 (MOSI) ────────┐       │   │  │
                        │  PA6 (MISO) ◄───┐   │       │   │  │
   ┌─────────┐   I2C   │  PA5 (SCK)  ─┐  │   │       │   │  │
   │ AS5600  │◄────────►│  PA4 (CS)  ──┐│  │   │       │   │  │
   │ 0x36    │          │  PC4 (RST) ──┐││  │   │       │   │  │
   └─────────┘          │  PC5 (IRQ) ◄─┐│││  │   │       │   │  │
                        │               ││││  │   │       │   │  │
   ┌─────────┐   I2C   │               ▼▼▼▼  ▼   ▼       ▼   ▼  │
   │ SI1145  │◄────────►│         ┌──────────────────────┐    │  │
   │ 0x60    │          │         │     SX1276 (LoRa)   │    │  │
   └─────────┘          │         │     RFM95W Module    │    │  │
                        │         └──────────────────────┘    │  │
   ┌─────────┐   I2C   │                                      │  │
   │ BH1750  │◄────────►│  PB0 ◄──── Anemometer (TIM3_CH3)   │  │
   │ 0x23    │          │  PB1 ◄──── Rain gauge (TIM3_CH4)    │  │
   └─────────┘          │  PA0 ◄──── Battery ADC              │  │
                        │                                      │  │
                        │  PA9  ────► UART TX (debug)          │  │
                        │  PA10 ◄──── UART RX (debug)          │  │
                        │                                      │  │
                        │  PB3-PB5 ──► ATWINC1500 WiFi (SPI3) │  │
                        │  PB10-PB11 ► RN4870 BLE (USART3)    │  │
                        │  PB12-PB15 ► SPI2 (W5500 + SD Card) │  │
                        │  PD10 ─────► SD Card CS              │  │
                        │  PD11 ◄────  SD Card Detect          │  │
                        │                                      │  │
                        └──────────────────────────────────────────┘

Power Supply:
  Solar Panel (6V) ──► TP4056 ──► LiPo 3.7V ──► AMS1117 ──► 3.3V rail
                        │
                     USB-C input
```

## 5. Power Budget

| Component | Active (mA) | Sleep (µA) |
|-----------|------------|-----------|
| STM32F407 @ 168 MHz | 93 | 2.4 (Stop) |
| ATWINC1500 (WiFi TX) | 120 | 10 |
| RN4870 (BLE) | 15 | 5 |
| BME280 | 0.35 | 0.1 |
| AS5600 | 6.5 | 1.5 |
| SI1145 | 5.0 | 0.5 |
| BH1750 | 0.12 | 0.01 |
| SX1276 (TX) | 120 | 0.2 |
| SX1276 (RX) | 12 | 0.2 |
| Micro-SD Card (write) | 100 | 0.2 |
| **Total (active)** | **~350 mA** | — |
| **Total (standby)** | — | **~18 µA** |

## 6. PCB Design Notes

- 4-layer PCB recommended (signal, ground, power, signal)
- Keep SX1276 antenna trace as 50Ω impedance-matched microstrip
- I2C pull-ups close to STM32F407
- Decoupling caps within 5mm of each IC VCC pin
- Ground pour on all layers
- Keep BME280 thermally isolated from heat-generating components
- Conformal coating for outdoor weather resistance
- IP65-rated enclosure minimum
