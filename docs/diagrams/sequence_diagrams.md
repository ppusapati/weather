# Sequence Diagrams

## 1. System Boot Sequence

```
    Main     HAL      Drivers    Comms      Storage    Scheduler
     │        │         │         │           │          │
     │──Init──►         │         │           │          │
     │        │         │         │           │          │
     │   Clock Config   │         │           │          │
     │   GPIO Setup     │         │           │          │
     │   WDT Enable     │         │           │          │
     │        │         │         │           │          │
     │────────┼──Init───►         │           │          │
     │        │         │         │           │          │
     │        │  I2C Start       │           │          │
     │        │  SPI Start       │           │          │
     │        │         │         │           │          │
     │        │  Probe BME280    │           │          │
     │        │  ◄── OK ────     │           │          │
     │        │  Probe AS5600    │           │          │
     │        │  ◄── OK ────     │           │          │
     │        │  Probe SI1145    │           │          │
     │        │  ◄── OK ────     │           │          │
     │        │  Probe BH1750    │           │          │
     │        │  ◄── OK ────     │           │          │
     │        │  Init Anemometer │           │          │
     │        │  Init Rain Gauge │           │          │
     │        │         │         │           │          │
     │────────┼─────────┼──Init───►           │          │
     │        │         │         │           │          │
     │        │         │  Load WiFi creds ───►          │
     │        │         │         │  ◄── creds ──        │
     │        │         │  WiFi Connect      │          │
     │        │         │  ◄── IP ─────      │          │
     │        │         │  NTP Sync          │          │
     │        │         │  BLE Init          │          │
     │        │         │  LoRa Init (SPI)   │          │
     │        │         │  UART Start        │          │
     │        │         │  MQTT Connect      │          │
     │        │         │  HTTP Server Start │          │
     │        │         │         │           │          │
     │────────┼─────────┼─────────┼──Init─────►          │
     │        │         │         │           │          │
     │        │         │         │  Load config ──►     │
     │        │         │         │           │  ◄──     │
     │        │         │         │           │          │
     │────────┼─────────┼─────────┼───────────┼──Start──►
     │        │         │         │           │          │
     │        │         │         │           │   Register tasks:
     │        │         │         │           │   - Sensor read (5-30s)
     │        │         │         │           │   - MQTT publish (30s)
     │        │         │         │           │   - Status report (60s)
     │        │         │         │           │   - OTA check (6h)
     │        │         │         │           │   - WDT feed (10s)
     │        │         │         │           │          │
     │◄───────┼─────────┼─────────┼───────────┼── Ready ─┤
     │        │         │         │           │          │
     ▼  MAIN LOOP BEGINS         │           │          │
```

## 2. Sensor Read Cycle

```
    Scheduler   Pipeline    BME280    AS5600    Anemometer   Rain    SI1145   BH1750
        │          │          │         │          │          │        │        │
        │──Read────►          │         │          │          │        │        │
        │  Timer   │          │         │          │          │        │        │
        │          │──I2C R──►│         │          │          │        │        │
        │          │          │         │          │          │        │        │
        │          │◄─T,H,P──┤         │          │          │        │        │
        │          │          │         │          │          │        │        │
        │          │──I2C R───────────►│          │          │        │        │
        │          │◄─angle────────────┤          │          │        │        │
        │          │          │         │          │          │        │        │
        │          │──Get count────────────────►  │          │        │        │
        │          │◄─pulses───────────────────   │          │        │        │
        │          │          │         │          │          │        │        │
        │          │──Get tips──────────────────────────────►│        │        │
        │          │◄─tips──────────────────────────────────┤        │        │
        │          │          │         │          │          │        │        │
        │          │──I2C R──────────────────────────────────────────►│        │
        │          │◄─UV──────────────────────────────────────────────┤        │
        │          │          │         │          │          │        │        │
        │          │──I2C R────────────────────────────────────────────────────►
        │          │◄─lux──────────────────────────────────────────────────────┤
        │          │          │         │          │          │        │        │
        │          │          │         │          │          │        │        │
        │          │ ┌─────────────────────────────────────────────────────┐   │
        │          │ │ PIPELINE:                                           │   │
        │          │ │ 1. Calibrate raw values                            │   │
        │          │ │ 2. Apply EMA filter                                │   │
        │          │ │ 3. Validate ranges                                 │   │
        │          │ │ 4. Compute derived values                          │   │
        │          │ │ 5. Package WeatherReading                          │   │
        │          │ └─────────────────────────────────────────────────────┘   │
        │          │          │         │          │          │        │        │
        │◄─Reading─┤          │         │          │          │        │        │
        │          │          │         │          │          │        │        │
        │──Queue───────────────────────────────────────────────────────────────►
        │  to comms│          │         │          │          │        │        │
        │  & store │          │         │          │          │        │        │
```

## 3. MQTT Publish with Retry

```
    Scheduler    MQTT Client    WiFi Manager    Broker     Flash Store
        │            │              │              │            │
        │──Publish──►│              │              │            │
        │  (reading) │              │              │            │
        │            │──Check conn──►              │            │
        │            │◄─Connected──┤              │            │
        │            │              │              │            │
        │            │──PUBLISH (QoS 1)───────────►            │
        │            │              │              │            │
        │            │  ┌─ Wait PUBACK (5s timeout) ─┐         │
        │            │  │                             │         │
        │            │◄─┤── PUBACK ──────────────────┤         │
        │            │  └─────────────────────────────┘         │
        │◄─Success──┤              │              │            │
        │            │              │              │            │
        │            │              │              │            │
        │   ═══ FAILURE SCENARIO ═══               │            │
        │            │              │              │            │
        │──Publish──►│              │              │            │
        │            │──Check conn──►              │            │
        │            │◄─Disconnected┤              │            │
        │            │              │              │            │
        │            │──Reconnect──►│              │            │
        │            │◄─Fail───────┤              │            │
        │            │              │              │            │
        │            │──Buffer─────────────────────────────────►
        │            │              │              │   Store in │
        │            │              │              │   circular │
        │            │              │              │   buffer   │
        │◄─Buffered─┤              │              │            │
        │            │              │              │            │
        │   ═══ RECONNECT + FLUSH ═══              │            │
        │            │              │              │            │
        │            │◄─WiFi Reconnected──────────┤            │
        │            │──MQTT CONNECT──────────────►            │
        │            │◄─CONNACK───────────────────┤            │
        │            │              │              │            │
        │            │──Read buffer────────────────────────────►
        │            │◄─Buffered readings──────────────────────┤
        │            │              │              │            │
        │            │  ┌─ For each buffered reading ──────┐   │
        │            │  │ PUBLISH ────────────────►         │   │
        │            │  │ ◄── PUBACK ─────────────         │   │
        │            │  └──────────────────────────────────┘   │
        │            │              │              │            │
        │            │──Clear buffer───────────────────────────►
        │◄─Flushed──┤              │              │            │
```

## 4. BLE WiFi Provisioning Flow

```
    Mobile App     BLE Stack     WiFi Manager    NVS Storage
        │              │              │              │
        │──Scan────────►              │              │
        │◄─Advertise──┤              │              │
        │              │              │              │
        │──Connect────►│              │              │
        │◄─Connected──┤              │              │
        │              │              │              │
        │──Discover services─────────►              │
        │◄─Service list──────────────┤              │
        │  (0x181A, 0xFFE0, 0xFFE1)  │              │
        │              │              │              │
        │──Read WiFi Status──────────►              │
        │  (FFE1-0030) │              │              │
        │◄─0x00 (not configured)─────┤              │
        │              │              │              │
        │──Write SSID──►              │              │
        │  (FFE1-0010) │              │              │
        │  "HomeWiFi"  │              │              │
        │◄─Write OK───┤              │              │
        │              │              │              │
        │──Write Password──────────►  │              │
        │  (FFE1-0011) │              │              │
        │  "secret123" │              │              │
        │◄─Write OK───┤              │              │
        │              │              │              │
        │              │──Save creds──────────────────►
        │              │              │   ◄── OK ────┤
        │              │              │              │
        │              │──Connect─────►              │
        │              │              │  WiFi connect │
        │              │              │  attempt      │
        │              │              │              │
        │◄─Notify Status──────────── │              │
        │  0x01 (connecting)          │              │
        │              │              │              │
        │              │◄─Connected──┤              │
        │              │              │              │
        │◄─Notify Status──────────── │              │
        │  0x02 (connected)           │              │
        │              │              │              │
        │──Read IP Address───────────►              │
        │◄─"192.168.1.42"────────────┤              │
        │              │              │              │
        │──Disconnect──►              │              │
        │              │              │              │
```

## 5. OTA Update Sequence

```
    Scheduler    OTA Manager    HTTP Client    Flash      Bootloader
        │            │              │            │            │
        │──Check────►│              │            │            │
        │  (6h timer)│              │            │            │
        │            │──GET /ota/version──────►  │            │
        │            │◄─{version, sha256}─────  │            │
        │            │              │            │            │
        │            │ Compare versions          │            │
        │            │ Current: 1.0.0            │            │
        │            │ Remote:  1.1.0            │            │
        │            │              │            │            │
        │            │──GET /ota/firmware──────► │            │
        │            │              │            │            │
        │            │  ┌── Stream response ────────────┐    │
        │            │  │                               │    │
        │            │  │ ◄── chunk (4KB) ──           │    │
        │            │  │ ── Write OTA partition ──────►│    │
        │            │  │ ◄── chunk (4KB) ──           │    │
        │            │  │ ── Write OTA partition ──────►│    │
        │            │  │ ...                           │    │
        │            │  └───────────────────────────────┘    │
        │            │              │            │            │
        │            │ SHA-256 verify             │            │
        │            │ ◄── Match ──              │            │
        │            │              │            │            │
        │            │──Set boot partition──────►│            │
        │            │              │            │            │
        │            │──Reboot──────────────────────────────►│
        │            │              │            │            │
        │            │              │            │   Boot new │
        │            │              │            │   partition│
        │            │              │            │            │
        │◄───────────┤── Self-test ──────────────┤            │
        │            │              │            │            │
        │            │──Mark stable─────────────►│            │
        │            │              │            │            │
```

## 6. Deep Sleep / Wake Cycle

```
    Power Mgr    Scheduler    Comms        Drivers     RTC
        │            │          │             │          │
        │◄─Battery   │          │             │          │
        │  < 20%     │          │             │          │
        │            │          │             │          │
        │──Stop──────►          │             │          │
        │  scheduler │          │             │          │
        │            │          │             │          │
        │──Final status────────►│             │          │
        │            │   MQTT: battery_low   │          │
        │            │   LoRa: status pkt    │          │
        │            │          │             │          │
        │──Flush─────────────────────────────►│          │
        │  store     │          │             │          │
        │            │          │             │          │
        │──Shutdown──────────►  │             │          │
        │  comms     │   WiFi off             │          │
        │            │   BLE off              │          │
        │            │   LoRa sleep           │          │
        │            │          │             │          │
        │──Shutdown────────────────────────►  │          │
        │  sensors   │          │   Power off │          │
        │            │          │             │          │
        │──Set wake timer──────────────────────────────►│
        │  (300s default)       │             │          │
        │            │          │             │          │
        │──Enter deep sleep────────────────────────────►│
        │            │          │             │   ~10µA  │
        │            │          │             │          │
        │  .... 300 seconds pass ....        │          │
        │            │          │             │          │
        │◄─────────────────────────────── RTC Wake ────┤
        │            │          │             │          │
        │  Full reboot sequence begins                  │
        │  (See Boot Sequence diagram)                  │
```
