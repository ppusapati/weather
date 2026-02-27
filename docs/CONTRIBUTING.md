# Contributing

## Getting Started

1. Install the Rust toolchain:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. Install the Xtensa target (ESP32-S3):
   ```bash
   cargo install espup
   espup install
   source ~/export-esp.sh
   ```

3. Clone the repository and build:
   ```bash
   git clone <repo-url>
   cd weather/firmware
   cargo build --release
   ```

## Development Workflow

1. Create a feature branch from `main`
2. Make your changes with clear, focused commits
3. Run `cargo fmt` and `cargo clippy` before committing
4. Run tests: `cargo test --target x86_64-unknown-linux-gnu`
5. Open a pull request with a description of changes

## Code Style

- Follow standard Rust conventions (enforced by `rustfmt.toml`)
- Use `clippy` with `--deny warnings`
- All public items must have doc comments
- Prefer `heapless` collections over heap allocation where possible
- Use named constants from `config.rs` instead of magic numbers
- Handle all `Result` values explicitly — never use `let _ =` to discard errors

## Commit Messages

Use conventional commit style:

```
feat: add wind gust detection to anemometer driver
fix: correct BME280 humidity compensation overflow
docs: update power budget figures in HARDWARE.md
refactor: extract HeaplessWriter to shared utils
```

## Adding a New Sensor

1. Create a driver module in `firmware/src/drivers/`
2. Add the sensor status field to `SensorStatusMap`
3. Add raw data fields to `RawSensorData` in `data_pipeline.rs`
4. Add processing logic in `DataPipeline::process()`
5. Add configuration constants to `config.rs`
6. Update documentation (HARDWARE.md, data_pipeline diagram)

## Adding a New Communication Channel

1. Create a module in `firmware/src/comms/`
2. Implement the `CommChannel` trait
3. Add a scheduler task in `scheduler.rs`
4. Handle the task in `main.rs` main loop
5. Update API documentation

## Reporting Issues

- Include firmware version, hardware revision, and reproduction steps
- Attach serial console output if available
- For sensor issues, include raw vs. calibrated values
