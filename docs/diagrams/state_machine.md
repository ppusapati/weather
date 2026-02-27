# State Machine Diagrams

## 1. Main System State Machine

```
                              ┌──────────┐
                              │  POWER   │
                              │   ON     │
                              └────┬─────┘
                                   │
                              ┌────▼─────┐
                              │  BOOT    │
                              │          │
                              │ • HW init│
                              │ • Load   │
                              │   config │
                              │ • Check  │
                              │   OTA    │
                              └────┬─────┘
                                   │
                         ┌─────────▼─────────┐
                         │   INITIALIZING    │
                         │                   │
                         │ • Init I2C/SPI    │
                         │ • Probe sensors   │
                         │ • Init WiFi/BLE   │
                         │ • Init LoRa       │
                         │ • Start UART      │
                         └─────────┬─────────┘
                                   │
               ┌───────── All OK? ─┤
               │ YES               │ NO (partial)
               │                   │
        ┌──────▼──────┐    ┌───────▼───────┐
        │   RUNNING   │    │   DEGRADED    │
        │   (Normal)  │    │   (Partial    │
        │             │◄───┤    sensors)   │
        │ • Sampling  │    │              │
        │ • Transmit  │    │ Same as       │
        │ • Serve API │    │ RUNNING but   │
        │             │    │ missing data  │
        └──────┬──────┘    │ flagged       │
               │           └───────────────┘
               │
     ┌─────────┼───────────────────┐
     │         │                   │
     │    ┌────▼─────┐      ┌─────▼──────┐
     │    │ SLEEPING │      │   ERROR    │
     │    │          │      │            │
     │    │ Light or │      │ • Log error│
     │    │ Deep     │      │ • Attempt  │
     │    │ sleep    │      │   recovery │
     │    │          │      │ • WDT may  │
     │    │ Wake on: │      │   trigger  │
     │    │ • Timer  │      │            │
     │    │ • GPIO   │      └─────┬──────┘
     │    └────┬─────┘            │
     │         │                  │ Recovery OK
     │         │ Timer expires    │
     └────◄────┘                  │
     │                            │
     └────────────────────────────┘
               │
        ┌──────▼──────┐
        │  UPDATING   │
        │  (OTA)      │
        │             │
        │ • Download  │
        │ • Verify    │
        │ • Flash     │
        │ • Reboot    │
        └──────┬──────┘
               │
          ┌────▼────┐
          │ REBOOT  │
          └─────────┘
```

## 2. WiFi Connection State Machine

```
    ┌──────────────┐
    │ DISCONNECTED │◄────────────────────────────────────────────────┐
    └──────┬───────┘                                                 │
           │                                                         │
           │ Has credentials?                                        │
           │                                                         │
    YES ───┤──── NO ───►┌───────────────┐                           │
           │            │ PROVISIONING  │  ← WiFi creds via BLE     │
           │            │ (BLE active)  │                            │
           │            └───────┬───────┘                            │
           │                    │ Credentials received               │
           │◄───────────────────┘                                    │
           │                                                         │
    ┌──────▼───────┐                                                 │
    │  CONNECTING  │                                                 │
    │              │                                                 │
    │  Attempt 1   │──── Timeout ────►┌──────────────┐              │
    │  (5s timeout)│                  │  RETRYING    │              │
    └──────┬───────┘                  │              │              │
           │                          │  Backoff:    │              │
           │ Success                  │  2s, 4s, 8s, │              │
           │                          │  16s, 30s    │              │
    ┌──────▼───────┐                  │              │──── Max ─────┘
    │  CONNECTED   │                  │  attempt++   │   retries
    │              │◄─────────────────┤              │   exceeded
    │ • Got IP     │  Retry success   └──────────────┘
    │ • NTP synced │
    │ • MQTT ready │
    └──────┬───────┘
           │
           │ Connection lost
           │
    ┌──────▼───────┐
    │ RECONNECTING │
    │              │
    │ Same backoff │──── Success ────► CONNECTED
    │ as RETRYING  │
    │              │──── Max retries ──► DISCONNECTED
    └──────────────┘                     (switch to LoRa)
```

## 3. BLE State Machine

```
    ┌──────────────┐
    │ BLE_INIT     │
    │              │
    │ • Init stack │
    │ • Set name   │
    │ • Add GATT   │
    │   services   │
    └──────┬───────┘
           │
    ┌──────▼───────┐
    │ ADVERTISING  │◄──────────────────────────────┐
    │              │                                │
    │ • Connectable│                                │
    │ • 100ms intv │                                │
    │ • Env Sensing│                                │
    └──────┬───────┘                                │
           │                                        │
           │ Central connects                       │
           │                                        │
    ┌──────▼───────┐                                │
    │  CONNECTED   │                                │
    │              │                                │
    │  ┌─────────────────────────────────────┐     │
    │  │ Handle GATT operations:             │     │
    │  │                                     │     │
    │  │  READ ──► Return sensor value       │     │
    │  │  WRITE ──► Update config / WiFi     │     │
    │  │  NOTIFY ──► Push sensor updates     │     │
    │  │            (if subscribed)           │     │
    │  └─────────────────────────────────────┘     │
    │              │                                │
    │              │ Disconnect                     │
    └──────────────┴────────────────────────────────┘
```

## 4. LoRa State Machine

```
    ┌──────────────┐
    │  LORA_INIT   │
    │              │
    │ • SPI init   │
    │ • Reset      │
    │ • Read ver   │
    │ • Config     │
    │   registers  │
    └──────┬───────┘
           │ Success
           │
    ┌──────▼───────┐
    │   STANDBY    │◄──────────────────────────────┐
    │              │                                │
    │  Power: low  │                                │
    │  Ready for   │                                │
    │  TX or RX    │                                │
    └──────┬───────┘                                │
           │                                        │
      ┌────┴────┐                                   │
      │         │                                   │
      ▼         ▼                                   │
┌─────────┐ ┌─────────┐                            │
│   TX    │ │   RX    │                             │
│         │ │         │                             │
│ • Load  │ │ • Set   │                             │
│   FIFO  │ │   RX    │                             │
│ • Start │ │   mode  │                             │
│   TX    │ │ • Wait  │                             │
│ • Wait  │ │   pkt   │                             │
│   done  │ │         │                             │
└────┬────┘ └────┬────┘                             │
     │           │                                  │
     │ TX Done   │ RX Done / Timeout                │
     │ (DIO0)    │ (DIO0)                           │
     │           │                                  │
     └─────┬─────┘                                  │
           │                                        │
           └────────────────────────────────────────┘
```

## 5. Power Management State Machine

```
                    ┌──────────────────┐
                    │   POWER_ACTIVE   │◄──────── Timer / GPIO wake
                    │                  │
                    │ All systems on   │
                    │ CPU: 240 MHz     │
                    │ WiFi: active     │
                    │ ~160 mA          │
                    └────────┬─────────┘
                             │
                    ┌────────┴─────────┐
                    │  Idle > 5s?      │
                    └────────┬─────────┘
                             │ YES
                    ┌────────▼─────────┐
                    │  MODEM_SLEEP     │◄──────── WiFi beacon interval
                    │                  │
                    │ CPU: active      │
                    │ WiFi: duty cycle │
                    │ ~20 mA           │
                    └────────┬─────────┘
                             │
                    ┌────────┴─────────┐
                    │  No TX pending   │
                    │  & idle > 30s?   │
                    └────────┬─────────┘
                             │ YES
                    ┌────────▼─────────┐
                    │  LIGHT_SLEEP     │
                    │                  │
                    │ CPU: paused      │
                    │ WiFi: off        │
                    │ RAM: retained    │
                    │ ~0.8 mA          │
                    │                  │
                    │ Wake: timer,     │
                    │       GPIO       │
                    └────────┬─────────┘
                             │
                    ┌────────┴─────────┐
                    │  Battery < 20%   │
                    │  or config?      │
                    └────────┬─────────┘
                             │ YES
                    ┌────────▼─────────┐
                    │  DEEP_SLEEP      │
                    │                  │
                    │ CPU: off         │
                    │ WiFi: off        │
                    │ RAM: lost        │
                    │ RTC: running     │
                    │ ~10 µA           │
                    │                  │
                    │ Wake: RTC timer  │
                    │  (configurable)  │
                    │ Full reboot on   │
                    │  wake            │
                    └──────────────────┘
```

## 6. OTA Update State Machine

```
    ┌──────────────┐
    │  OTA_IDLE    │◄──────────────────────────────────────────────┐
    │              │                                                │
    │  Check every │                                                │
    │  6 hours     │                                                │
    └──────┬───────┘                                                │
           │ Timer or HTTP POST                                     │
           │                                                        │
    ┌──────▼───────┐                                                │
    │  CHECKING    │                                                │
    │              │                                                │
    │  GET /ota    │──── No update ────► OTA_IDLE                  │
    │  version     │                                                │
    └──────┬───────┘                                                │
           │ Newer version                                          │
           │                                                        │
    ┌──────▼───────┐                                                │
    │ DOWNLOADING  │                                                │
    │              │                                                │
    │  Stream to   │──── Network error ──► RETRY (3x) ──► ABORT   │
    │  OTA partn.  │                                        │       │
    │              │                                        │       │
    └──────┬───────┘                                        │       │
           │ Complete                                       │       │
           │                                                │       │
    ┌──────▼───────┐                                        │       │
    │  VERIFYING   │                                        │       │
    │              │                                        │       │
    │  SHA-256     │──── Hash mismatch ──► ABORT ──────────┘       │
    │  check       │                                                │
    └──────┬───────┘                                                │
           │ Valid                                                   │
           │                                                        │
    ┌──────▼───────┐                                                │
    │  APPLYING    │                                                │
    │              │                                                │
    │  Set boot    │                                                │
    │  partition   │                                                │
    │  Reboot      │                                                │
    └──────┬───────┘                                                │
           │                                                        │
    ┌──────▼───────┐                                                │
    │ VALIDATING   │                                                │
    │              │                                                │
    │ New firmware │──── Boot OK ──► Mark stable ──► OTA_IDLE ─────┘
    │ self-test    │
    │              │──── Boot fail ──► ROLLBACK ──► Previous partition
    └──────────────┘
```
