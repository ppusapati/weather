# Troubleshooting

## Sensor Issues

### BME280 not responding
- **Symptom**: `SensorStatus::NotFound` for BME280, temperature/humidity/pressure all `None`
- **Check**: I2C address (0x76 vs 0x77 — some breakouts use 0x77 with SDO pulled high)
- **Check**: I2C pull-ups (4.7k on SDA/SCL)
- **Check**: Wiring — SDA to PB7, SCL to PB6
- **Fix**: If address is 0x77, update `BME280_ADDR` in `config.rs`

### BME280 stuck sensor detection
- **Symptom**: `SensorStatus::Degraded` for BME280
- **Cause**: Sensor returns identical values for 10+ consecutive reads
- **Check**: Sensor may be in a fault state — power cycle the sensor
- **Check**: BME280 may be in sleep mode — verify forced-mode trigger

### Wind speed always zero
- **Symptom**: `wind_speed_kmh` is always `Some(0.0)`
- **Check**: Anemometer reed switch wiring to PB0 (TIM3_CH3)
- **Check**: 10k pull-up resistor on PB0
- **Check**: GPIO interrupt configuration (rising edge)
- **Test**: Manually trigger the reed switch, check pulse counter via UART `reading` command

### Wind direction stuck at one value
- **Symptom**: `wind_dir_deg` doesn't change when vane rotates
- **Check**: AS5600 I2C communication (address 0x36)
- **Check**: Diode magnet placement and rotation axis alignment
- **Check**: AS5600 `MagnetStatus` — should report `MagnetDetected`

### Rain gauge over-counting
- **Symptom**: Rain accumulation increasing without rain
- **Cause**: Electrical noise on PB1 causing false triggers
- **Fix**: Add hardware debounce (100nF cap across reed switch)
- **Fix**: Check software debounce interval in rain gauge driver

### UV readings saturated
- **Symptom**: `uv_index` always at maximum (15.0)
- **Check**: SI1145 coefficient registers — must be written during init
- **Check**: Sensor orientation (must face upward, no obstruction)

## SD Card Issues

### SD card not detected
- **Symptom**: "No SD card inserted" in log, firmware falls back to flash storage
- **Check**: Card is fully inserted into micro-SD holder (J22)
- **Check**: PD11 detect pin — should go LOW when card is inserted
- **Check**: R27 (10kΩ pull-up) on PD11 is populated
- **Check**: Card is FAT32 formatted (SDXC cards with exFAT are not supported)

### SD card init fails
- **Symptom**: "SD card init failed" in log
- **Check**: SPI2 bus connections (PB13 SCK, PB14 MISO, PB15 MOSI, PD10 CS)
- **Check**: C38 (100nF) decoupling cap on SD card VCC
- **Check**: Card is not write-protected
- **Note**: Init uses 400 kHz SPI clock; if SPI2 bus is noisy, check signal integrity

### SD card write errors
- **Symptom**: "SD store failed" in log, data falls back to flash
- **Check**: Card has free space (FAT32 max 32 GB partition)
- **Check**: `/weather/` directory exists (created automatically on init)
- **Check**: SPI2 bus sharing with W5500 — verify CS toggling is correct
- **Recovery**: Remove card, format as FAT32, re-insert

## Communication Issues

### WiFi won't connect
- **Check**: SSID and password via UART `wifi status` command
- **Check**: WiFi signal strength — station may be too far from AP
- **Check**: 2.4 GHz only (ATWINC1500 does not support 5 GHz)
- **Retry**: WiFi uses exponential backoff (2s, 4s, 8s... up to 30s)
- **Max retries**: 5 attempts before `WifiState::Failed`
- **Recovery**: Power cycle or send BLE provisioning with new credentials

### MQTT publish failures
- **Symptom**: `MQTT publish failed` in serial log
- **Check**: Broker address and port in runtime config
- **Check**: WiFi is connected first (`wifi status`)
- **Check**: Broker authentication credentials
- **Note**: Failed messages are buffered to flash and replayed on reconnect

### BLE not discoverable
- **Check**: RN4870 BLE module is powered and RST pin (PD8) released
- **Check**: Device name in runtime config
- **Fix**: Some phones require clearing BLE cache after firmware update

### LoRa no acknowledgement
- **Check**: Frequency setting matches regional band (868 MHz EU, 915 MHz US)
- **Check**: Spreading factor and bandwidth match gateway settings
- **Check**: Antenna connection to SX1276 module
- **Check**: TX power setting (default 14 dBm)

## Power Issues

### Battery draining fast
- **Check**: Power mode via UART `status` command
- **Check**: WiFi reconnect loop (constant retries drain battery)
- **Fix**: If WiFi is unavailable, firmware should fall back to LoRa-only mode
- **Expected**: ~250 mA active, ~2 µA standby

### Won't enter deep sleep
- **Check**: Battery percentage — deep sleep triggers below 20%
- **Check**: Active WiFi connections prevent deep sleep
- **Check**: Pending OTA update blocks deep sleep

### Solar charging not working
- **Check**: Solar panel voltage (should be ~6V in direct sunlight)
- **Check**: TP4056 charge LED indicators
- **Check**: USB-C is disconnected (some TP4056 boards conflict with dual input)

## Flash Storage Issues

### "Flash store failed" errors
- **Check**: Flash partition is properly initialized
- **Check**: Available space via UART `status` command
- **Note**: Circular buffer overwrites oldest records when full

### Data corruption after power loss
- **Check**: CRC verification errors in flash records
- **Recovery**: The circular buffer skips corrupted records automatically
- **Prevention**: Ensure clean shutdown via deep sleep rather than hard power cut

## Build Issues

### Compilation fails with missing target
```bash
rustup target add thumbv7em-none-eabihf  # ARM Cortex-M4F target
cargo install probe-rs-tools  # Flashing tools
```

### Linker errors about memory
- **Check**: `memory.x` flash/RAM regions match STM32F407 (1 MB Flash, 128 KB RAM, 64 KB CCM)
- **Check**: HEAP_SIZE in `config.rs` (default 48 KB)
- **Fix**: Reduce binary size with `opt-level = "z"` and `lto = true`

## UART Console Commands

Use these for live diagnostics:

| Command | Description |
|---------|-------------|
| `status` | Show all sensor statuses, WiFi, battery |
| `reading` | Display current weather reading |
| `config` | Show runtime configuration |
| `calibrate <sensor> <offset>` | Set calibration offset |
| `wifi status` | Show WiFi connection details |
| `dump` | Dump flash storage records |
| `reset` | Restart firmware |
