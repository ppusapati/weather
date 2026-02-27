/// Flash storage driver — circular buffer for weather readings.
///
/// Uses a dedicated flash partition (~7.9 MB) to store readings
/// as a circular log. When the storage is full, the oldest records
/// are overwritten.

use crate::config;
use crate::core::data_pipeline::WeatherReading;
use crate::error::{Error, Result};
use crate::utils::crc::crc16;

/// Record header stored before each reading in flash.
#[repr(C, packed)]
#[derive(Clone, Copy)]
struct RecordHeader {
    /// Magic number to identify valid records.
    magic: u16,
    /// Record sequence number.
    sequence: u32,
    /// Data length in bytes.
    data_len: u16,
    /// CRC16 of the data payload.
    crc: u16,
}

const RECORD_MAGIC: u16 = 0xBEEF;
const HEADER_SIZE: usize = core::mem::size_of::<RecordHeader>();

/// Flash-backed circular buffer for weather readings.
pub struct FlashStorage {
    /// Start address of the data partition.
    base_addr: u32,
    /// Total size of the data partition in bytes.
    partition_size: u32,
    /// Size of each record slot (header + max payload).
    record_slot_size: usize,
    /// Maximum number of records that fit.
    max_records: u32,
    /// Index of the next write position.
    write_index: u32,
    /// Number of records currently stored.
    count: u32,
    /// Monotonically increasing sequence counter.
    sequence: u32,
}

impl FlashStorage {
    pub fn new() -> Self {
        let record_slot_size = config::FLASH_RECORD_SIZE;
        let max_records = config::FLASH_DATA_SIZE / record_slot_size as u32;

        Self {
            base_addr: config::FLASH_DATA_START,
            partition_size: config::FLASH_DATA_SIZE,
            record_slot_size,
            max_records,
            write_index: 0,
            count: 0,
            sequence: 0,
        }
    }

    /// Initialize storage: scan for the latest write position.
    pub fn init(&mut self) -> Result<()> {
        log::info!(
            "Flash: initializing storage at 0x{:08X}, {} slots of {} bytes",
            self.base_addr,
            self.max_records,
            self.record_slot_size
        );

        // In real firmware: scan flash for the record with the highest
        // sequence number to find the current write position.
        // For now, start fresh.
        self.write_index = 0;
        self.count = 0;
        self.sequence = 0;

        log::info!("Flash: storage ready, {} records capacity", self.max_records);
        Ok(())
    }

    /// Store a weather reading to flash.
    pub fn store(&mut self, reading: &WeatherReading) -> Result<()> {
        // Serialize the reading to a compact binary format
        let mut payload = [0u8; 48];
        let payload_len = self.serialize_reading(reading, &mut payload);

        // Compute CRC
        let crc = crc16(&payload[..payload_len]);

        let header = RecordHeader {
            magic: RECORD_MAGIC,
            sequence: self.sequence,
            data_len: payload_len as u16,
            crc,
        };

        // Calculate flash address
        let addr = self.base_addr + (self.write_index * self.record_slot_size as u32);

        // In real firmware:
        // 1. Erase flash sector if needed (4KB sectors)
        // 2. Write header
        // 3. Write payload
        // esp_storage::FlashStorage::write(addr, &header_bytes)?;
        // esp_storage::FlashStorage::write(addr + HEADER_SIZE, &payload)?;

        log::debug!(
            "Flash: stored record #{} at 0x{:08X} ({} bytes)",
            self.sequence,
            addr,
            HEADER_SIZE + payload_len
        );

        // Advance write pointer (circular)
        self.write_index = (self.write_index + 1) % self.max_records;
        self.sequence += 1;
        if self.count < self.max_records {
            self.count += 1;
        }

        Ok(())
    }

    /// Read the most recent N records from flash.
    pub fn read_recent(&self, count: u32) -> Result<heapless::Vec<WeatherReading, 64>> {
        let mut readings = heapless::Vec::new();
        let actual_count = count.min(self.count);

        for i in 0..actual_count {
            let idx = if self.write_index >= i + 1 {
                self.write_index - i - 1
            } else {
                self.max_records - (i + 1 - self.write_index)
            };

            let _addr = self.base_addr + (idx * self.record_slot_size as u32);

            // In real firmware:
            // 1. Read header from flash
            // 2. Verify magic and CRC
            // 3. Read and deserialize payload
            // 4. Push to readings vector

            // Placeholder: push a default reading
            let _ = readings.push(WeatherReading::default());
        }

        Ok(readings)
    }

    /// Get the number of stored records.
    pub fn record_count(&self) -> u32 {
        self.count
    }

    /// Get storage usage as a percentage.
    pub fn usage_pct(&self) -> u8 {
        if self.max_records == 0 {
            return 0;
        }
        ((self.count as u64 * 100) / self.max_records as u64) as u8
    }

    /// Erase all stored data.
    pub fn erase_all(&mut self) -> Result<()> {
        log::warn!("Flash: erasing all stored data");

        // In real firmware: erase the entire data partition
        // esp_storage::FlashStorage::erase(self.base_addr, self.partition_size)?;

        self.write_index = 0;
        self.count = 0;
        self.sequence = 0;

        Ok(())
    }

    /// Serialize a WeatherReading to a compact binary format.
    fn serialize_reading(&self, reading: &WeatherReading, buf: &mut [u8]) -> usize {
        let mut pos = 0;

        // Timestamp (8 bytes)
        buf[pos..pos + 8].copy_from_slice(&reading.timestamp_ms.to_le_bytes());
        pos += 8;

        // Temperature (4 bytes, f32)
        let temp = reading.temperature_c.unwrap_or(f32::NAN);
        buf[pos..pos + 4].copy_from_slice(&temp.to_le_bytes());
        pos += 4;

        // Humidity (4 bytes, f32)
        let hum = reading.humidity_pct.unwrap_or(f32::NAN);
        buf[pos..pos + 4].copy_from_slice(&hum.to_le_bytes());
        pos += 4;

        // Pressure (4 bytes, f32)
        let press = reading.pressure_hpa.unwrap_or(f32::NAN);
        buf[pos..pos + 4].copy_from_slice(&press.to_le_bytes());
        pos += 4;

        // Wind speed (4 bytes, f32)
        let wind = reading.wind_speed_kmh.unwrap_or(f32::NAN);
        buf[pos..pos + 4].copy_from_slice(&wind.to_le_bytes());
        pos += 4;

        // Wind direction (2 bytes, u16)
        let wdir = reading.wind_dir_deg.unwrap_or(0);
        buf[pos..pos + 2].copy_from_slice(&wdir.to_le_bytes());
        pos += 2;

        // Rain total (4 bytes, f32)
        buf[pos..pos + 4].copy_from_slice(&reading.rain_mm.to_le_bytes());
        pos += 4;

        // UV index (4 bytes, f32)
        let uv = reading.uv_index.unwrap_or(f32::NAN);
        buf[pos..pos + 4].copy_from_slice(&uv.to_le_bytes());
        pos += 4;

        // Light (4 bytes, f32)
        let light = reading.light_lux.unwrap_or(f32::NAN);
        buf[pos..pos + 4].copy_from_slice(&light.to_le_bytes());
        pos += 4;

        pos // 42 bytes total
    }
}
