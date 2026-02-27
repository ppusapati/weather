/// CRC-16/CCITT-FALSE implementation for data integrity checks.
///
/// Used in:
/// - LoRa packet verification
/// - Flash storage record validation

const CRC16_POLY: u16 = 0x1021;
const CRC16_INIT: u16 = 0xFFFF;

/// Compute CRC-16/CCITT-FALSE over a byte slice.
pub fn crc16(data: &[u8]) -> u16 {
    let mut crc = CRC16_INIT;

    for &byte in data {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            if crc & 0x8000 != 0 {
                crc = (crc << 1) ^ CRC16_POLY;
            } else {
                crc <<= 1;
            }
        }
    }

    crc
}

/// Verify CRC-16 of a data block against an expected value.
pub fn verify_crc16(data: &[u8], expected: u16) -> bool {
    crc16(data) == expected
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crc16_empty() {
        assert_eq!(crc16(&[]), 0xFFFF);
    }

    #[test]
    fn test_crc16_known_value() {
        // "123456789" → CRC-16/CCITT-FALSE = 0x29B1
        let data = b"123456789";
        assert_eq!(crc16(data), 0x29B1);
    }

    #[test]
    fn test_crc16_verify() {
        let data = b"hello";
        let crc = crc16(data);
        assert!(verify_crc16(data, crc));
        assert!(!verify_crc16(data, crc.wrapping_add(1)));
    }

    #[test]
    fn test_crc16_single_byte() {
        let crc = crc16(&[0x00]);
        assert_ne!(crc, 0xFFFF); // should differ from init
    }
}
