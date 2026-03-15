/// STM32F407 hardware abstraction layer.
///
/// Provides MCU-specific initialization for the STM32F407VGT6 variant
/// of the weather station, which adds industrial SCADA connectivity
/// via Modbus RS485 alongside the standard cloud (MQTT/HTTP) path.
///
/// # STM32F407VGT6 — Industrial-Grade Specifications
///
/// | Parameter | Value |
/// |-----------|-------|
/// | Core | ARM Cortex-M4F, 168 MHz |
/// | Flash | 1 MB |
/// | SRAM | 192 KB (128+64) |
/// | FPU | Single-precision hardware FPU |
/// | Temp Range | -40°C to +105°C (industrial) |
/// | Package | LQFP-100 |
/// | Supply | 1.8V – 3.6V |
/// | ADC | 3× 12-bit, 2.4 MSPS |
/// | UART | 6× USART/UART |
/// | SPI | 3× SPI (up to 42 Mbps) |
/// | I2C | 3× I2C (up to 400 kHz) |
/// | Timers | 14× (2× 32-bit, 12× 16-bit) |
/// | DMA | 2× DMA controllers, 16 streams |
/// | CAN | 2× CAN 2.0B |
/// | GPIO | 82 I/O pins |
///
/// # Pin Assignment (LQFP-100)
///
/// ```text
/// ┌─────────────────────────────────────────────────────────────┐
/// │ STM32F407VGT6 Pin Assignment (Weather Station)             │
/// ├─────────────────────────────────────────────────────────────┤
/// │ I2C1 (Sensors):                                            │
/// │   PB6 ──── SCL (BME280, AS5600, SI1145, BH1750)           │
/// │   PB7 ──── SDA                                             │
/// │                                                            │
/// │ SPI1 (LoRa SX1276):                                       │
/// │   PA5 ──── SCK                                             │
/// │   PA6 ──── MISO                                            │
/// │   PA7 ──── MOSI                                            │
/// │   PA4 ──── NSS (CS)                                        │
/// │   PC4 ──── RST                                             │
/// │   PC5 ──── DIO0 (IRQ)                                      │
/// │                                                            │
/// │ USART2 (Modbus RS485):                                     │
/// │   PA2 ──── TX → MAX3485 DI                                 │
/// │   PA3 ──── RX ← MAX3485 RO                                 │
/// │   PA1 ──── DE/RE (driver enable, active high)              │
/// │                                                            │
/// │ USART1 (Debug Console):                                    │
/// │   PA9 ──── TX                                              │
/// │   PA10 ──── RX                                             │
/// │                                                            │
/// │ USART3 (ESP32-S3 Bridge):                                  │
/// │   PB10 ──── TX → ESP32 RX                                  │
/// │   PB11 ──── RX ← ESP32 TX                                  │
/// │                                                            │
/// │ ADC1 (Analog Sensors):                                     │
/// │   PA0 ──── Battery voltage (divider)                       │
/// │   PC0 ──── Wind speed (analog fallback)                    │
/// │   PC1 ──── PM2.5 sensor (GP2Y1014AU0F)                    │
/// │   PC2 ──── Soil moisture shallow                           │
/// │   PC3 ──── Soil moisture deep                              │
/// │                                                            │
/// │ GPIO (Digital):                                             │
/// │   PB0 ──── Wind speed (pulse input, TIM3_CH3)              │
/// │   PB1 ──── Rain gauge (pulse input, TIM3_CH4)              │
/// │   PD0 ──── Status LED (green)                              │
/// │   PD1 ──── Error LED (red)                                 │
/// │   PD2 ──── SCADA active LED (amber)                        │
/// │   PE0 ──── Mode select DIP switch bit 0                    │
/// │   PE1 ──── Mode select DIP switch bit 1                    │
/// │                                                            │
/// │ 1-Wire (Temp sensors):                                     │
/// │   PC6 ──── 1-Wire bus A (soil temp, panel temp front)      │
/// │   PC7 ──── 1-Wire bus B (panel temp back)                  │
/// │                                                            │
/// │ Watchdog:                                                   │
/// │   IWDG ──── Independent watchdog, 4s timeout               │
/// └─────────────────────────────────────────────────────────────┘
/// ```

use serde::{Deserialize, Serialize};

/// STM32F407 pin assignments.
pub mod pins {
    // I2C1 — Sensor bus
    pub const I2C1_SCL: u8 = 22; // PB6
    pub const I2C1_SDA: u8 = 23; // PB7

    // SPI1 — LoRa SX1276
    pub const SPI1_SCK: u8 = 5;   // PA5
    pub const SPI1_MISO: u8 = 6;  // PA6
    pub const SPI1_MOSI: u8 = 7;  // PA7
    pub const SPI1_NSS: u8 = 4;   // PA4
    pub const LORA_RST: u8 = 36;  // PC4
    pub const LORA_DIO0: u8 = 37; // PC5

    // USART2 — Modbus RS485
    pub const RS485_TX: u8 = 2;    // PA2
    pub const RS485_RX: u8 = 3;    // PA3
    pub const RS485_DE_RE: u8 = 1; // PA1 (driver enable / receiver enable)

    // USART1 — Debug console
    pub const DEBUG_TX: u8 = 9;  // PA9
    pub const DEBUG_RX: u8 = 10; // PA10

    // USART3 — ESP32-S3 bridge (for cloud connectivity)
    pub const BRIDGE_TX: u8 = 26; // PB10
    pub const BRIDGE_RX: u8 = 27; // PB11

    // ADC1 — Analog inputs
    pub const ADC_BATTERY: u8 = 0;        // PA0 (ADC1_CH0)
    pub const ADC_WIND_SPEED: u8 = 32;    // PC0 (ADC1_CH10)
    pub const ADC_PM25: u8 = 33;          // PC1 (ADC1_CH11)
    pub const ADC_SOIL_SHALLOW: u8 = 34;  // PC2 (ADC1_CH12)
    pub const ADC_SOIL_DEEP: u8 = 35;     // PC3 (ADC1_CH13)

    // GPIO — Digital I/O
    pub const WIND_PULSE: u8 = 16;    // PB0 (TIM3_CH3)
    pub const RAIN_PULSE: u8 = 17;    // PB1 (TIM3_CH4)
    pub const LED_STATUS: u8 = 48;    // PD0
    pub const LED_ERROR: u8 = 49;     // PD1
    pub const LED_SCADA: u8 = 50;     // PD2
    pub const MODE_DIP_0: u8 = 64;    // PE0
    pub const MODE_DIP_1: u8 = 65;    // PE1

    // 1-Wire buses
    pub const ONEWIRE_A: u8 = 38; // PC6
    pub const ONEWIRE_B: u8 = 39; // PC7
}

/// STM32F407 system clocks configuration.
pub mod clocks {
    /// HSE crystal frequency (Hz).
    pub const HSE_HZ: u32 = 8_000_000;
    /// System clock (HCLK) frequency (Hz).
    pub const SYSCLK_HZ: u32 = 168_000_000;
    /// APB1 clock (Hz) — USART2/3, I2C, TIM2-7.
    pub const APB1_HZ: u32 = 42_000_000;
    /// APB2 clock (Hz) — USART1, SPI1, ADC, TIM1/8.
    pub const APB2_HZ: u32 = 84_000_000;
    /// SysTick interval (ms).
    pub const SYSTICK_MS: u32 = 1;
}

/// Modbus RS485 configuration.
pub mod modbus {
    /// Default Modbus slave address (configurable via DIP switch or NVS).
    pub const DEFAULT_SLAVE_ADDR: u8 = 1;
    /// Baud rate for RS485 Modbus RTU.
    pub const BAUD_RATE: u32 = 9600;
    /// Inter-frame silence (3.5 char times at 9600 baud = ~4.06 ms).
    pub const T35_US: u32 = 4_063;
    /// Inter-character timeout (1.5 char times at 9600 baud = ~1.74 ms).
    pub const T15_US: u32 = 1_741;
    /// Maximum Modbus RTU frame size (bytes).
    pub const MAX_FRAME_SIZE: usize = 256;
    /// Maximum number of registers per read request.
    pub const MAX_REGISTERS_PER_READ: u16 = 125;
    /// RS485 driver enable settling time (µs).
    pub const DE_SETTLE_US: u32 = 10;
}

/// Operating mode for the weather station.
///
/// Selected via DIP switches on PE0/PE1, or configured via NVS.
///
/// | PE1 | PE0 | Mode |
/// |-----|-----|------|
/// | 0 | 0 | Cloud (MQTT/HTTP only) |
/// | 0 | 1 | SCADA (Modbus RS485 only) |
/// | 1 | 0 | Hybrid (both simultaneously) |
/// | 1 | 1 | Reserved (defaults to Hybrid) |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum OperatingMode {
    /// Cloud mode: MQTT + HTTP over WiFi/Ethernet.
    /// ESP32-S3 handles all cloud connectivity.
    /// STM32F407 handles sensors only and bridges data to ESP32 via USART3.
    Cloud,
    /// SCADA mode: Modbus RTU over RS485.
    /// All data exposed as Modbus holding/input registers.
    /// No cloud connectivity — fully air-gapped.
    Scada,
    /// Hybrid mode: both cloud and SCADA simultaneously.
    /// ESP32-S3 handles MQTT/HTTP, STM32F407 handles Modbus RS485.
    /// Data is served on both interfaces concurrently.
    #[default]
    Hybrid,
}

impl OperatingMode {
    /// Read operating mode from DIP switch GPIO pins.
    pub fn from_dip_switch(bit0: bool, bit1: bool) -> Self {
        match (bit1, bit0) {
            (false, false) => OperatingMode::Cloud,
            (false, true) => OperatingMode::Scada,
            (true, false) => OperatingMode::Hybrid,
            (true, true) => OperatingMode::Hybrid, // reserved → default to hybrid
        }
    }

    /// Whether this mode enables cloud connectivity (MQTT/HTTP via ESP32-S3).
    pub fn cloud_enabled(&self) -> bool {
        matches!(self, OperatingMode::Cloud | OperatingMode::Hybrid)
    }

    /// Whether this mode enables SCADA connectivity (Modbus RS485).
    pub fn scada_enabled(&self) -> bool {
        matches!(self, OperatingMode::Scada | OperatingMode::Hybrid)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            OperatingMode::Cloud => "Cloud (MQTT/HTTP)",
            OperatingMode::Scada => "SCADA (Modbus RS485)",
            OperatingMode::Hybrid => "Hybrid (MQTT/HTTP + Modbus RS485)",
        }
    }
}

/// STM32F407 hardware initialization status.
#[derive(Debug, Clone)]
pub struct Stm32Status {
    pub sysclk_mhz: u32,
    pub mode: OperatingMode,
    pub rs485_ok: bool,
    pub esp32_bridge_ok: bool,
    pub iwdg_enabled: bool,
    pub fpu_enabled: bool,
    pub dma_enabled: bool,
}

impl Default for Stm32Status {
    fn default() -> Self {
        Self {
            sysclk_mhz: clocks::SYSCLK_HZ / 1_000_000,
            mode: OperatingMode::Hybrid,
            rs485_ok: false,
            esp32_bridge_ok: false,
            iwdg_enabled: false,
            fpu_enabled: true,
            dma_enabled: false,
        }
    }
}

/// ESP32-S3 ↔ STM32F407 bridge protocol.
///
/// The two MCUs communicate via USART3 at 921600 baud using a simple
/// framed protocol:
///
/// ```text
/// ┌──────┬──────┬────────┬─────────┬───────┐
/// │ SYNC │ LEN  │  CMD   │ PAYLOAD │  CRC  │
/// │ 0xA5 │ u16  │  u8    │  var    │ u16   │
/// └──────┴──────┴────────┴─────────┴───────┘
/// ```
pub mod bridge {
    /// Sync byte for frame detection.
    pub const SYNC_BYTE: u8 = 0xA5;
    /// Bridge UART baud rate.
    pub const BAUD_RATE: u32 = 921_600;
    /// Maximum bridge frame payload size.
    pub const MAX_PAYLOAD: usize = 512;

    /// Bridge commands.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(u8)]
    pub enum BridgeCmd {
        /// STM32 → ESP32: Send weather reading for MQTT publish.
        SendReading = 0x01,
        /// STM32 → ESP32: Send alert for MQTT publish.
        SendAlert = 0x02,
        /// STM32 → ESP32: Send status update.
        SendStatus = 0x03,
        /// ESP32 → STM32: Configuration update from cloud.
        ConfigUpdate = 0x10,
        /// ESP32 → STM32: OTA trigger.
        OtaTrigger = 0x11,
        /// ESP32 → STM32: Time sync (NTP epoch).
        TimeSync = 0x12,
        /// Bidirectional: Heartbeat/keepalive.
        Heartbeat = 0xFE,
        /// Bidirectional: ACK.
        Ack = 0xFF,
    }
}
