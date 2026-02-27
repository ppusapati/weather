# Communication Flow Diagrams

## 1. Multi-Channel Communication Architecture

```
                                    ┌─────────────────────────────┐
                                    │     WEATHER STATION          │
                                    │        ESP32-S3              │
                                    │                             │
                                    │  ┌──────────────────────┐  │
                                    │  │   Channel Router     │  │
                                    │  │                      │  │
                                    │  │  WeatherReading ─────┤──┤──────────────────────────────┐
                                    │  │       │              │  │                              │
                                    │  │       ├── WiFi? ─────┤──┤──► MQTT Broker              │
                                    │  │       │    YES       │  │     │                        │
                                    │  │       │    ├─ MQTT ──┤──┤──►  ├── Cloud Dashboard     │
                                    │  │       │    └─ HTTP ──┤──┤──►  └── Data Lake           │
                                    │  │       │              │  │                              │
                                    │  │       ├── BLE? ──────┤──┤──► Mobile App               │
                                    │  │       │    YES       │  │     ├── Live Display         │
                                    │  │       │    └─ GATT ──┤──┤──►  └── Config UI           │
                                    │  │       │              │  │                              │
                                    │  │       ├── LoRa? ─────┤──┤──► LoRaWAN Gateway          │
                                    │  │       │    YES       │  │     └── Network Server      │
                                    │  │       │    └─ Pkt ───┤──┤──►       └── Application    │
                                    │  │       │              │  │                              │
                                    │  │       └── UART ──────┤──┤──► Serial Console           │
                                    │  │            ALWAYS    │  │     └── Debug Terminal       │
                                    │  └──────────────────────┘  │                              │
                                    └─────────────────────────────┘                              │
                                                                                                │
                                    ┌───────────────────────────────────────────────────────────┘
                                    │
                                    ▼
                              ┌─────────────┐
                              │ Flash Store │  ← Fallback when all
                              │ (Circular)  │     channels unavailable
                              └─────────────┘
```

## 2. WiFi + MQTT Connection Flow

```
    ┌──────────┐                ┌──────────┐              ┌──────────────┐
    │ ESP32-S3 │                │  WiFi AP │              │ MQTT Broker  │
    └────┬─────┘                └────┬─────┘              └──────┬───────┘
         │                           │                           │
         │  ── WiFi Connect ──►     │                           │
         │     (SSID + Password)     │                           │
         │                           │                           │
         │  ◄── DHCP Assign ───     │                           │
         │     (IP Address)          │                           │
         │                           │                           │
         │  ── NTP Sync ──────────────────────────────────►     │
         │  ◄── Time Response ────────────────────────────      │
         │                           │                           │
         │  ── TCP Connect ──────────────────────────────►     │
         │  ── TLS Handshake ────────────────────────────►     │
         │  ◄── TLS Established ─────────────────────────      │
         │                           │                           │
         │  ── MQTT CONNECT ─────────────────────────────►     │
         │     (client_id, user, pw) │                           │
         │  ◄── CONNACK ────────────────────────────────       │
         │                           │                           │
         │  ── SUBSCRIBE ────────────────────────────────►     │
         │     (weather/{id}/cmd)    │                           │
         │  ◄── SUBACK ────────────────────────────────        │
         │                           │                           │
    ┌────┴─────────── OPERATIONAL LOOP ──────────────────┐      │
    │    │                           │                    │      │
    │    │  ── PUBLISH (QoS 1) ──────────────────────►  │      │
    │    │     topic: weather/{id}/telemetry              │      │
    │    │     payload: { readings JSON }                 │      │
    │    │  ◄── PUBACK ─────────────────────────────    │      │
    │    │                           │                    │      │
    │    │  ◄── PUBLISH (cmd) ──────────────────────    │      │
    │    │     topic: weather/{id}/cmd                    │      │
    │    │  ── PUBACK ──────────────────────────────►   │      │
    │    │                           │                    │      │
    └────┴───────────────────────────┴────────────────────┘      │
         │                           │                           │
```

## 3. BLE GATT Communication Flow

```
    ┌──────────┐                           ┌──────────────┐
    │ ESP32-S3 │                           │  Mobile App  │
    │ (GATT    │                           │  (Central)   │
    │  Server) │                           │              │
    └────┬─────┘                           └──────┬───────┘
         │                                        │
         │  ◄── BLE Scan ────────────────────    │
         │  ── Advertising ──────────────────►   │
         │     (name: "WeatherStation-XXXX")      │
         │                                        │
         │  ◄── Connect Request ─────────────    │
         │  ── Connection Established ───────►   │
         │                                        │
         │  ◄── Discover Services ───────────    │
         │  ── Service List ─────────────────►   │
         │     0x181A (Environmental Sensing)     │
         │     0xFFE0 (Custom Weather)            │
         │     0xFFE1 (Configuration)             │
         │                                        │
         │  ◄── Read Temperature (0x2A6E) ───    │
         │  ── Value: 2350 (23.50°C) ────────►   │
         │                                        │
         │  ◄── Enable Notify (0x2A6E) ──────    │
         │  ── Notification: 2355 ───────────►   │  (every 5s)
         │  ── Notification: 2348 ───────────►   │
         │                                        │
         │  ◄── Write WiFi SSID (FFE1-0010) ─    │
         │  ── Write Response ───────────────►   │
         │                                        │
         │  ◄── Write WiFi Pass (FFE1-0011) ─    │
         │  ── Write Response ───────────────►   │
         │                                        │
         │  ── Notify WiFi Status ───────────►   │
         │     (0x01 = connecting)                 │
         │  ── Notify WiFi Status ───────────►   │
         │     (0x02 = connected)                  │
         │                                        │
```

## 4. LoRa Communication Flow

```
    ┌──────────┐        ┌─────────┐        ┌──────────────┐      ┌────────────┐
    │ ESP32-S3 │        │ SX1276  │        │ LoRaWAN      │      │ Network    │
    │          │  SPI   │ Radio   │  RF    │ Gateway      │  IP  │ Server     │
    └────┬─────┘        └────┬────┘        └──────┬───────┘      └──────┬─────┘
         │                   │                     │                     │
         │ ── Set Freq ──►  │                     │                     │
         │ ── Set SF ────►  │                     │                     │
         │ ── Set BW ────►  │                     │                     │
         │ ── Set Power ──► │                     │                     │
         │                   │                     │                     │
    ┌────┴── TRANSMIT CYCLE ─┴─────────────────────┴─────────────────────┤
    │    │                   │                     │                     │
    │    │ ── Write FIFO ──► │                     │                     │
    │    │    (27-byte pkt)  │                     │                     │
    │    │ ── Set TX Mode ─► │                     │                     │
    │    │                   │ ══ RF TX ═══════►  │                     │
    │    │                   │   868/915 MHz       │                     │
    │    │                   │   SF7-SF12          │                     │
    │    │ ◄── DIO0 IRQ ──  │                     │                     │
    │    │    (TX Complete)   │                     │ ── Forward ──────► │
    │    │                   │                     │    (MQTT/HTTP)      │
    │    │                   │                     │                     │
    │    │ ── Set RX Mode ─► │                     │                     │
    │    │                   │ ◄═ RF RX ═════════ │                     │
    │    │ ◄── DIO0 IRQ ──  │                     │ ◄── Downlink ──── │
    │    │ ── Read FIFO ──►  │                     │                     │
    │    │ ◄── RX Data ────  │                     │                     │
    │    │                   │                     │                     │
    └────┴───────────────────┴─────────────────────┴─────────────────────┘
         │                   │                     │                     │

    Packet Format (27 bytes):
    ┌──────┬────────┬───────────┬──────┬──────┬──────┬──────┬──────┬──────┬─────┬──────┬──────┬──────┐
    │ Type │Dev ID  │ Timestamp │Temp  │Humid │Press │Wind  │WDir  │Rain  │ UV  │Light │Batt  │CRC16 │
    │ 1B   │ 2B     │ 4B        │ 2B   │ 2B   │ 4B   │ 2B   │ 2B   │ 2B   │ 1B  │ 2B   │ 1B   │ 2B   │
    └──────┴────────┴───────────┴──────┴──────┴──────┴──────┴──────┴──────┴─────┴──────┴──────┴──────┘
```

## 5. Channel Failover Logic

```
                    ┌─────────────┐
                    │ New Reading  │
                    │  Available   │
                    └──────┬──────┘
                           │
                    ┌──────▼──────┐
                    │ WiFi        │     YES    ┌──────────────┐
                    │ Connected?  ├───────────►│ MQTT Publish │
                    └──────┬──────┘            │ + HTTP Cache │
                           │ NO                └──────────────┘
                           │
                    ┌──────▼──────┐
                    │ Reconnect   │     YES    ┌──────────────┐
                    │ Attempt     ├───────────►│ Flush Buffer │
                    │ (3 tries)   │            │ via MQTT     │
                    └──────┬──────┘            └──────────────┘
                           │ FAIL
                           │
                    ┌──────▼──────┐
                    │ LoRa        │     YES    ┌──────────────┐
                    │ Available?  ├───────────►│ LoRa TX      │
                    └──────┬──────┘            │ (compact)    │
                           │ NO                └──────────────┘
                           │
                    ┌──────▼──────┐
                    │ Buffer to   │
                    │ Flash       │
                    │ (Circular)  │
                    └─────────────┘

    BLE: Always advertising (independent of failover)
    UART: Always logging (independent of failover)
```

## 6. HTTP API Request Flow

```
    ┌──────────┐              ┌──────────┐
    │  Client  │              │ ESP32-S3 │
    │ (Browser/│              │ HTTP     │
    │  curl)   │              │ Server   │
    └────┬─────┘              └────┬─────┘
         │                         │
         │  ── GET /api/v1/current ────►
         │                         │
         │                    ┌────┴────┐
         │                    │ Read    │
         │                    │ latest  │
         │                    │ reading │
         │                    └────┬────┘
         │                         │
         │  ◄── 200 OK ───────────
         │      Content-Type: application/json
         │      { temperature_c: 23.5, ... }
         │                         │
         │  ── POST /api/v1/config ────►
         │      { interval: 15 }   │
         │                    ┌────┴────┐
         │                    │ Update  │
         │                    │ NVS     │
         │                    │ config  │
         │                    └────┬────┘
         │  ◄── 200 OK ───────────
         │      { status: "ok" }   │
         │                         │
         │  ── POST /api/v1/ota ──────►
         │      Binary firmware    │
         │                    ┌────┴────┐
         │                    │ Verify  │
         │                    │ SHA-256 │
         │                    │ Flash   │
         │                    └────┬────┘
         │  ◄── 202 Accepted ─────
         │      { rebooting... }   │
         │                         │
```
