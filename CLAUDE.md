# CLAUDE.md

> This is a living document. Update it when you learn new preferences, patterns, or project conventions. Don't ask, just update it if something is missing or outdated.

## Project Overview

Multi-binary bare-metal Rust firmware for ESP32-C3 (RISC-V). Uses `esp-hal` 1.0 (no_std, no OS) with Embassy async runtime via `esp-rtos`. Primary binary is an 8-channel NiMH battery capacity tester with WiFi push notifications.

See `.specs/battery-capacity-tester.md` for the full hardware and firmware specification.

## Quick Reference

```bash
# Battery tester (default, includes WiFi)
cargo run --bin battery-tester             # Flash and monitor
cargo build --bin battery-tester           # Build only
cargo build --bin battery-tester --release # Release build

# Blink (no WiFi, minimal)
cargo run --bin blink --no-default-features --features esp32c3

# General
cargo fmt && cargo clippy                  # Format and lint
espflash monitor                           # Monitor serial output (no rebuild)
espflash board-info                        # Check connected device
```

**After making changes, always run `cargo fmt && cargo build` to verify.**

## Setup

1. Copy config: `cp .cargo/config.toml.example .cargo/config.toml`
2. Fill in WiFi credentials and Pushover API keys in `.cargo/config.toml`
3. Build: `cargo build --bin battery-tester`

Secrets are compiled in via `env!()` macros. The build fails with a clear error if any env var is missing.

## Architecture

```
src/
  lib.rs                          # Shared library crate
  adc.rs                          #   ADS1115 8-channel ADC driver (I2C, shared bus)
  wifi.rs                         #   WiFi station init + DHCP (behind "wifi" feature)
  bin/
    battery_tester/               # Main application binary
      main.rs                     #   Entry point, Embassy setup, discharge manager task
      config.rs                   #   Thresholds, timing, Pushover config (env vars)
      channel.rs                  #   Discharge state machine (Idle/Scanning/Ready/Discharging/Complete/Error)
      led.rs                      #   SK6812 LED status task (color-coded per channel)
      serial.rs                   #   UART command interface (START/STATUS/STOP)
      notify.rs                   #   Pushover HTTP notification task (via reqwless)
    blink/                        # Simple LED blink test binary
      main.rs
```

## Key Patterns

- **Multi-binary with shared lib**: `src/lib.rs` exports reusable modules (`adc`, `wifi`). Each `src/bin/*/main.rs` is a standalone firmware image.
- **Feature-gated WiFi**: The `wifi` feature (default on) pulls in `esp-radio`, `embassy-net`, `reqwless`, `esp-alloc`. Disable with `--no-default-features --features esp32c3` for lightweight binaries.
- **Embassy async tasks**: Each module runs as an `#[embassy_executor::task]` for cooperative multitasking.
- **Shared state via mutex**: Battery channel state shared between tasks using `embassy_sync::mutex::Mutex<CriticalSectionRawMutex, _>`.
- **Compile-time secrets**: WiFi creds and API keys via `env!()` macros, sourced from `.cargo/config.toml` (gitignored). Template in `.cargo/config.toml.example`.
- **ADS1115 over shared I2C**: Two ADC modules on one bus via `embedded_hal_bus::i2c::CriticalSectionDevice`.
- **SK6812 LEDs via RMT**: RGB status LEDs driven through ESP32-C3 RMT peripheral.
- **HTTP notifications**: Pushover via `reqwless` HTTP client through a local nginx HTTPS relay.

## Build Toolchain

- Target: `riscv32imc-unknown-none-elf`
- Nightly Rust (`build-std = ["core", "alloc"]`)
- Runner: `espflash flash --monitor` (configured in `.cargo/config.toml`)
- Linker script: `linkall.x` from esp-hal (via rustflags in config)

## Code Style

- Standard `cargo fmt` formatting
- Logging via `log` crate macros + `esp-println` backend (level set by `ESP_LOG` env var, default `INFO`)
