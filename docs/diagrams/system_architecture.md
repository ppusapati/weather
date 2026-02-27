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
║  │  │ (Core 0)     │  │ (Core 1)     │  │ (Core 1, low priority)  │ │   ║
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
║  │  │ monitoring   │  │ Modem Sleep  │                               │   ║
║  │  │              │  │ Light Sleep  │                               │   ║
║  │  │              │  │ Deep Sleep   │                               │   ║
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
║  LAYER 1: HAL (esp-hal)                                                   ║
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
║  │  ESP32-S3-WROOM-1-N16R8                                              │ ║
║  │  Xtensa LX7 Dual-Core @ 240 MHz                                     │ ║
║  │  512 KB SRAM · 8 MB PSRAM · 16 MB Flash                             │ ║
║  │  WiFi 802.11 b/g/n · BLE 5.0                                        │ ║
║  │  45 GPIO · 20 ADC channels · 2 I2C · 4 SPI · 3 UART                │ ║
║  └───────────────────────────────────────────────────────────────────────┘ ║
║                                                                           ║
╚═══════════════════════════════════════════════════════════════════════════╝
```

## Memory Layout

```
Flash (16 MB)
┌───────────────────────┐ 0x00000000
│ Bootloader (64 KB)    │
├───────────────────────┤ 0x00010000
│ Partition Table (4 KB)│
├───────────────────────┤ 0x00011000
│ NVS (24 KB)           │  ← config, calibration, WiFi creds
├───────────────────────┤ 0x00017000
│ OTA Data (8 KB)       │  ← boot partition selector
├───────────────────────┤ 0x00019000
│ App Partition 0 (4 MB)│  ← active firmware
├───────────────────────┤ 0x00419000
│ App Partition 1 (4 MB)│  ← OTA staging
├───────────────────────┤ 0x00819000
│ Data Storage (7.9 MB) │  ← circular buffer for readings
└───────────────────────┘ 0x01000000

SRAM (512 KB)
┌───────────────────────┐ 0x3FC88000
│ Stack (Core 0) 8 KB   │
├───────────────────────┤
│ Stack (Core 1) 8 KB   │
├───────────────────────┤
│ Heap (~400 KB)        │
│  ├─ Ring Buffers      │
│  ├─ MQTT buffers      │
│  ├─ HTTP buffers      │
│  └─ Driver state      │
├───────────────────────┤
│ Static Data (~16 KB)  │
│  ├─ Config            │
│  ├─ Current readings  │
│  └─ Sensor state      │
└───────────────────────┘
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
