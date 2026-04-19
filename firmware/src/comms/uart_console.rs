/// UART serial console for debug output and interactive commands.
///
/// Supports structured log output and a simple command parser.
/// Runs at 115200 baud, 8N1.

use crate::config;
use crate::core::data_pipeline::WeatherReading;
use crate::drivers::SensorStatusMap;
use crate::error::Result;
use heapless::String;

/// Maximum command line length.
const MAX_CMD_LEN: usize = 128;

/// UART console command.
#[derive(Debug, Clone)]
pub enum ConsoleCommand {
    /// Print device status.
    Status,
    /// Print latest sensor readings.
    Reading,
    /// Get a config value.
    ConfigGet { key: String<32> },
    /// Set a config value.
    ConfigSet { key: String<32>, value: String<32> },
    /// Run calibration for a sensor.
    Calibrate { sensor: String<16> },
    /// Soft reset.
    Reset,
    /// Factory reset (erase NVS).
    FactoryReset,
    /// Dump raw sensor data.
    Dump { sensor: String<16>, count: u16 },
    /// Manual LoRa transmit.
    LoraSend { message: String<64> },
    /// Scan WiFi networks.
    WifiScan,
    /// Show help text.
    Help,
    /// Unknown command.
    Unknown(String<64>),
}

/// UART console handler.
pub struct UartConsole {
    rx_buffer: [u8; MAX_CMD_LEN],
    rx_pos: usize,
}

impl UartConsole {
    pub fn new() -> Self {
        Self {
            rx_buffer: [0u8; MAX_CMD_LEN],
            rx_pos: 0,
        }
    }

    /// Initialize UART console.
    pub fn init(&self) {
        log::info!(
            "UART console ready ({}N1, {} baud)",
            8,
            config::UART_BAUD_RATE
        );
        self.print_banner();
    }

    fn print_banner(&self) {
        log::info!("========================================");
        log::info!("  Weather Station Firmware v{}", config::FIRMWARE_VERSION);
        log::info!("  STM32F407 | Rust Embedded");
        log::info!("  Type 'help' for available commands");
        log::info!("========================================");
    }

    /// Feed a received byte into the console. Returns a command when a complete line is received.
    pub fn feed_byte(&mut self, byte: u8) -> Option<ConsoleCommand> {
        match byte {
            b'\r' | b'\n' => {
                if self.rx_pos > 0 {
                    let line = core::str::from_utf8(&self.rx_buffer[..self.rx_pos])
                        .unwrap_or("")
                        .trim();
                    let cmd = self.parse_command(line);
                    self.rx_pos = 0;
                    Some(cmd)
                } else {
                    None
                }
            }
            b'\x7f' | b'\x08' => {
                // Backspace
                if self.rx_pos > 0 {
                    self.rx_pos -= 1;
                }
                None
            }
            _ => {
                if self.rx_pos < MAX_CMD_LEN - 1 {
                    self.rx_buffer[self.rx_pos] = byte;
                    self.rx_pos += 1;
                }
                None
            }
        }
    }

    /// Parse a command line into a ConsoleCommand.
    fn parse_command(&self, line: &str) -> ConsoleCommand {
        let mut parts = line.split_whitespace();
        let cmd = parts.next().unwrap_or("");

        match cmd {
            "status" => ConsoleCommand::Status,
            "reading" => ConsoleCommand::Reading,
            "config" => {
                let subcmd = parts.next().unwrap_or("");
                match subcmd {
                    "get" => {
                        let key = parts.next().unwrap_or("");
                        ConsoleCommand::ConfigGet {
                            key: String::try_from(key).unwrap_or_default(),
                        }
                    }
                    "set" => {
                        let key = parts.next().unwrap_or("");
                        let value = parts.next().unwrap_or("");
                        ConsoleCommand::ConfigSet {
                            key: String::try_from(key).unwrap_or_default(),
                            value: String::try_from(value).unwrap_or_default(),
                        }
                    }
                    _ => ConsoleCommand::Unknown(String::try_from(line).unwrap_or_default()),
                }
            }
            "calibrate" => {
                let sensor = parts.next().unwrap_or("all");
                ConsoleCommand::Calibrate {
                    sensor: String::try_from(sensor).unwrap_or_default(),
                }
            }
            "reset" => ConsoleCommand::Reset,
            "factory-reset" => ConsoleCommand::FactoryReset,
            "dump" => {
                let sensor = parts.next().unwrap_or("bme280");
                let count: u16 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(10);
                ConsoleCommand::Dump {
                    sensor: String::try_from(sensor).unwrap_or_default(),
                    count,
                }
            }
            "lora" => {
                if parts.next() == Some("send") {
                    let msg: String<64> = parts
                        .collect::<heapless::Vec<&str, 8>>()
                        .iter()
                        .fold(String::new(), |mut acc, s| {
                            if !acc.is_empty() {
                                let _ = acc.push(' ');
                            }
                            let _ = acc.push_str(s);
                            acc
                        });
                    ConsoleCommand::LoraSend { message: msg }
                } else {
                    ConsoleCommand::Unknown(String::try_from(line).unwrap_or_default())
                }
            }
            "wifi" => {
                if parts.next() == Some("scan") {
                    ConsoleCommand::WifiScan
                } else {
                    ConsoleCommand::Unknown(String::try_from(line).unwrap_or_default())
                }
            }
            "help" => ConsoleCommand::Help,
            _ => ConsoleCommand::Unknown(String::try_from(line).unwrap_or_default()),
        }
    }

    /// Format and print a weather reading to UART.
    pub fn print_reading(reading: &WeatherReading) {
        log::info!("--- Sensor Readings ---");
        if let Some(t) = reading.temperature_c {
            log::info!("  Temperature:    {:.1} C", t);
        }
        if let Some(h) = reading.humidity_pct {
            log::info!("  Humidity:       {:.1} %", h);
        }
        if let Some(p) = reading.pressure_hpa {
            log::info!("  Pressure:       {:.1} hPa", p);
        }
        if let Some(w) = reading.wind_speed_kmh {
            log::info!("  Wind Speed:     {:.1} km/h", w);
        }
        if let Some(d) = reading.wind_dir_deg {
            log::info!("  Wind Direction: {} deg", d);
        }
        log::info!("  Rain Total:     {:.1} mm", reading.rain_mm);
        if let Some(r) = reading.rain_rate_mm_hr {
            log::info!("  Rain Rate:      {:.1} mm/hr", r);
        }
        if let Some(u) = reading.uv_index {
            log::info!("  UV Index:       {:.1}", u);
        }
        if let Some(l) = reading.light_lux {
            log::info!("  Light:          {:.0} lux", l);
        }
        if let Some(hi) = reading.heat_index_c {
            log::info!("  Heat Index:     {:.1} C", hi);
        }
        if let Some(dp) = reading.dew_point_c {
            log::info!("  Dew Point:      {:.1} C", dp);
        }
        if let Some(wc) = reading.wind_chill_c {
            log::info!("  Wind Chill:     {:.1} C", wc);
        }
        log::info!("-----------------------");
    }

    /// Print help text.
    pub fn print_help() {
        log::info!("Available commands:");
        log::info!("  status            - Print device status");
        log::info!("  reading           - Print latest readings");
        log::info!("  config get <key>  - Read config value");
        log::info!("  config set <k> <v>- Write config value");
        log::info!("  calibrate <sensor>- Run calibration");
        log::info!("  reset             - Soft reset");
        log::info!("  factory-reset     - Erase NVS + reset");
        log::info!("  dump <sensor> <n> - Raw data dump");
        log::info!("  lora send <msg>   - Manual LoRa transmit");
        log::info!("  wifi scan         - Scan WiFi networks");
        log::info!("  help              - Show this help");
    }

    /// Print sensor status.
    pub fn print_status(status: &SensorStatusMap, uptime_s: u64) {
        log::info!("--- Device Status ---");
        log::info!("  Uptime: {}s", uptime_s);
        log::info!("  FW:     v{}", config::FIRMWARE_VERSION);
        log::info!("  BME280: {}", status.bme280.as_str());
        log::info!("  Wind:   {}", status.wind.as_str());
        log::info!("  Rain:   {}", status.rain.as_str());
        log::info!("  UV:     {}", status.uv.as_str());
        log::info!("  Light:  {}", status.light.as_str());
        log::info!("---------------------");
    }
}
