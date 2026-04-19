# System Architecture Diagram

## Multi-Layer Architecture

```
╔═══════════════════════════════════════════════════════════════════════════╗
║                                                                           ║
║  LAYER 5: CLOUD / EXTERNAL SYSTEMS                                       ║
║  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────────┐   ║
║  │  MQTT Broker    │  │  LoRaWAN        │  │  Mobile App (BLE)       │   ║
║  │  (Mosquitto /   │  │  Network Server │  │  iOS / Android          │   ║
║  │   AWS IoT)      │  │  (TTN/Chirp)    │  │                         │   ║
║  └────────┬────────┘  └────────┬────────┘  └────────────┬────────────┘   ║
║           │                    │                         │                ║
╠═══════════╪════════════════════╪═════════════════════════╪════════════════╣
║           │                    │                         │                ║
║  LAYER 4: APPLICATION          │                         │                ║
║  ┌────────┴────────────────────┴─────────────────────────┴────────────┐   ║
║  │                        SCHEDULER                                   │   ║
║  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────────┐ │   ║
║  │  │ Sensor Task  │  │ Comms Task   │  │ Maintenance Task         │ │   ║
║  │  │ (high pri)   │  │ (med pri)    │  │ (low priority)          │ │   ║
║  │  │ Period: 5-30s│  │ Event-driven │  │ Period: 6h              │ │   ║
║  │  └──────┬───────┘  └──────┬───────┘  └──────────┬───────────────┘ │   ║
║  │         │                 │                      │                 │   ║
║  │  ┌──────▼───────┐  ┌─────▼────────┐  ┌──────────▼──────────────┐ │   ║
║  │  │Data Pipeline │  │Channel Router│  │ OTA Update Manager      │ │   ║
║  │  │ Calibrate    │  │ Priority:    │  │ Check → Download →      │ │   ║
║  │  │ Filter       │  │ WiFi>LoRa>BLE│  │ Verify → Flash →       │ │   ║
║  │  │ Validate     │  │              │  │ Reboot → Validate       │ │   ║
║  │  │ Package      │  │              │  │                         │ │   ║
║  │  └──────┬───────┘  └──────┬───────┘  └─────────────────────────┘ │   ║
║  │         │                 │                                       │   ║
║  │  ┌──────▼───────┐  ┌─────▼────────┐                              │   ║
║  │  │Alert Engine  │  │Power Manager │                               │   ║
║  │  │ Threshold    │  │ Active       │                               │   ║
║  │  │ monitoring   │  │ Sleep (WFI)  │                               │   ║
║  │  │              │  │ Stop         │                               │   ║
║  │  │              │  │ Standby      │                               │   ║
║  │  └──────────────┘  └──────────────┘                               │   ║
║  └───────────────────────────────────────────────────────────────────┘   ║
║                                                                           ║
╠═══════════════════════════════════════════════════════════════════════════╣
║                                                                           ║
║  LAYER 3: COMMUNICATION                                                   ║
║  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────┐   ║
║  │  WiFi    │ │  BLE     │ │  LoRa    │ │  UART    │ │  Protocols   │   ║
║  │  Manager │ │  GATT    │ │  Radio   │ │  Console │ │              │   ║
║  │          │ │  Server  │ │  SX1276  │ │          │ │  ┌────────┐  │   ║
║  │ ┌──────┐ │ │          │ │          │ │ ┌──────┐ │ │  │  MQTT  │  │   ║
║  │ │Reconnect││ │ ┌──────┐ │ │ ┌──────┐ │ │ │Parser│ │ │  │Client │  │   ║
║  │ │Logic │ │ │ │Advert │ │ │ │Packet│ │ │ │Cmd  │ │ │  │        │  │   ║
║  │ └──────┘ │ │ │Engine │ │ │ │Codec │ │ │ │Exec │ │ │  ├────────┤  │   ║
║  │          │ │ └──────┘ │ │ └──────┘ │ │ └──────┘ │ │  │  HTTP  │  │   ║
║  │ ┌──────┐ │ │          │ │          │ │          │ │  │Server  │  │   ║
║  │ │NTP   │ │ │ ┌──────┐ │ │ ┌──────┐ │ │          │ │  │REST API│  │   ║
║  │ │Sync  │ │ │ │Pairing│ │ │ │LoRaWAN│ │ │          │ │  │        │  │   ║
║  │ └──────┘ │ │ └──────┘ │ │ │Stack │ │ │          │ │  └────────┘  │   ║
║  └──────────┘ └──────────┘ │ └──────┘ │ └──────────┘ └──────────────┘   ║
║                             └──────────┘                                  ║
║                                                                           ║
╠═══════════════════════════════════════════════════════════════════════════╣
║                                                                           ║
║  LAYER 2: DRIVERS                                                         ║
║  ┌────────────┐ ┌────────────┐ ┌────────────┐ ┌──────────┐ ┌──────────┐ ║
║  │  BME280    │ │  AS5600    │ │ Anemometer │ │ Rain     │ │ SI1145   │ ║
║  │  Driver    │ │  Driver    │ │ Driver     │ │ Gauge    │ │ Driver   │ ║
║  │            │ │            │ │            │ │ Driver   │ │          │ ║
║  │ read_temp()│ │ read_angle │ │ get_speed()│ │          │ │ read_uv()│ ║
║  │ read_hum() │ │ ()         │ │            │ │get_rain()│ │          │ ║
║  │ read_pres()│ │            │ │  ┌──────┐  │ │          │ │          │ ║
║  │            │ │  ┌──────┐  │ │  │Pulse │  │ │ ┌──────┐ │ │          │ ║
║  │  ┌──────┐  │ │  │Magnet│  │ │  │Count │  │ │ │Tip   │ │ │          │ ║
║  │  │Calib │  │ │  │Offset│  │ │  │ISR   │  │ │ │Count │ │ │          │ ║
║  │  └──────┘  │ │  └──────┘  │ │  └──────┘  │ │ │ISR   │ │ │          │ ║
║  └────────────┘ └────────────┘ └────────────┘ │ └──────┘ │ └──────────┘ ║
║                                                └──────────┘              ║
║  ┌────────────┐                                                          ║
║  │  BH1750    │                                                          ║
║  │  Driver    │                                                          ║
║  │ read_lux() │                                                          ║
║  └────────────┘                                                          ║
║                                                                           ║
╠═══════════════════════════════════════════════════════════════════════════╣
║                                                                           ║
║  LAYER 1: HAL (stm32f4xx-hal)                                             ║
║  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐ ┌────────────┐ ║
║  │  I2C   │ │  SPI   │ │  ADC   │ │  GPIO  │ │  UART  │ │  Timers    │ ║
║  │ Master │ │ Master │ │ 12-bit │ │ In/Out │ │ Serial │ │  HW Timer  │ ║
║  │ 400kHz │ │ 10MHz  │ │ SAR    │ │ IRQ    │ │ 115200 │ │  Watchdog  │ ║
║  └────────┘ └────────┘ └────────┘ └────────┘ └────────┘ │  RTC       │ ║
║                                                          └────────────┘ ║
║                                                                           ║
╠═══════════════════════════════════════════════════════════════════════════╣
║                                                                           ║
║  LAYER 0: HARDWARE                                                        ║
║  ┌───────────────────────────────────────────────────────────────────────┐ ║
║  │  STM32F407VGT6 — ARM Cortex-M4F @ 168 MHz (Industrial -40/+105°C)    │ ║
║  │  192 KB SRAM (128 KB + 64 KB CCM) · 1 MB Flash                      │ ║
║  │  ATWINC1500 WiFi (SPI3) · RN4870 BLE (USART3)                       │ ║
║  │  W5500 Ethernet + SD Card (SPI2, optional) · SIM7600 (UART4, opt)  │ ║
║  │  82 GPIO · 16 ADC channels · 3 I2C · 3 SPI · 4 UART                │ ║
║  └───────────────────────────────────────────────────────────────────────┘ ║
║                                                                           ║
╚═══════════════════════════════════════════════════════════════════════════╝
```

## Memory Layout

```
Flash (1 MB) — STM32F407VGT6
┌───────────────────────┐ 0x08000000
│ Sector 0-3 (64 KB)    │  ← Firmware (active)
├───────────────────────┤ 0x08010000
│ Sector 4 (64 KB)      │  ← Data storage (circular buffer)
├───────────────────────┤ 0x08020000
│ Sectors 5-7 (384 KB)  │  ← Firmware continued
├───────────────────────┤ 0x08080000
│ Sectors 8-11 (512 KB) │  ← OTA staging (bank 2)
└───────────────────────┘ 0x08100000

SRAM (192 KB)
┌───────────────────────┐ 0x20000000
│ Stack (8 KB)          │
├───────────────────────┤
│ Heap (48 KB)          │
│  ├─ Ring Buffers      │
│  ├─ MQTT buffers      │
│  ├─ HTTP buffers      │
│  └─ Driver state      │
├───────────────────────┤
│ Static Data (~16 KB)  │
│  ├─ Config            │
│  ├─ Current readings  │
│  └─ Sensor state      │
├───────────────────────┤ 0x20020000
│ CCM RAM (64 KB)       │  ← DMA-inaccessible, stack/scratch
└───────────────────────┘ 0x20030000
```

## Interrupt Priority Map

```
Priority 1 (Highest): Watchdog Timer
Priority 2:           GPIO ISR (wind pulse, rain tip)
Priority 3:           SPI DMA complete (LoRa)
Priority 4:           I2C transaction complete
Priority 5:           UART RX
Priority 6 (Lowest):  Software timer callbacks
```
