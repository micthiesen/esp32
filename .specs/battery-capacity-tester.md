# Battery Capacity Tester

## Overview

8-channel NiMH battery capacity tester built on ESP32-C3 with bare-metal Rust (esp-hal 1.0, no_std, no ESP-IDF). Discharges AA/AAA batteries through fixed resistors, measures voltage via external ADS1115 ADCs, integrates current over time to calculate mAh capacity, and reports results via serial output and WS2812B RGB LED indicators.

## Why ESP32-C3

The original design targeted an ESP32 (Xtensa) for its 34 GPIOs, allowing direct drive of 16 discrete LEDs. By switching to WS2812B addressable LEDs (1 data pin for all 8 indicators), the ESP32-C3's 13 usable GPIOs are sufficient. Benefits:

- Standard nightly Rust toolchain (no Xtensa LLVM fork / `espup`)
- Same esp-hal 1.0 API surface
- Smaller, cheaper module
- Already in hand

The project retains feature flags for ESP32 (Xtensa) and ESP32-H2 if needed later.

## Hardware

### Microcontroller

ESP32-C3 dev board (RISC-V, USB-serial). Any module exposing GPIO0-GPIO10 + GPIO18-GPIO21.

### ADCs

- 2x ADS1115 16-bit I2C ADC modules on shared I2C bus
- ADC #1 (addr 0x48, ADDR pin to GND): A0-A3 = AA slots 1-4
- ADC #2 (addr 0x49, ADDR pin to VDD): A0-A3 = AAA slots 5-8
- Single-ended mode, PGA +/-4.096V (1 LSB = 0.125mV), 128 SPS

### Discharge Circuit (per channel, x8 identical)

```
Battery (+) ---+---- ADS1115 input (Ax)
               |
               +---- 2.2R 5W resistor ---- IRLZ44N drain
                                                |
                                           IRLZ44N source ---- GND
                                                |
                                           IRLZ44N gate ---- ESP32-C3 GPIO
Battery (-) ---- GND
```

IRLZ44N is logic-level (Vgs threshold ~1.0V, 3.3V = full enhancement). No gate driver needed. Discharge current = V_battery / 2.2R (approx 545mA at 1.2V).

### Status LEDs

8x WS2812B (NeoPixel) addressable RGB LEDs on a single data line, driven via ESP32-C3 RMT peripheral. One LED per slot. Colors encode state:

| Color | Meaning |
|-------|---------|
| Off | No battery / idle |
| Solid red | Error (no battery, wrong chemistry, dead result) |
| Blinking red (fast) | Wrong chemistry detected (alkaline) |
| Blinking red (slow) | Not fully charged |
| Solid green | Ready / good result |
| Blinking green | Discharge in progress / weak result |
| Yellow | Scanning |

### GPIO Assignments

```
GPIO0  - I2C SDA (to both ADS1115 modules, 4.7k pull-up to 3.3V)
GPIO1  - I2C SCL (to both ADS1115 modules, 4.7k pull-up to 3.3V)
GPIO2  - MOSFET gate slot 1 (AA)
GPIO3  - MOSFET gate slot 2 (AA)
GPIO4  - MOSFET gate slot 3 (AA)
GPIO5  - MOSFET gate slot 4 (AA)
GPIO6  - MOSFET gate slot 5 (AAA)
GPIO7  - MOSFET gate slot 6 (AAA)
GPIO8  - MOSFET gate slot 7 (AAA)
GPIO9  - MOSFET gate slot 8 (AAA)
GPIO10 - WS2812B LED data line
GPIO20 - UART0 RX (serial commands)
GPIO21 - UART0 TX (serial output)
```

Total: 11 GPIOs used, 2 reserved for UART. Fits ESP32-C3 comfortably.

## Software Architecture

### Toolchain and Dependencies

Pure Rust, no_std, no ESP-IDF. Standard nightly Rust toolchain (RISC-V target).

```
esp-hal 1.0           # HAL (stable I2C, GPIO, UART; unstable RMT for LEDs)
esp-rtos 0.2          # Embassy integration for esp-hal (replaces esp-hal-embassy)
esp-println 0.16      # UART logging backend
esp-backtrace 0.18    # Panic handler
esp-alloc 0.9         # Optional heap allocator
embassy-executor 0.9  # Async task executor
embassy-time 0.5      # Async timers and delays
embassy-sync 0.7      # Async synchronization primitives
embassy-net 0.9       # TCP/IP networking stack
ads1x1x 0.3           # ADS1115 I2C driver (embedded-hal 1.0)
esp-hal-smartled 0.17 # WS2812B via RMT peripheral
smart-leds 0.4        # LED color/brightness utilities
esp-radio 0.17        # WiFi driver (replaces esp-wifi)
reqwless 0.14         # no_std HTTP client
heapless 0.8          # Stack-allocated collections
```

### Module Layout

```
src/
  main.rs                 # Entry point, Embassy executor, spawns tasks
  config.rs               # Pin assignments, thresholds, constants
  channel.rs              # Per-channel state machine
  adc.rs                  # ADS1115 abstraction (both modules)
  led.rs                  # WS2812B LED controller with blink patterns
  serial.rs               # UART command parser (START/STATUS/STOP)
```

### Entry Point (main.rs)

1. Initialize peripherals, clocks, Embassy runtime
2. Configure I2C bus, both ADS1115 modules
3. Configure MOSFET gate pins as output-low
4. Configure WS2812B LED strip via RMT
5. Spawn Embassy tasks: channel manager, LED updater, serial handler

### Channel State Machine (channel.rs)

Each slot runs through these states:

```
Idle --> Scanning --> Ready --> Discharging --> Complete
                 \-> Error (no battery / wrong chemistry / not charged)
```

**Shared state**: An `embassy_sync::mutex::Mutex`-protected array of 8 channel states, readable by the LED and serial tasks.

**Scanning phase** (on START command):
- Read OCV (open-circuit voltage) with MOSFET off
- Classify: <0.1V = no battery, >1.5V = alkaline, <1.1V = not charged, else ready

**Discharge phase** (per ready channel, all run concurrently):
1. Record start time, set MOSFET gate HIGH
2. Sample voltage every 1s via ADS1115 (round-robin all 8 channels within each second)
3. Accumulate capacity: `mAh += (V / 2.2) * (dt_seconds / 3.6)`
4. Terminate when voltage < 1.0V for 3 consecutive readings
5. Set MOSFET gate LOW

**Post-discharge classification**:
- AA (slots 1-4): good >= 1600mAh, weak 1000-1599, dead < 1000
- AAA (slots 5-8): good >= 600mAh, weak 400-599, dead < 400

### ADC Abstraction (adc.rs)

Wraps two `ads1x1x::Ads1x1x` instances sharing one I2C bus. Provides:
- `read_voltage(slot: u8) -> Result<f32, _>` - returns voltage in volts for any slot 0-7
- Maps slot number to correct ADS1115 address + channel
- Single-shot mode, waits for conversion ready

I2C bus sharing: use `embassy_sync::mutex::Mutex` around the I2C peripheral, or use `embedded_hal_bus::i2c::MutexDevice` for shared bus access.

### LED Controller (led.rs)

Embassy task running at ~30Hz (or slower, 10Hz is fine for blink patterns):
- Reads channel states from shared state
- Maps each channel's state to an RGB color + blink pattern
- Writes all 8 LED colors to WS2812B strip via `esp-hal-smartled`
- Blink patterns implemented via frame counter (toggle color vs off)

### Serial Interface (serial.rs)

- Baud: 115200 over UART0
- Embassy task that reads UART, parses commands
- Commands:
  - `START` - begin scan + discharge sequence
  - `STATUS` - print current state of all 8 channels
  - `STOP` - abort all active discharges
- Periodic output during discharge: every 10s per active channel, print slot/voltage/current/mAh/elapsed
- Summary table on completion

### Concurrency Model

Embassy async executor with cooperative multitasking:
- **Channel manager task**: owns MOSFET GPIOs, runs discharge loops for all active channels
- **LED task**: periodically reads shared state, updates WS2812B strip
- **Serial task**: reads UART commands, writes status output
- Shared state via `embassy_sync::mutex::Mutex<NoopRawMutex, _>` (single-core, no real mutex needed)

## Constants and Thresholds

```
DISCHARGE_RESISTOR_OHMS: 2.2
VOLTAGE_NO_BATTERY: 0.1      // Below this = empty slot
VOLTAGE_ALKALINE: 1.5        // Above this = wrong chemistry
VOLTAGE_NOT_CHARGED: 1.1     // Below this = not ready
VOLTAGE_CUTOFF: 1.0          // Discharge termination
CUTOFF_CONSECUTIVE: 3        // Readings below cutoff before stopping
SAMPLE_INTERVAL_MS: 1000     // ADC sample period
LOG_INTERVAL_S: 10           // Serial logging period
AA_GOOD_MAH: 1600
AA_WEAK_MAH: 1000
AAA_GOOD_MAH: 600
AAA_WEAK_MAH: 400
```

## Build and Flash

```bash
cargo build                   # Debug build
cargo build --release         # Release build (size-optimized)
cargo run                     # Build + flash + monitor via espflash
cargo fmt && cargo clippy     # Format and lint
```

## Open Questions

- **I2C bus sharing**: `embedded_hal_bus::i2c::MutexDevice` vs manual mutex. Decide during implementation based on what compiles cleanly with ads1x1x.
- **Heap usage**: ads1x1x and smartled may need small allocations. May need `esp-alloc` enabled, or everything fits on stack with `heapless`.
- **Embassy version pinning**: The spec lists versions from the current Cargo.toml era. During implementation, pin to whatever resolves cleanly with esp-hal 1.0.
