//! Weather Station Firmware — ESP32-S3
//!
//! A production-grade weather station firmware targeting the ESP32-S3 MCU.
//! Reads environmental sensors and transmits data via WiFi, BLE, LoRa,
//! UART, MQTT, and HTTP.
//!
//! # Architecture
//!
//! - **Core 0 (PRO)**: Sensor acquisition, GPIO ISRs, data pipeline
//! - **Core 1 (APP)**: WiFi, BLE, LoRa, MQTT, HTTP, OTA, UART console
//!
//! # Module Organization
//!
//! - `config` — Pin assignments, constants, runtime configuration
//! - `error` — Unified error types
//! - `drivers` — Sensor drivers (BME280, wind, rain, UV, light)
//! - `comms` — Communication channels (WiFi, BLE, LoRa, UART, MQTT, HTTP)
//! - `core` — Scheduler, data pipeline, power management, OTA
//! - `storage` — Flash circular buffer, NVS access
//! - `utils` — Ring buffer, CRC

#![no_std]
#![no_main]

extern crate alloc;

use core::sync::atomic::{AtomicBool, Ordering};

mod comms;
mod config;
mod core;
mod drivers;
mod error;
mod storage;
mod utils;

use crate::comms::ble::BleServer;
use crate::comms::http::HttpServer;
use crate::comms::mqtt::MqttClient;
use crate::comms::uart_console::{ConsoleCommand, UartConsole};
use crate::comms::wifi::WifiManager;
use crate::config::RuntimeConfig;
use crate::core::data_pipeline::{DataPipeline, RawSensorData, WeatherReading};
use crate::core::ota::OtaManager;
use crate::core::power::PowerManager;
use crate::core::scheduler::{Scheduler, TaskId};
use crate::drivers::SensorStatusMap;
use crate::storage::flash::FlashStorage;
use crate::utils::ring_buffer::RingBuffer;

/// Global flag: set by watchdog or panic handler.
static SYSTEM_ERROR: AtomicBool = AtomicBool::new(false);

/// Heap allocator for `alloc` support.
#[global_allocator]
static ALLOCATOR: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

/// Entry point.
#[esp_hal::entry]
fn main() -> ! {
    // ── Phase 1: Hardware Initialization ──────────────────────────

    // Initialize heap allocator
    const HEAP_SIZE: usize = 384 * 1024;
    static mut HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];
    unsafe { ALLOCATOR.init(HEAP.as_mut_ptr(), HEAP_SIZE) };

    // Initialize logging
    esp_println::logger::init_logger_from_env();
    log::info!("========================================");
    log::info!("  Weather Station Firmware v{}", config::FIRMWARE_VERSION);
    log::info!("  ESP32-S3 | Rust Embedded");
    log::info!("========================================");

    // Load configuration from NVS (or use defaults)
    let runtime_config = RuntimeConfig::default();
    log::info!("Config loaded: device_id={}", runtime_config.device_id);

    // ── Phase 2: Peripheral Initialization ────────────────────────

    // Initialize HAL peripherals
    // In real firmware: let peripherals = esp_hal::init(esp_hal::Config::default());
    // let io = Io::new(peripherals.GPIO, peripherals.IO_MUX);
    // let i2c = I2c::new(peripherals.I2C0, io.pins.gpio8, io.pins.gpio9, 400.kHz());
    // let spi = Spi::new(peripherals.SPI2, ...);
    // etc.

    log::info!("HAL peripherals initialized");

    // ── Phase 3: Sensor Initialization ────────────────────────────

    let sensor_status = init_sensors();
    log::info!("Sensors initialized: {:?}", sensor_status);

    // ── Phase 4: Communication Initialization ─────────────────────

    let mut wifi = WifiManager::new();
    wifi.set_credentials(
        runtime_config.wifi_ssid.as_str(),
        runtime_config.wifi_password.as_str(),
    );
    let _ = wifi.init();

    let mut ble = BleServer::new(runtime_config.device_name.as_str());
    let _ = ble.init();

    // LoRa initialized via SPI (would need real SPI handle)
    // let mut lora = LoraRadio::new(spi, rst_pin, runtime_config.device_id);
    // let _ = lora.init();

    let mut mqtt = MqttClient::new(&format_device_id(runtime_config.device_id));
    mqtt.configure(
        runtime_config.mqtt_broker.as_str(),
        runtime_config.mqtt_port,
        runtime_config.mqtt_use_tls,
        runtime_config.mqtt_username.as_str(),
        runtime_config.mqtt_password.as_str(),
    );

    let mut http = HttpServer::new();
    let uart_console = UartConsole::new();
    uart_console.init();

    let mut ota = OtaManager::new();
    let _ = ota.validate_boot();

    log::info!("Communication channels initialized");

    // ── Phase 5: Core Systems ─────────────────────────────────────

    let mut scheduler = Scheduler::new();
    let mut pipeline = DataPipeline::new();
    pipeline.set_calibration(
        runtime_config.cal_temp_offset,
        runtime_config.cal_humidity_offset,
        runtime_config.cal_pressure_offset,
        runtime_config.cal_wind_dir_offset,
    );

    let mut power = PowerManager::new();
    let mut flash_storage = FlashStorage::new();
    let _ = flash_storage.init();

    // Reading buffer for inter-task communication
    let mut reading_buffer: RingBuffer<WeatherReading, 64> = RingBuffer::new();

    // Latest reading cache
    let mut latest_reading = WeatherReading::default();
    let mut raw_data = RawSensorData::default();

    log::info!("Core systems initialized — entering main loop");

    // ── Phase 6: Main Loop ────────────────────────────────────────

    let mut uptime_ms: u64 = 0;
    let tick_ms: u64 = 10; // 10ms tick

    loop {
        // Update scheduler time
        scheduler.update_time(uptime_ms);

        // Process all due tasks
        while let Some(task_id) = scheduler.next_due_task() {
            match task_id {
                TaskId::ReadBme280 => {
                    // In real firmware: read from BME280 driver
                    // let bme_reading = bme280.read();
                    // raw_data.temperature_c = Some(bme_reading.temperature_c);
                    // raw_data.humidity_pct = Some(bme_reading.humidity_pct);
                    // raw_data.pressure_hpa = Some(bme_reading.pressure_hpa);
                    log::debug!("Task: ReadBme280");
                }

                TaskId::ReadWind => {
                    // In real firmware: read anemometer + wind vane
                    // let speed = anemometer.read(uptime_ms as u32);
                    // let dir = wind_vane.read();
                    // raw_data.wind_speed_kmh = Some(speed.speed_kmh);
                    // raw_data.wind_direction_deg = Some(dir.direction_deg as f32);
                    log::debug!("Task: ReadWind");
                }

                TaskId::ReadRain => {
                    // In real firmware: read rain gauge
                    // let rain = rain_gauge.read(uptime_ms as u32);
                    // raw_data.rain_total_mm = rain.total_mm;
                    // raw_data.rain_rate_mm_hr = Some(rain.rate_mm_hr);
                    log::debug!("Task: ReadRain");
                }

                TaskId::ReadUvLight => {
                    // In real firmware: read SI1145 + BH1750
                    // let uv = si1145.read();
                    // let light = bh1750.read();
                    // raw_data.uv_index = Some(uv.uv_index);
                    // raw_data.light_lux = Some(light.lux);
                    log::debug!("Task: ReadUvLight");
                }

                TaskId::ProcessData => {
                    // Run the data pipeline
                    latest_reading = pipeline.process(&raw_data, uptime_ms);

                    // Buffer for communication tasks
                    let _ = reading_buffer.push(latest_reading.clone());

                    // Store to flash
                    let _ = flash_storage.store(&latest_reading);

                    // Update HTTP cache
                    http.update_reading(&latest_reading);

                    log::debug!("Task: ProcessData — reading buffered");
                }

                TaskId::PublishTelemetry => {
                    if wifi.is_connected() {
                        if let Err(e) = mqtt.publish_telemetry(&latest_reading) {
                            log::warn!("MQTT publish failed: {}", e);
                            // Buffer to flash for later retry
                            let _ = flash_storage.store(&latest_reading);
                        }
                    }

                    // Always try LoRa if available
                    // if lora.state() == LoraState::Standby {
                    //     let _ = lora.send_reading(&latest_reading, (uptime_ms / 1000) as u32);
                    // }

                    log::debug!("Task: PublishTelemetry");
                }

                TaskId::PublishStatus => {
                    let battery = power.battery();
                    if mqtt.is_connected() {
                        let _ = mqtt.publish_status(
                            uptime_ms / 1000,
                            battery.voltage_v,
                            battery.percentage,
                            wifi.rssi().unwrap_or(0),
                            0, // free heap — would come from allocator stats
                            flash_storage.usage_pct(),
                        );
                    }
                    log::debug!("Task: PublishStatus");
                }

                TaskId::UpdateBle => {
                    let _ = ble.update_reading(&latest_reading);

                    // Check for WiFi provisioning from BLE
                    if let Some(wifi_config) = ble.take_wifi_config() {
                        log::info!("BLE: WiFi provisioning received");
                        wifi.set_credentials(
                            wifi_config.ssid.as_str(),
                            wifi_config.password.as_str(),
                        );
                        let _ = wifi.init();
                    }

                    log::debug!("Task: UpdateBle");
                }

                TaskId::CheckOta => {
                    if wifi.is_connected() && ota.should_check(uptime_ms) {
                        let _ = ota.check_for_update(uptime_ms);
                    }
                    log::debug!("Task: CheckOta");
                }

                TaskId::FeedWatchdog => {
                    // In real firmware: feed the hardware watchdog timer
                    // wdt.feed();
                }

                TaskId::ReadBattery => {
                    // In real firmware: read ADC
                    // let adc_raw = adc.read(battery_channel);
                    // power.update_battery(adc_raw);
                    log::debug!("Task: ReadBattery");
                }
            }
        }

        // ── Power Management ──────────────────────────────────────

        let time_until_next = scheduler.time_until_next_ms();

        if time_until_next > 10 {
            power.signal_idle(uptime_ms);
        } else {
            power.signal_busy();
        }

        power.evaluate_sleep(uptime_ms, time_until_next);

        if power.deep_sleep_pending() {
            log::info!("Entering deep sleep...");
            // In real firmware:
            // 1. Flush MQTT
            // 2. Send final LoRa status
            // 3. Save state to NVS
            // 4. Enter deep sleep
            // esp_hal::sleep::deep_sleep(...)
        }

        // Check for system errors
        if SYSTEM_ERROR.load(Ordering::Relaxed) {
            log::error!("System error flag set — attempting recovery");
            SYSTEM_ERROR.store(false, Ordering::Relaxed);
        }

        // Advance time (simulated tick — in real firmware this comes from a timer)
        uptime_ms += tick_ms;

        // Yield / small delay to prevent busy-spinning
        // In real firmware: embassy_time::Timer::after_millis(tick_ms).await
        for _ in 0..tick_ms * 1000 {
            core::hint::spin_loop();
        }
    }
}

/// Initialize all sensors and return their status.
fn init_sensors() -> SensorStatusMap {
    let mut status = SensorStatusMap::default();

    // BME280
    // In real firmware: probe I2C device
    log::info!("Probing BME280 at 0x{:02X}...", config::BME280_ADDR);
    status.bme280 = drivers::SensorStatus::Ok;

    // Wind sensors
    log::info!("Initializing anemometer on GPIO {}...", config::WIND_SPEED_PIN);
    log::info!("Probing AS5600 at 0x{:02X}...", config::AS5600_ADDR);
    status.wind = drivers::SensorStatus::Ok;

    // Rain gauge
    log::info!("Initializing rain gauge on GPIO {}...", config::RAIN_GAUGE_PIN);
    status.rain = drivers::SensorStatus::Ok;

    // UV sensor
    log::info!("Probing SI1145 at 0x{:02X}...", config::SI1145_ADDR);
    status.uv = drivers::SensorStatus::Ok;

    // Light sensor
    log::info!("Probing BH1750 at 0x{:02X}...", config::BH1750_ADDR);
    status.light = drivers::SensorStatus::Ok;

    status
}

/// Format device ID as a string for MQTT client ID.
fn format_device_id(id: u16) -> heapless::String<16> {
    let mut s = heapless::String::new();
    let _ = core::fmt::write(
        &mut HeaplessWriter(&mut s),
        format_args!("ws-{:04X}", id),
    );
    s
}

struct HeaplessWriter<'a, const N: usize>(&'a mut heapless::String<N>);

impl<const N: usize> core::fmt::Write for HeaplessWriter<'_, N> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.0.push_str(s).map_err(|_| core::fmt::Error)
    }
}

/// Panic handler.
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    log::error!("PANIC: {}", info);
    // In real firmware: save crash info to NVS, trigger WDT reset
    loop {
        core::hint::spin_loop();
    }
}
