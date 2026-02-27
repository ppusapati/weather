/// Shared utility modules.
///
/// Contains platform-independent helpers used across the firmware:
/// - `crc` — CRC-16/CCITT-FALSE for data integrity
/// - `ring_buffer` — Lock-free SPSC ring buffer for inter-core communication
/// - `fmt` — Shared formatting helpers for `heapless::String`

pub mod crc;
pub mod fmt;
pub mod ring_buffer;
