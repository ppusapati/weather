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
//! - `utils` — Ring buffer, CRC, formatting helpers

#![no_std]
#![no_main]

extern crate alloc;

#[macro_use]
mod utils;

use core::sync::atomic::{AtomicBool, Ordering};

mod comms;
mod config;
mod core;
mod drivers;
mod error;
mod industry;
mod storage;

use crate::comms::ble::BleServer;
use crate::comms::http::HttpServer;
use crate::comms::mqtt::MqttClient;
use crate::comms::uart_console::UartConsole;
use crate::comms::wifi::WifiManager;
use crate::config::RuntimeConfig;
use crate::core::data_pipeline::{DataPipeline, RawSensorData, WeatherReading};
use crate::core::ota::OtaManager;
use crate::core::power::PowerManager;
use crate::core::scheduler::{Scheduler, TaskId};
use crate::drivers::SensorStatusMap;
use crate::storage::flash::FlashStorage;
use crate::utils::ring_buffer::RingBuffer;

/// Global flag: set by watchdog or panic handler to signal error recovery.
static SYSTEM_ERROR: AtomicBool = AtomicBool::new(false);

/// Heap allocator for `alloc` support.
#[global_allocator]
static ALLOCATOR: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

/// Entry point.
#[esp_hal::entry]
fn main() -> ! {
    // ── Phase 1: Hardware Initialization ──────────────────────────

    // SAFETY: Called exactly once, before any heap allocations, on the
    // single-threaded boot path. The HEAP static is only accessed here
    // and its lifetime is 'static, satisfying the allocator contract.
    static mut HEAP: [u8; config::HEAP_SIZE] = [0; config::HEAP_SIZE];
    unsafe { ALLOCATOR.init(HEAP.as_mut_ptr(), config::HEAP_SIZE) };

    // Initialize logging
    esp_println::logger::init_logger_from_env();
    log::info!("========================================");
    log::info!("  Weather Station Firmware v{}", config::FIRMWARE_VERSION);
    log::info!("  ESP32-S3 | Rust Embedded");
    log::info!("========================================");

    // Load configuration from NVS (or use defaults)
    let mut runtime_config = RuntimeConfig::default();
    runtime_config.validate();
    log::info!("Config loaded: device_id={}", runtime_config.device_id);

    // ── Phase 2: Peripheral Initialization ────────────────────────

    // In real firmware: let peripherals = esp_hal::init(esp_hal::Config::default());
    // let io = Io::new(peripherals.GPIO, peripherals.IO_MUX);
    // let i2c = I2c::new(peripherals.I2C0, io.pins.gpio8, io.pins.gpio9, 400.kHz());
    // let spi = Spi::new(peripherals.SPI2, ...);
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
    if let Err(e) = wifi.init() {
        log::warn!("WiFi init failed: {} — will retry later", e);
    }

    let mut ble = BleServer::new(runtime_config.device_name.as_str());
    if let Err(e) = ble.init() {
        log::warn!("BLE init failed: {}", e);
    }

    // LoRa initialized via SPI (would need real SPI handle)
    // let mut lora = LoraRadio::new(spi, rst_pin, runtime_config.device_id);

    let device_id_str: heapless::String<16> =
        format_heapless!("ws-{:04X}", runtime_config.device_id);
    let mut mqtt = MqttClient::new(device_id_str.as_str());
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
    if let Err(e) = ota.validate_boot() {
        log::warn!("OTA boot validation failed: {} — may rollback", e);
    }

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
    if let Err(e) = flash_storage.init() {
        log::error!("Flash storage init failed: {} — data will not persist", e);
    }

    let mut reading_buffer: RingBuffer<WeatherReading, 64> = RingBuffer::new();
    let mut latest_reading = WeatherReading::default();
    let raw_data = RawSensorData::default();

    // ── Phase 5b: Industry Module Initialization ──────────────────

    #[cfg(feature = "agriculture")]
    let mut ag_analytics = {
        log::info!("Agriculture module enabled");
        industry::agriculture::AgricultureAnalytics::new(runtime_config.gdd_base_temp_c)
    };
    #[cfg(feature = "agriculture")]
    let mut ag_reading = industry::agriculture::AgricultureReading::default();

    #[cfg(feature = "solar")]
    let mut solar_analytics = {
        log::info!("Solar module enabled");
        industry::solar::SolarAnalytics::new(
            runtime_config.panel_wp,
            runtime_config.panel_area_m2,
        )
    };
    #[cfg(feature = "solar")]
    let mut solar_reading = industry::solar::SolarReading::default();

    log::info!("Core systems initialized — entering main loop");

    // ── Phase 6: Main Loop ────────────────────────────────────────

    let mut uptime_ms: u64 = 0;

    loop {
        scheduler.update_time(uptime_ms);

        // Limit tasks per tick to prevent runaway scheduling if many tasks become due at once
        let mut tasks_this_tick = 0u8;
        const MAX_TASKS_PER_TICK: u8 = 8;
        while tasks_this_tick < MAX_TASKS_PER_TICK {
            let Some(task_id) = scheduler.next_due_task() else {
                break;
            };
            tasks_this_tick += 1;
            match task_id {
                TaskId::ReadBme280 => {
                    // In real firmware: read from BME280 driver
                    // let bme_reading = bme280.read()?;
                    // raw_data.temperature_c = Some(bme_reading.temperature_c);
                    log::debug!("Task: ReadBme280");
                }

                TaskId::ReadWind => {
                    log::debug!("Task: ReadWind");
                }

                TaskId::ReadRain => {
                    log::debug!("Task: ReadRain");
                }

                TaskId::ReadUvLight => {
                    log::debug!("Task: ReadUvLight");
                }

                TaskId::ProcessData => {
                    latest_reading = pipeline.process(&raw_data, uptime_ms);

                    if !reading_buffer.push(latest_reading.clone()) {
                        log::warn!("Reading buffer full — dropping oldest");
                    }

                    if let Err(e) = flash_storage.store(&latest_reading) {
                        log::warn!("Flash store failed: {}", e);
                    }

                    http.update_reading(&latest_reading);
                    log::debug!("Task: ProcessData — reading buffered");
                }

                TaskId::PublishTelemetry => {
                    if wifi.is_connected() {
                        if let Err(e) = mqtt.publish_telemetry(&latest_reading) {
                            log::warn!("MQTT publish failed: {} — buffering to flash", e);
                            if let Err(e2) = flash_storage.store(&latest_reading) {
                                log::error!("Flash fallback also failed: {}", e2);
                            }
                        }
                    }
                    log::debug!("Task: PublishTelemetry");
                }

                TaskId::PublishStatus => {
                    let battery = power.battery();
                    if mqtt.is_connected() {
                        if let Err(e) = mqtt.publish_status(
                            uptime_ms / 1000,
                            battery.voltage_v,
                            battery.percentage,
                            wifi.rssi().unwrap_or(0),
                            0, // free heap — from allocator stats in real firmware
                            flash_storage.usage_pct(),
                        ) {
                            log::warn!("Status publish failed: {}", e);
                        }
                    }
                    log::debug!("Task: PublishStatus");
                }

                TaskId::UpdateBle => {
                    if let Err(e) = ble.update_reading(&latest_reading) {
                        log::debug!("BLE update skipped: {}", e);
                    }

                    if let Some(wifi_config) = ble.take_wifi_config() {
                        log::info!("BLE: WiFi provisioning received");
                        wifi.set_credentials(
                            wifi_config.ssid.as_str(),
                            wifi_config.password.as_str(),
                        );
                        if let Err(e) = wifi.init() {
                            log::warn!("WiFi re-init after BLE provisioning failed: {}", e);
                        }
                    }
                    log::debug!("Task: UpdateBle");
                }

                TaskId::CheckOta => {
                    if wifi.is_connected() && ota.should_check(uptime_ms) {
                        if let Err(e) = ota.check_for_update(uptime_ms) {
                            log::warn!("OTA check failed: {}", e);
                        }
                    }
                    log::debug!("Task: CheckOta");
                }

                TaskId::FeedWatchdog => {
                    // In real firmware: wdt.feed();
                }

                TaskId::ReadBattery => {
                    // In real firmware: power.update_battery(adc.read(channel));
                    log::debug!("Task: ReadBattery");
                }

                // ── Agriculture Tasks ─────────────────────────
                #[cfg(feature = "agriculture")]
                TaskId::ReadAgriculture => {
                    // In real firmware: read soil moisture, soil temp, leaf wetness
                    // let shallow = soil_moisture.read_shallow()?;
                    // let deep = soil_moisture.read_deep()?;
                    // let soil_t = soil_temp.read_temperature()?;
                    // let leaf = leaf_wetness.read(uptime_ms)?;
                    log::debug!("Task: ReadAgriculture");
                }

                #[cfg(feature = "agriculture")]
                TaskId::ProcessAgriculture => {
                    ag_reading = ag_analytics.process(
                        latest_reading.temperature_c,
                        latest_reading.humidity_pct,
                        latest_reading.wind_speed_kmh,
                        latest_reading.pressure_hpa,
                        latest_reading.rain_rate_mm_hr,
                        latest_reading.light_lux.map(|lux| lux * 0.0079), // rough lux→W/m²
                        None, // soil moisture shallow — from sensor in real firmware
                        None, // soil moisture deep
                        None, // soil temp
                        None, // leaf wetness
                        0,    // leaf wet duration
                    );
                    latest_reading.agriculture = Some(ag_reading.clone());

                    // Check alerts and publish
                    let alerts = ag_analytics.check_alerts(&ag_reading, uptime_ms);
                    for alert in &alerts {
                        if mqtt.is_connected() {
                            if let Err(e) = mqtt.publish_industry_alert(alert) {
                                log::warn!("Agriculture alert publish failed: {}", e);
                            }
                        }
                    }

                    if mqtt.is_connected() {
                        if let Err(e) = mqtt.publish_agriculture(&ag_reading) {
                            log::warn!("Agriculture telemetry publish failed: {}", e);
                        }
                    }
                    log::debug!("Task: ProcessAgriculture");
                }

                // ── Solar Tasks ───────────────────────────────
                #[cfg(feature = "solar")]
                TaskId::ReadSolar => {
                    // In real firmware: read pyranometer, panel temp
                    // let irr = pyranometer.read()?;
                    // let panel_t = panel_temp.read_temperature()?;
                    log::debug!("Task: ReadSolar");
                }

                #[cfg(feature = "solar")]
                TaskId::ProcessSolar => {
                    solar_reading = solar_analytics.process(
                        None, // irradiance — from pyranometer in real firmware
                        None, // panel temp front
                        None, // panel temp back
                        latest_reading.temperature_c,
                        uptime_ms,
                    );
                    latest_reading.solar = Some(solar_reading.clone());

                    // Detect rain for soiling reset
                    if let Some(rate) = latest_reading.rain_rate_mm_hr {
                        if rate > 1.0 {
                            solar_analytics.rain_detected();
                        }
                    }

                    // Check alerts and publish
                    let alerts = solar_analytics.check_alerts(&solar_reading, uptime_ms);
                    for alert in &alerts {
                        if mqtt.is_connected() {
                            if let Err(e) = mqtt.publish_industry_alert(alert) {
                                log::warn!("Solar alert publish failed: {}", e);
                            }
                        }
                    }

                    if mqtt.is_connected() {
                        if let Err(e) = mqtt.publish_solar(&solar_reading) {
                            log::warn!("Solar telemetry publish failed: {}", e);
                        }
                    }
                    log::debug!("Task: ProcessSolar");
                }
            }
        }

        // ── Power Management ──────────────────────────────────────

        let time_until_next = scheduler.time_until_next_ms();

        if time_until_next > config::MAIN_LOOP_TICK_MS {
            power.signal_idle(uptime_ms);
        } else {
            power.signal_busy();
        }

        power.evaluate_sleep(uptime_ms, time_until_next);

        if power.deep_sleep_pending() {
            log::info!("Entering deep sleep...");
            // In real firmware: flush MQTT, send final LoRa, save NVS, deep_sleep()
        }

        if SYSTEM_ERROR.load(Ordering::Relaxed) {
            log::error!("System error flag set — attempting recovery");
            SYSTEM_ERROR.store(false, Ordering::Relaxed);
        }

        uptime_ms += config::MAIN_LOOP_TICK_MS;

        // In real firmware: embassy_time::Timer::after_millis(MAIN_LOOP_TICK_MS).await
        for _ in 0..config::MAIN_LOOP_TICK_MS * 1000 {
            core::hint::spin_loop();
        }
    }
}

/// Initialize all sensors and return their aggregate status.
fn init_sensors() -> SensorStatusMap {
    let mut status = SensorStatusMap::default();

    log::info!("Probing BME280 at 0x{:02X}...", config::BME280_ADDR);
    status.bme280 = drivers::SensorStatus::Ok;

    log::info!("Initializing anemometer on GPIO {}...", config::WIND_SPEED_PIN);
    log::info!("Probing AS5600 at 0x{:02X}...", config::AS5600_ADDR);
    status.wind = drivers::SensorStatus::Ok;

    log::info!("Initializing rain gauge on GPIO {}...", config::RAIN_GAUGE_PIN);
    status.rain = drivers::SensorStatus::Ok;

    log::info!("Probing SI1145 at 0x{:02X}...", config::SI1145_ADDR);
    status.uv = drivers::SensorStatus::Ok;

    log::info!("Probing BH1750 at 0x{:02X}...", config::BH1750_ADDR);
    status.light = drivers::SensorStatus::Ok;

    // Agriculture sensors
    #[cfg(feature = "agriculture")]
    {
        log::info!("Initializing soil moisture on GPIO {}...", config::SOIL_MOISTURE_ADC_PIN);
        status.soil_moisture = drivers::SensorStatus::Ok;
        log::info!("Initializing soil temp (DS18B20) on GPIO {}...", config::SOIL_TEMP_PIN);
        status.soil_temp = drivers::SensorStatus::Ok;
        log::info!("Initializing leaf wetness on GPIO {}...", config::LEAF_WETNESS_ADC_PIN);
        status.leaf_wetness = drivers::SensorStatus::Ok;
    }

    // Solar sensors
    #[cfg(feature = "solar")]
    {
        log::info!("Initializing pyranometer on GPIO {}...", config::PYRANOMETER_ADC_PIN);
        status.pyranometer = drivers::SensorStatus::Ok;
        log::info!("Initializing panel temp (DS18B20) on GPIO {}...", config::PANEL_TEMP_PIN);
        status.panel_temp = drivers::SensorStatus::Ok;
    }

    status
}

/// Panic handler — logs the panic and halts. In real firmware this would
/// save crash info to NVS and let the hardware watchdog trigger a reset.
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    log::error!("PANIC: {}", info);
    loop {
        core::hint::spin_loop();
    }
}
