# CLAUDE.md

> This is a living document. Update it when you learn new preferences, patterns, or project conventions. Don't ask, just update it if something is missing or outdated.

## Project Overview

8-channel NiMH battery capacity tester running on ESP32-C3 (RISC-V). Bare-metal Rust firmware using `esp-hal` 1.0 (no_std, no OS) with Embassy async runtime. Measures battery discharge capacity via ADS1115 ADCs over I2C, controls discharge loads via GPIO, and displays per-channel status on SK6812 RGB LEDs.

See `.specs/battery-capacity-tester.md` for the full hardware and firmware specification.

## Quick Reference

```bash
cargo build                  # Debug build
cargo build --release        # Release build (opt-level "s")
cargo run                    # Build, flash, and monitor (via espflash)
cargo run --release          # Release flash and monitor
cargo fmt && cargo clippy    # Format and lint
espflash monitor             # Monitor serial output (no rebuild)
espflash board-info          # Check connected device
```

**After making changes, always run `cargo fmt && cargo build` to verify.**

## Architecture

```
src/
  main.rs              # Entry point, Embassy executor setup, task spawning
  config.rs            # Compile-time configuration (pins, thresholds, timing)
  channel.rs           # Battery channel state machine (idle/discharge/done/error)
  adc.rs               # ADS1115 ADC reading task (I2C, voltage/current measurement)
  led.rs               # SK6812 LED status display task (RMT peripheral)
  serial.rs            # UART serial output task (periodic status reporting)
```

## Key Patterns

- **no_std + no_main**: All code is bare-metal. No standard library, no heap by default (though `esp-alloc` is available).
- **Embassy async tasks**: Each module runs as an `#[embassy_executor::task]` for cooperative multitasking.
- **Shared state via mutex**: Battery channel state is shared between tasks using `embassy_sync::mutex::Mutex`.
- **ADS1115 over I2C**: External 16-bit ADCs for precise voltage and current measurement, accessed via the `ads1x1x` driver crate.
- **SK6812 LEDs via RMT**: RGB status LEDs driven through the ESP32-C3 RMT peripheral using `esp-hal-smartled`.
- **Feature flags for chip selection**: `esp32c3` (default) feature gates HAL/println/backtrace/embassy/smartled crate chip support.
- **Static configuration**: `config.rs` holds compile-time constants (pin assignments, voltage thresholds, timing intervals).

## Build Toolchain

- Target: `riscv32imc-unknown-none-elf`
- Uses nightly Rust (`build-std = ["core", "alloc"]`)
- Runner: `espflash flash --monitor` (configured in `.cargo/config.toml`)
- esp-hal 1.0 provides its own linker script (no custom `link.x` needed)

## Code Style

- Standard `cargo fmt` formatting
- Logging via `log` crate macros + `esp-println` backend (level set by `ESP_LOG` env var, default `INFO`)
