# Circuit Diagrams

## 1. Main System Circuit

```
                              ┌─────────────────────────────────────────────────────┐
                              │               STM32F407VGT6                      │
                              │                                                     │
      VCC 3.3V ───────────────┤ 3V3                                          GND ├──── GND
                              │                                                     │
                              │                  ┌──────── PB7 (I2C1 SDA) ───────┤────────┐
                              │                  │  ┌───── PB6 (I2C1 SCL) ───────┤───────┐│
                              │                  │  │                               │       ││
                              │                  │  │  ┌── PA7 (SPI1 MOSI) ──────┤──┐    ││
                              │                  │  │  │   PA6 (SPI1 MISO) ──────┤──┤┐   ││
                              │                  │  │  │   PA5 (SPI1 SCK)  ──────┤──┤┤┐  ││
                              │                  │  │  │   PA4 (SPI1 CS)   ──────┤──┤┤┤┐ ││
                              │                  │  │  │   PC4 (LoRa RST) ──────┤──┤┤┤┤ ││
                              │                  │  │  │   PC5 (LoRa IRQ) ──────┤──┤┤┤┤ ││
                              │                  │  │  │                             │  ││││ ││
                              │                  │  │  │   PB0 (Wind TIM3) ──────┤──┼┼┼┼─┼┼───┐
                              │                  │  │  │   PB1 (Rain TIM3) ──────┤──┼┼┼┼─┼┼───┤─┐
                              │                  │  │  │   PA0 (ADC Batt) ──────┤──┼┼┼┼─┼┼───┤─┤─┐
                              │                  │  │  │                             │  ││││ ││   │ │ │
                              │                  │  │  │   PA9 (UART TX) ───────┤──┼┼┼┼─┼┼───┤─┤─┤─┐
                              │                  │  │  │   PA10 (UART RX) ───────┤──┼┼┼┼─┼┼───┤─┤─┤─┤─┐
                              │                  │  │  │                             │  ││││ ││   │ │ │ │ │
                              │                  │  │  │   PD0 (LED)     ───────┤──┼┼┼┼─┼┼───┤─┤─┤─┤─┤─┐
                              │                  │  │  │                             │  ││││ ││   │ │ │ │ │ │
                              └─────────────────────────────────────────────────────┘  ││││ ││   │ │ │ │ │ │
                                                 │  │  │                               ││││ ││   │ │ │ │ │ │
                                                 │  │  │  ┌────────────────────────────┘│││ ││   │ │ │ │ │ │
                                                 │  │  │  │  ┌─────────────────────────┘││ ││   │ │ │ │ │ │
                                                 │  │  │  │  │  ┌──────────────────────┘│ ││   │ │ │ │ │ │
                                                 │  │  │  │  │  │  ┌───────────────────┘ ││   │ │ │ │ │ │
                                                 ▼  ▼  ▼  ▼  ▼  ▼  ▼                    ▼▼   ▼ ▼ ▼ ▼ ▼ ▼
                                            (see subsystem diagrams below)
```

## 2. I2C Bus Circuit

```
       3.3V
        │
       ┌┴┐ 4.7kΩ    ┌─┐ 4.7kΩ
       │ │           │ │
       └┬┘           └┬┘
        │              │
  SDA ──┼──────┬───────┼──────┬──────────┬──────────┬───── PB7
        │      │       │      │          │          │
  SCL ──┼──┬───┼───────┼──┬───┼──────┬───┼──────┬───┼───── PB6
        │  │   │       │  │   │      │   │      │   │
   ┌────┴──┴┐  │  ┌────┴──┴┐  │ ┌────┴───┴┐ ┌───┴───┴┐
   │ BME280 │  │  │ AS5600 │  │ │ SI1145  │ │ BH1750 │
   │ 0x76   │  │  │ 0x36   │  │ │ 0x60    │ │ 0x23   │
   │        │  │  │        │  │ │         │ │        │
   │ VCC GND│  │  │ VCC GND│  │ │ VCC GND │ │ VCC GND│
   └──┬──┬──┘  │  └──┬──┬──┘  │ └──┬──┬───┘ └──┬──┬──┘
      │  │     │     │  │     │    │  │         │  │
    3.3V GND   │   3.3V GND   │  3.3V GND    3.3V GND
               │              │
         100nF ═         100nF ═        (decoupling caps on each VCC)
               │              │
              GND            GND
```

## 3. SPI Bus — LoRa SX1276 (RFM95W)

```
                          ┌─────────────────────┐
                          │    RFM95W / SX1276  │
    PA7 (MOSI) ───────┤ MOSI           ANT ├───── 50Ω Antenna
    PA6 (MISO) ◄──────┤ MISO               │
    PA5 (SCK)  ───────┤ SCK            VCC ├───── 3.3V
    PA4 (CS)   ───────┤ NSS            GND ├───── GND
    PC4 (RST)  ───────┤ RESET              │
    PC5 (IRQ)  ◄──────┤ DIO0          DIO1 ├───── (NC)
                          │               DIO2 ├───── (NC)
                          └─────────────────────┘
                                  │
                             100nF ═  (VCC decoupling)
                                  │
                                 GND

    Note: SPI clock = 10 MHz
          NSS active low
          DIO0 = TX Done / RX Done interrupt
```

## 3b. SPI2 Bus — SD Card (Shared with W5500 Ethernet)

```
                          ┌─────────────────────┐
                          │  Micro-SD Card (J22) │
    PB13 (SCK)  ─────────┤ CLK  (pin 5)        │
    PB15 (MOSI) ─────────┤ DI   (pin 2)        │
    PB14 (MISO) ◄────────┤ DO   (pin 7)        │
    PD10 (CS)   ─────────┤ CS   (pin 1)        │
                          │               VCC ├───── 3.3V
                          │               VSS ├───── GND
                          │                CD ├────┐
                          └─────────────────────┘    │
                                 │                     │
                            100nF ═  (C38)          ┌┴┐ 10kΩ (R27)
                                 │                  │ │  pull-up
                                GND                 └┬┘
                                                     │
                                              PD11 ──┘ (Card Detect)

    SPI2 shared bus: W5500 (CS=PB12) + SD Card (CS=PD10)
    Init clock: 400 kHz, Normal: 25 MHz
    Card detect: PD11 active low (LOW = card inserted)
    CS managed via CriticalSectionDevice (embedded-hal-bus)
```

## 4. Pulse Input — Anemometer & Rain Gauge

```
    3.3V                              3.3V
     │                                 │
    ┌┴┐ 10kΩ (pull-up)              ┌┴┐ 10kΩ (pull-up)
    │ │                              │ │
    └┬┘                              └┬┘
     │                                │
     ├──── PB0 (Wind TIM3)         ├──── PB1 (Rain TIM3)
     │                                │
     │    ┌──────────────┐            │    ┌──────────────┐
     └────┤ Reed Switch  │            └────┤ Reed Switch  │
          │ (Anemometer) │                 │ (Rain Gauge) │
     ┌────┤              │            ┌────┤              │
     │    └──────────────┘            │    └──────────────┘
     │                                │
    GND                              GND

    100nF debounce caps optional (software debounce preferred)

    Anemometer: 1 pulse per revolution
    Wind speed (km/h) = pulse_count × 2.4 / sample_period_s

    Rain gauge: 1 pulse per 0.2mm tip
    Rainfall (mm) = tip_count × 0.2
```

## 5. Battery Monitoring — ADC

```
    LiPo Battery (3.0V – 4.2V)
     │
    ┌┴┐ 100kΩ
    │ │
    └┬┘
     │
     ├──── PA0 (ADC1_CH0)
     │
    ┌┴┐ 100kΩ
    │ │
    └┬┘
     │
    GND

    Voltage divider ratio: 1:2
    ADC reads Vbat / 2
    Vbat = ADC_reading × 2 × (3.3V / 4095)

    Full:  4.2V → ADC reads ~2.1V → ~2612 counts
    Empty: 3.0V → ADC reads ~1.5V → ~1862 counts
```

## 6. Power Supply Circuit

```
    Solar Panel 6V 2W          USB-C 5V
         │                       │
         │    ┌──────────────┐   │
         └────┤ IN+    VBUS ├───┘
              │              │
              │   TP4056     │
              │   Charge     │
              │   Controller │
              │              │
         ┌────┤ BAT+    BAT-├────┐
         │    └──────────────┘    │
         │                        │
    ┌────┴────┐                   │
    │  LiPo   │                   │
    │ 3.7V    │                   │
    │ 3000mAh │                   │
    └────┬────┘                   │
         │                        │
         │    ┌──────────────┐    │
         └────┤ VIN    VOUT ├────┼──── 3.3V Rail
              │              │    │
              │  AMS1117-3.3 │    │
              │              │    │
         ┌────┤ GND          │    │
         │    └──────────────┘    │
         │                        │
         └────────────────────────┘
                   │
                  GND

    Decoupling:
    - 10µF tantalum on AMS1117 input and output
    - 100nF ceramic on 3.3V rail near STM32
```

## 7. Status LED

```
    PD0
     │
    ┌┴┐ 330Ω
    │ │
    └┬┘
     │
     ▼ LED (Green, 2mA)
     │
    GND

    LED States:
    - Solid:       System OK, WiFi connected
    - Slow blink:  WiFi disconnected, operating on LoRa
    - Fast blink:  Error condition
    - Off:         Deep sleep
```

## 8. UART Debug Header

```
    PA9 (TX) ──────┤ TX  ┌──────────┐
                       │     │ USB-UART │ ──── USB to PC
    PA10 (RX) ──────┤ RX  │ CP2102   │
                       │     └──────────┘
    GND ───────────────┤ GND

    Settings: 115200 baud, 8N1, no flow control
```
