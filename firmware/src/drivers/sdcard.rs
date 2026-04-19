/// SD card SPI driver wrapper using `embedded-sdmmc`.
///
/// Wraps an SD card connected to SPI2 (shared bus with W5500 Ethernet).
/// Uses `embedded-hal-bus::spi::CriticalSectionDevice` for bus sharing
/// with per-device CS pins. The SD card CS is on PD10, detect on PD11.
///
/// # SPI Bus Sharing
///
/// SPI2 is shared between W5500 (ethernet feature, CS=PB12) and
/// SD card (sdcard feature, CS=PD10). Each device gets a
/// `CriticalSectionDevice` wrapper around the shared `SpiBus`,
/// ensuring exclusive access via critical sections (safe on
/// single-core STM32F407).

use crate::error::{Error, Result};

/// SD card error types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SdCardError {
    /// Card not inserted (PD11 detect pin high).
    NotInserted,
    /// SPI communication failed.
    SpiFailed,
    /// Card initialization failed (CMD0/CMD8/ACMD41).
    InitFailed,
    /// Card capacity could not be read.
    CapacityReadFailed,
    /// FAT32 volume not found.
    NoFat32Volume,
    /// File operation failed.
    FileError,
    /// Directory creation failed.
    DirError,
    /// Write buffer overflow.
    BufferOverflow,
}

impl core::fmt::Display for SdCardError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SdCardError::NotInserted => write!(f, "SD card not inserted"),
            SdCardError::SpiFailed => write!(f, "SD SPI communication failed"),
            SdCardError::InitFailed => write!(f, "SD card init failed"),
            SdCardError::CapacityReadFailed => write!(f, "SD capacity read failed"),
            SdCardError::NoFat32Volume => write!(f, "No FAT32 volume found"),
            SdCardError::FileError => write!(f, "SD file operation failed"),
            SdCardError::DirError => write!(f, "SD directory operation failed"),
            SdCardError::BufferOverflow => write!(f, "SD write buffer overflow"),
        }
    }
}

impl From<SdCardError> for Error {
    fn from(e: SdCardError) -> Self {
        match e {
            SdCardError::NotInserted => Error::SdCardNotInserted,
            SdCardError::BufferOverflow => Error::SdCardFull,
            _ => Error::SdCardInitFailed,
        }
    }
}

/// SD card driver state.
///
/// In real firmware, this wraps `embedded_sdmmc::SdCard` with a
/// `CriticalSectionDevice` SPI bus wrapper. The generic types would be:
///
/// ```ignore
/// type SharedSpi2 = CriticalSectionDevice<'static, Spi<SPI2>, OutputPin<PD10>>;
/// type SdCardInner = embedded_sdmmc::SdCard<SharedSpi2, DummyCsPin, Timer>;
/// ```
pub struct SdCardDriver {
    /// Whether a card is physically inserted (PD11 low).
    inserted: bool,
    /// Whether the card has been successfully initialized.
    initialized: bool,
    /// Card capacity in bytes (read during init).
    capacity_bytes: u64,
    /// Last error encountered.
    last_error: Option<SdCardError>,
}

impl SdCardDriver {
    /// Create a new SD card driver (call before init).
    pub fn new() -> Self {
        Self {
            inserted: false,
            initialized: false,
            capacity_bytes: 0,
            last_error: None,
        }
    }

    /// Check if an SD card is physically inserted (read PD11 detect pin).
    ///
    /// The detect pin is active-low: low = card inserted, high = no card.
    /// In real firmware this reads the GPIO pin; here we default to true
    /// for development.
    pub fn is_inserted(&self) -> bool {
        // In real firmware:
        // let detect_pin = unsafe { &*GPIOD::ptr() };
        // detect_pin.idr.read().idr11().bit_is_clear()  // active low
        self.inserted
    }

    /// Initialize the SD card.
    ///
    /// Sequence:
    /// 1. Set SPI2 clock to 400 kHz (SD init mode)
    /// 2. Send 80+ clock pulses with CS high (card power-up)
    /// 3. CMD0 (GO_IDLE_STATE) — card enters SPI mode
    /// 4. CMD8 (SEND_IF_COND) — check voltage support
    /// 5. ACMD41 (SD_SEND_OP_COND) — init card, wait for ready
    /// 6. CMD58 (READ_OCR) — read card type (SDHC/SDXC)
    /// 7. CMD9 (SEND_CSD) — read capacity
    /// 8. Increase SPI2 clock to 25 MHz (normal operation)
    pub fn init(&mut self) -> Result<()> {
        if !self.is_inserted() {
            self.last_error = Some(SdCardError::NotInserted);
            return Err(SdCardError::NotInserted.into());
        }

        // In real firmware:
        // self.spi.set_clock(config::SD_SPI_INIT_CLOCK_HZ);
        // // Send 80+ clocks with CS high
        // self.cs_pin.set_high();
        // for _ in 0..10 { self.spi.transfer(&mut [0xFF]); }
        // self.cs_pin.set_low();
        //
        // let sd = embedded_sdmmc::SdCard::new(spi_device, dummy_cs);
        // sd.init()?;
        // let size = sd.num_bytes()?;
        // self.spi.set_clock(config::SD_SPI_CLOCK_HZ);

        // Placeholder: simulate successful init with 4GB card
        self.capacity_bytes = 4 * 1024 * 1024 * 1024; // 4 GB
        self.initialized = true;
        self.last_error = None;

        log::info!(
            "SD card initialized: {} MB (CS={}, DET={})",
            self.capacity_mb(),
            crate::config::SD_CS_PIN,
            crate::config::SD_DETECT_PIN,
        );
        Ok(())
    }

    /// Get card capacity in megabytes.
    pub fn capacity_mb(&self) -> u32 {
        (self.capacity_bytes / (1024 * 1024)) as u32
    }

    /// Check if the card is initialized and ready for I/O.
    pub fn is_ready(&self) -> bool {
        self.inserted && self.initialized
    }

    /// Get the last error, if any.
    pub fn last_error(&self) -> Option<SdCardError> {
        self.last_error
    }

    /// Simulate card insertion for development/testing.
    pub fn set_inserted(&mut self, inserted: bool) {
        self.inserted = inserted;
        if !inserted {
            self.initialized = false;
            self.capacity_bytes = 0;
        }
    }
}
