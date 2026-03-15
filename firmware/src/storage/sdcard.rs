/// FAT32 CSV data logger for SD card storage.
///
/// Writes weather readings as CSV files with daily rotation:
/// ```text
/// /weather/
///   2026-03-15.csv    ← current day
///   2026-03-14.csv    ← previous day
///   ...
/// ```
///
/// CSV format (one header line + data rows):
/// ```text
/// timestamp_ms,temp_c,humidity_pct,pressure_hpa,wind_kmh,wind_deg,rain_mm,uv,lux
/// 1710504000000,23.5,65.2,1013.25,12.3,225,0.4,3.2,45000
/// ```
///
/// Buffering: rows accumulate in a 512-byte buffer (one SD sector),
/// flushed when full or on the `FlushSdCard` scheduler task (every 60s).

use crate::core::data_pipeline::WeatherReading;
use crate::drivers::sdcard::{SdCardDriver, SdCardError};
use crate::error::{Error, Result};

/// CSV header written at the start of each daily log file.
const CSV_HEADER: &[u8] = b"timestamp_ms,temp_c,humidity_pct,pressure_hpa,wind_kmh,wind_deg,rain_mm,uv,lux\n";

/// SD card FAT32 CSV logger.
pub struct SdCardStorage {
    /// Current date string (YYYY-MM-DD) for file rotation.
    current_date: heapless::String<10>,
    /// Number of records written today.
    records_today: u32,
    /// Total records written across all sessions.
    total_records: u64,
    /// Write buffer — accumulates CSV rows until one sector (512 bytes).
    write_buffer: heapless::Vec<u8, 512>,
    /// Whether the buffer has data that needs flushing.
    flush_pending: bool,
    /// Whether the storage has been initialized.
    initialized: bool,
}

impl SdCardStorage {
    /// Create a new SD card storage instance.
    pub fn new() -> Self {
        Self {
            current_date: heapless::String::new(),
            records_today: 0,
            total_records: 0,
            write_buffer: heapless::Vec::new(),
            flush_pending: false,
            initialized: false,
        }
    }

    /// Initialize SD card storage.
    ///
    /// Opens (or creates) the `/weather/` directory on the FAT32 volume.
    /// If a file for today already exists, appends to it; otherwise creates
    /// a new file with the CSV header.
    pub fn init(&mut self, driver: &mut SdCardDriver) -> Result<()> {
        if !driver.is_ready() {
            return Err(SdCardError::InitFailed.into());
        }

        // In real firmware:
        // let volume_mgr = VolumeManager::new(sd_card, time_source);
        // let volume = volume_mgr.open_volume(VolumeIdx(0))?;
        // let root_dir = volume.open_root_dir()?;
        // root_dir.make_dir_if_not_exists("weather")?;

        self.initialized = true;
        self.write_buffer.clear();
        self.flush_pending = false;

        log::info!(
            "SD card storage initialized (dir: /{}/, capacity: {} MB)",
            crate::config::SD_LOG_DIR,
            driver.capacity_mb(),
        );
        Ok(())
    }

    /// Store a weather reading to the SD card buffer.
    ///
    /// Formats the reading as a CSV row and appends it to the write buffer.
    /// When the buffer is full (≥512 bytes), it is auto-flushed.
    pub fn store(&mut self, reading: &WeatherReading) -> Result<()> {
        if !self.initialized {
            return Err(Error::SdCardInitFailed);
        }

        // Format CSV row
        let mut row_buf: heapless::Vec<u8, 128> = heapless::Vec::new();
        let row = format_csv_row(reading);

        for byte in row.as_bytes() {
            let _ = row_buf.push(*byte);
        }

        // Check if buffer has room
        if self.write_buffer.len() + row_buf.len() > 512 {
            // Buffer full — mark for flush (caller should flush before next store)
            self.flush_pending = true;
        }

        // Append to buffer
        for byte in &row_buf {
            if self.write_buffer.push(*byte).is_err() {
                self.flush_pending = true;
                return Err(SdCardError::BufferOverflow.into());
            }
        }

        self.records_today += 1;
        self.total_records += 1;
        self.flush_pending = true;

        Ok(())
    }

    /// Flush the write buffer to the SD card.
    ///
    /// Writes accumulated CSV rows to the current day's file.
    /// Called by the `FlushSdCard` scheduler task every 60s,
    /// or when the buffer is full.
    pub fn flush(&mut self, _driver: &mut SdCardDriver) -> Result<()> {
        if !self.initialized || !self.flush_pending {
            return Ok(());
        }

        if self.write_buffer.is_empty() {
            self.flush_pending = false;
            return Ok(());
        }

        // In real firmware:
        // let volume_mgr = VolumeManager::new(sd_card, time_source);
        // let volume = volume_mgr.open_volume(VolumeIdx(0))?;
        // let root_dir = volume.open_root_dir()?;
        // let weather_dir = root_dir.open_dir("weather")?;
        //
        // // Open or create today's file
        // let filename = format!("{}.csv", self.current_date);
        // let file = weather_dir.open_file_in_dir(
        //     &filename,
        //     Mode::ReadWriteCreateOrAppend,
        // )?;
        //
        // // Write header if file is new (size == 0)
        // if file.length() == 0 {
        //     file.write(CSV_HEADER)?;
        // }
        //
        // // Write buffered data
        // file.write(&self.write_buffer)?;
        // file.close()?;

        let bytes_flushed = self.write_buffer.len();
        self.write_buffer.clear();
        self.flush_pending = false;

        log::debug!(
            "SD card flushed {} bytes ({} records today, {} total)",
            bytes_flushed,
            self.records_today,
            self.total_records,
        );

        Ok(())
    }

    /// Set the current date for file rotation.
    ///
    /// When the date changes, the next write goes to a new daily file.
    /// In real firmware this is called from an RTC timestamp.
    pub fn set_date(&mut self, date: &str) {
        if self.current_date.as_str() != date {
            log::info!("SD card: rotating to new daily file {}.csv", date);
            self.records_today = 0;
            self.current_date.clear();
            let _ = self.current_date.push_str(date);
        }
    }

    /// Check if a flush is pending.
    pub fn flush_pending(&self) -> bool {
        self.flush_pending
    }

    /// Get the number of records written today.
    pub fn records_today(&self) -> u32 {
        self.records_today
    }

    /// Get the total number of records written.
    pub fn record_count(&self) -> u64 {
        self.total_records
    }

    /// Get approximate SD card usage percentage.
    ///
    /// In real firmware, this reads the FAT32 free cluster count.
    /// Returns 0 as a placeholder.
    pub fn usage_pct(&self) -> u8 {
        // In real firmware:
        // let free = volume.free_clusters_count()? * bytes_per_cluster;
        // let total = volume.total_clusters_count()? * bytes_per_cluster;
        // ((total - free) * 100 / total) as u8
        0
    }
}

/// Write an optional f32 field as CSV.
fn write_opt_f32(s: &mut heapless::String<128>, val: Option<f32>, decimals: u8) {
    use core::fmt::Write;
    match val {
        Some(v) => match decimals {
            0 => { let _ = write!(s, ",{:.0}", v); }
            1 => { let _ = write!(s, ",{:.1}", v); }
            _ => { let _ = write!(s, ",{:.2}", v); }
        },
        None => { let _ = write!(s, ","); }
    }
}

/// Format a weather reading as a CSV row.
fn format_csv_row(reading: &WeatherReading) -> heapless::String<128> {
    use core::fmt::Write;

    let mut s = heapless::String::<128>::new();

    // timestamp_ms
    let _ = write!(s, "{}", reading.timestamp_ms);

    // temp_c, humidity_pct, pressure_hpa, wind_kmh
    write_opt_f32(&mut s, reading.temperature_c, 1);
    write_opt_f32(&mut s, reading.humidity_pct, 1);
    write_opt_f32(&mut s, reading.pressure_hpa, 2);
    write_opt_f32(&mut s, reading.wind_speed_kmh, 1);

    // wind_deg (Option<u16>)
    if let Some(dir) = reading.wind_dir_deg {
        let _ = write!(s, ",{}", dir);
    } else {
        let _ = write!(s, ",");
    }

    // rain_mm (f32, not Option)
    let _ = write!(s, ",{:.1}", reading.rain_mm);

    // uv, lux
    write_opt_f32(&mut s, reading.uv_index, 1);
    write_opt_f32(&mut s, reading.light_lux, 0);

    let _ = write!(s, "\n");
    s
}
