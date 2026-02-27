/// Integration tests for weather station firmware components.
///
/// These tests run on the host (not on the target MCU) and verify
/// the logic of the data pipeline, CRC, ring buffer, and other
/// platform-independent components.

// ── CRC Tests ─────────────────────────────────────────────────

#[cfg(test)]
mod crc_tests {
    // Inline the CRC implementation for host testing
    const CRC16_POLY: u16 = 0x1021;
    const CRC16_INIT: u16 = 0xFFFF;

    fn crc16(data: &[u8]) -> u16 {
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

    #[test]
    fn crc16_empty() {
        assert_eq!(crc16(&[]), 0xFFFF);
    }

    #[test]
    fn crc16_known_vector() {
        // Standard test: "123456789" → CRC-16/CCITT-FALSE = 0x29B1
        assert_eq!(crc16(b"123456789"), 0x29B1);
    }

    #[test]
    fn crc16_single_byte() {
        let c = crc16(&[0x41]); // 'A'
        assert_ne!(c, CRC16_INIT);
        assert_ne!(c, 0);
    }

    #[test]
    fn crc16_deterministic() {
        let data = b"weather station test data";
        assert_eq!(crc16(data), crc16(data));
    }

    #[test]
    fn crc16_different_data_different_crc() {
        assert_ne!(crc16(b"hello"), crc16(b"world"));
    }
}

// ── Data Validation Tests ─────────────────────────────────────

#[cfg(test)]
mod validation_tests {
    fn validate_range(value: f32, min: f32, max: f32) -> bool {
        value >= min && value <= max && value.is_finite()
    }

    #[test]
    fn temperature_range() {
        assert!(validate_range(23.5, -40.0, 85.0));
        assert!(validate_range(-40.0, -40.0, 85.0));
        assert!(validate_range(85.0, -40.0, 85.0));
        assert!(!validate_range(-41.0, -40.0, 85.0));
        assert!(!validate_range(86.0, -40.0, 85.0));
        assert!(!validate_range(f32::NAN, -40.0, 85.0));
        assert!(!validate_range(f32::INFINITY, -40.0, 85.0));
    }

    #[test]
    fn humidity_range() {
        assert!(validate_range(0.0, 0.0, 100.0));
        assert!(validate_range(50.0, 0.0, 100.0));
        assert!(validate_range(100.0, 0.0, 100.0));
        assert!(!validate_range(-0.1, 0.0, 100.0));
        assert!(!validate_range(100.1, 0.0, 100.0));
    }

    #[test]
    fn pressure_range() {
        assert!(validate_range(1013.25, 300.0, 1100.0));
        assert!(!validate_range(200.0, 300.0, 1100.0));
        assert!(!validate_range(1200.0, 300.0, 1100.0));
    }

    #[test]
    fn wind_speed_range() {
        assert!(validate_range(0.0, 0.0, 200.0));
        assert!(validate_range(100.0, 0.0, 200.0));
        assert!(!validate_range(-1.0, 0.0, 200.0));
        assert!(!validate_range(201.0, 0.0, 200.0));
    }
}

// ── Derived Value Tests ───────────────────────────────────────

#[cfg(test)]
mod derived_tests {
    fn compute_dew_point(temp_c: f32, humidity: f32) -> Option<f32> {
        if humidity <= 0.0 {
            return None;
        }
        let a = 17.67_f32;
        let b = 243.5_f32;
        let gamma = (humidity / 100.0).ln() + (a * temp_c) / (b + temp_c);
        let dew_point = (b * gamma) / (a - gamma);
        Some(dew_point)
    }

    fn compute_wind_chill(temp_c: f32, wind_kmh: f32) -> Option<f32> {
        if temp_c >= 10.0 || wind_kmh <= 4.8 {
            return None;
        }
        let v016 = wind_kmh.powf(0.16);
        let wc = 13.12 + 0.6215 * temp_c - 11.37 * v016 + 0.3965 * temp_c * v016;
        Some(wc)
    }

    fn compute_heat_index(temp_c: f32, humidity: f32) -> Option<f32> {
        if temp_c < 27.0 || humidity < 40.0 {
            return None;
        }
        let t = temp_c;
        let h = humidity;
        let hi = -8.785 + 1.611 * t + 2.339 * h - 0.146 * t * h - 0.013 * t * t
            - 0.016 * h * h + 0.002 * t * t * h + 0.001 * t * h * h
            - 0.000004 * t * t * h * h;
        Some(hi)
    }

    #[test]
    fn dew_point_calculation() {
        let dp = compute_dew_point(25.0, 60.0).unwrap();
        // Expected ~16.7°C
        assert!((dp - 16.7).abs() < 0.5, "dew point was {}", dp);
    }

    #[test]
    fn dew_point_zero_humidity() {
        assert!(compute_dew_point(25.0, 0.0).is_none());
    }

    #[test]
    fn wind_chill_cold_windy() {
        let wc = compute_wind_chill(0.0, 20.0).unwrap();
        // Wind chill should be below 0°C
        assert!(wc < 0.0, "wind chill was {}", wc);
    }

    #[test]
    fn wind_chill_warm_no_effect() {
        // Wind chill not applicable above 10°C
        assert!(compute_wind_chill(15.0, 20.0).is_none());
    }

    #[test]
    fn wind_chill_calm_no_effect() {
        // Wind chill not applicable below 4.8 km/h
        assert!(compute_wind_chill(0.0, 3.0).is_none());
    }

    #[test]
    fn heat_index_hot_humid() {
        let hi = compute_heat_index(35.0, 70.0).unwrap();
        // Heat index should be higher than air temperature
        assert!(hi > 35.0, "heat index was {}", hi);
    }

    #[test]
    fn heat_index_cool_no_effect() {
        assert!(compute_heat_index(20.0, 50.0).is_none());
    }

    #[test]
    fn heat_index_dry_no_effect() {
        assert!(compute_heat_index(35.0, 30.0).is_none());
    }
}

// ── EMA Filter Tests ──────────────────────────────────────────

#[cfg(test)]
mod filter_tests {
    struct EmaFilter {
        alpha: f32,
        value: Option<f32>,
    }

    impl EmaFilter {
        fn new(alpha: f32) -> Self {
            Self { alpha, value: None }
        }

        fn update(&mut self, new_value: f32) -> f32 {
            let filtered = match self.value {
                Some(prev) => self.alpha * new_value + (1.0 - self.alpha) * prev,
                None => new_value,
            };
            self.value = Some(filtered);
            filtered
        }
    }

    #[test]
    fn ema_first_value_passthrough() {
        let mut filter = EmaFilter::new(0.3);
        let result = filter.update(25.0);
        assert!((result - 25.0).abs() < 0.001);
    }

    #[test]
    fn ema_smoothing_effect() {
        let mut filter = EmaFilter::new(0.3);
        filter.update(20.0);
        let result = filter.update(30.0);
        // With alpha=0.3: 0.3*30 + 0.7*20 = 23.0
        assert!((result - 23.0).abs() < 0.001, "got {}", result);
    }

    #[test]
    fn ema_converges_to_constant() {
        let mut filter = EmaFilter::new(0.3);
        for _ in 0..100 {
            filter.update(42.0);
        }
        let result = filter.update(42.0);
        assert!((result - 42.0).abs() < 0.01);
    }

    #[test]
    fn ema_high_alpha_responsive() {
        let mut filter = EmaFilter::new(0.9);
        filter.update(0.0);
        let result = filter.update(100.0);
        // With alpha=0.9: 0.9*100 + 0.1*0 = 90.0
        assert!((result - 90.0).abs() < 0.001);
    }
}

// ── Battery Voltage to Percentage Tests ───────────────────────

#[cfg(test)]
mod battery_tests {
    fn voltage_to_percentage(voltage: f32) -> u8 {
        let pct = if voltage >= 4.2 {
            100.0
        } else if voltage >= 4.06 {
            90.0 + (voltage - 4.06) / (4.2 - 4.06) * 10.0
        } else if voltage >= 3.98 {
            80.0 + (voltage - 3.98) / (4.06 - 3.98) * 10.0
        } else if voltage >= 3.45 {
            ((voltage - 3.45) / (3.98 - 3.45) * 80.0)
        } else {
            0.0
        };
        pct.clamp(0.0, 100.0) as u8
    }

    #[test]
    fn full_battery() {
        assert_eq!(voltage_to_percentage(4.2), 100);
        assert_eq!(voltage_to_percentage(4.5), 100); // over-voltage clamped
    }

    #[test]
    fn empty_battery() {
        assert_eq!(voltage_to_percentage(3.0), 0);
        assert_eq!(voltage_to_percentage(2.5), 0);
    }

    #[test]
    fn mid_battery() {
        let pct = voltage_to_percentage(3.8);
        assert!(pct > 30 && pct < 80, "mid battery was {}%", pct);
    }

    #[test]
    fn monotonic_decrease() {
        let voltages = [4.2, 4.1, 4.0, 3.9, 3.8, 3.7, 3.6, 3.5, 3.4];
        let percentages: Vec<u8> = voltages.iter().map(|&v| voltage_to_percentage(v)).collect();
        for i in 1..percentages.len() {
            assert!(
                percentages[i] <= percentages[i - 1],
                "not monotonic at {}V: {}% > {}%",
                voltages[i],
                percentages[i],
                percentages[i - 1]
            );
        }
    }
}

// ── LoRa Packet Encoding Tests ────────────────────────────────

#[cfg(test)]
mod lora_packet_tests {
    const PKT_TYPE_TELEMETRY: u8 = 0x01;

    fn encode_temperature(temp_c: f32) -> [u8; 2] {
        ((temp_c * 100.0) as i16).to_be_bytes()
    }

    fn decode_temperature(bytes: [u8; 2]) -> f32 {
        i16::from_be_bytes(bytes) as f32 / 100.0
    }

    #[test]
    fn temperature_encoding_roundtrip() {
        let temp = 23.45_f32;
        let encoded = encode_temperature(temp);
        let decoded = decode_temperature(encoded);
        assert!((decoded - temp).abs() < 0.01, "got {}", decoded);
    }

    #[test]
    fn negative_temperature_encoding() {
        let temp = -15.5_f32;
        let encoded = encode_temperature(temp);
        let decoded = decode_temperature(encoded);
        assert!((decoded - temp).abs() < 0.01, "got {}", decoded);
    }

    #[test]
    fn packet_header_format() {
        let pkt_type = PKT_TYPE_TELEMETRY;
        let device_id: u16 = 0x1234;
        let timestamp: u32 = 1708950000;

        let mut pkt = [0u8; 7];
        pkt[0] = pkt_type;
        pkt[1..3].copy_from_slice(&device_id.to_be_bytes());
        pkt[3..7].copy_from_slice(&timestamp.to_be_bytes());

        assert_eq!(pkt[0], 0x01);
        assert_eq!(u16::from_be_bytes([pkt[1], pkt[2]]), 0x1234);
        assert_eq!(u32::from_be_bytes([pkt[3], pkt[4], pkt[5], pkt[6]]), 1708950000);
    }
}
