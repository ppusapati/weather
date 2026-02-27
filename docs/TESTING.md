# Testing Guide

## Test Architecture

The firmware uses a dual testing strategy:

1. **Host-side unit tests** — Run on `x86_64`, test pure logic (CRC, filters, validation, derived values)
2. **On-target integration tests** — Run on ESP32-S3 hardware, test I2C/SPI/GPIO interactions

## Running Host Tests

```bash
cd firmware
cargo test --target x86_64-unknown-linux-gnu
```

This runs tests in:
- `utils/crc.rs` — CRC-16 computation and verification
- `tests/integration.rs` — Data pipeline, validation, derived values, filtering, battery

## Test Categories

### CRC Tests (`utils/crc.rs`)
- Empty input produces init value (0xFFFF)
- Known test vector: `"123456789"` → `0x29B1`
- Round-trip verify/reject

### Data Validation Tests (`tests/integration.rs`)
- Temperature within/outside BME280 range (-40 to 85 C)
- Humidity clamping (0-100%)
- Pressure validation (300-1100 hPa)
- NaN/infinity rejection

### Derived Value Tests
- Heat index: only computed when T >= 27 C and H >= 40%
- Dew point: Magnus formula, rejects H <= 0%
- Wind chill: only when T < 10 C and wind > 4.8 km/h

### EMA Filter Tests
- First sample passes through unfiltered
- Subsequent samples are smoothed
- Alpha = 1.0 means no filtering

### Battery Voltage Tests
- Full charge (4.2V) → 100%
- Empty (3.0V) → 0%
- Below minimum → clamped to 0%

## Adding New Tests

Add host-testable functions with `#[cfg(test)]`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_my_function() {
        assert_eq!(my_function(input), expected);
    }
}
```

For integration tests, add to `tests/integration.rs`.

## On-Target Testing

For hardware integration testing on the ESP32-S3:

1. Connect the board via USB
2. Flash test firmware: `cargo espflash flash --monitor`
3. Observe serial output for sensor readings
4. Use UART console commands:
   - `status` — Check all sensor statuses
   - `reading` — View current sensor values
   - `calibrate temp <offset>` — Verify calibration pipeline

## CI Integration

The CI pipeline runs:
1. `cargo fmt --check` — Formatting verification
2. `cargo clippy -- -D warnings` — Lint checks
3. `cargo test --target x86_64-unknown-linux-gnu` — Host tests
4. `cargo build --release` — Full release build
