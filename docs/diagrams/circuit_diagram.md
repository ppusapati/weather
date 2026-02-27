# Circuit Diagrams

## 1. Main System Circuit

```
                              ┌─────────────────────────────────────────────────────┐
                              │               ESP32-S3-WROOM-1                      │
                              │                                                     │
      VCC 3.3V ───────────────┤ 3V3                                          GND ├──── GND
                              │                                                     │
                              │                  ┌──────── GPIO 8  (I2C SDA) ───────┤────────┐
                              │                  │  ┌───── GPIO 9  (I2C SCL) ───────┤───────┐│
                              │                  │  │                               │       ││
                              │                  │  │  ┌── GPIO 10 (SPI MOSI) ──────┤──┐    ││
                              │                  │  │  │   GPIO 11 (SPI MISO) ──────┤──┤┐   ││
                              │                  │  │  │   GPIO 12 (SPI SCK)  ──────┤──┤┤┐  ││
                              │                  │  │  │   GPIO 13 (SPI CS)   ──────┤──┤┤┤┐ ││
                              │                  │  │  │   GPIO 14 (LoRa RST) ──────┤──┤┤┤┤ ││
                              │                  │  │  │   GPIO 15 (LoRa IRQ) ──────┤──┤┤┤┤ ││
                              │                  │  │  │                             │  ││││ ││
                              │                  │  │  │   GPIO 4  (Wind IRQ) ──────┤──┼┼┼┼─┼┼───┐
                              │                  │  │  │   GPIO 5  (Rain IRQ) ──────┤──┼┼┼┼─┼┼───┤─┐
                              │                  │  │  │   GPIO 6  (ADC Batt) ──────┤──┼┼┼┼─┼┼───┤─┤─┐
                              │                  │  │  │                             │  ││││ ││   │ │ │
                              │                  │  │  │   GPIO 43 (UART TX) ───────┤──┼┼┼┼─┼┼───┤─┤─┤─┐
                              │                  │  │  │   GPIO 44 (UART RX) ───────┤──┼┼┼┼─┼┼───┤─┤─┤─┤─┐
                              │                  │  │  │                             │  ││││ ││   │ │ │ │ │
                              │                  │  │  │   GPIO 2  (LED)     ───────┤──┼┼┼┼─┼┼───┤─┤─┤─┤─┤─┐
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
  SDA ──┼──────┬───────┼──────┬──────────┬──────────┬───── GPIO 8
        │      │       │      │          │          │
  SCL ──┼──┬───┼───────┼──┬───┼──────┬───┼──────┬───┼───── GPIO 9
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
    GPIO 10 (MOSI) ───────┤ MOSI           ANT ├───── 50Ω Antenna
    GPIO 11 (MISO) ◄──────┤ MISO               │
    GPIO 12 (SCK)  ───────┤ SCK            VCC ├───── 3.3V
    GPIO 13 (CS)   ───────┤ NSS            GND ├───── GND
    GPIO 14 (RST)  ───────┤ RESET              │
    GPIO 15 (IRQ)  ◄──────┤ DIO0          DIO1 ├───── (NC)
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

## 4. Pulse Input — Anemometer & Rain Gauge

```
    3.3V                              3.3V
     │                                 │
    ┌┴┐ 10kΩ (pull-up)              ┌┴┐ 10kΩ (pull-up)
    │ │                              │ │
    └┬┘                              └┬┘
     │                                │
     ├──── GPIO 4 (Wind IRQ)         ├──── GPIO 5 (Rain IRQ)
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
     ├──── GPIO 6 (ADC1_CH5)
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
    - 100nF ceramic on 3.3V rail near ESP32
```

## 7. Status LED

```
    GPIO 2
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
    GPIO 43 (TX) ──────┤ TX  ┌──────────┐
                       │     │ USB-UART │ ──── USB to PC
    GPIO 44 (RX) ──────┤ RX  │ CP2102   │
                       │     └──────────┘
    GND ───────────────┤ GND

    Settings: 115200 baud, 8N1, no flow control
```
