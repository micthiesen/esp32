# Continuous Per-Channel Mode

## Summary

Replace the current batch "START all channels" model with continuous per-channel operation. Each slot runs an independent state machine. Inserting a battery triggers that slot's test automatically. Every terminal state (success or error) produces both an LED indication and a push notification.

## Current Problems

The current design requires a manual `START` command that scans all 8 slots simultaneously and discharges them in lockstep. This means:

- You must load all batteries before pressing START
- You can't add a battery to an empty slot while others are discharging
- Removing a completed battery and inserting a new one requires waiting for all channels to finish, then sending START again
- Error states (wrong chemistry, not charged) are fire-and-forget with no notification

## UX Flow

1. Device powers on. All slots idle, LEDs off.
2. Insert a battery into any slot.
3. That slot auto-detects the battery (voltage appears), scans it, and begins discharging. LED shows progress.
4. Other slots can be filled independently at any time.
5. When a slot reaches a terminal state (complete, error, timeout), LED shows the result and a push notification is sent.
6. Remove the battery. Slot detects removal (voltage drops to ~0), resets to idle, LED turns off. Ready for the next battery.

No START command needed. STOP remains as an emergency stop for individual or all channels. STATUS still prints the table.

## Channel State Machine

Each slot runs this state machine independently:

```
                    ┌──────────────────────────────────┐
                    │                                  │
                    v                                  │
Idle ──> Scanning ──┬──> Ready ──> Discharging ──┬──> Complete ──> (remove battery) ──> Idle
                    │                            │
                    ├──> Error(WrongChemistry) ───┤
                    ├──> Error(NotCharged) ───────┤
                    │                            ├──> Error(Timeout) ──> (remove) ──> Idle
                    │                            └──> Error(AdcFault) ──> (remove) ──> Idle
                    │
                    └──> (no battery / removed during scan) ──> Idle
```

### State Definitions

| State | Meaning | LED | Notification |
|-------|---------|-----|-------------|
| **Idle** | No battery detected | Off | No |
| **Scanning** | Battery just detected, reading OCV | Yellow | No |
| **Ready** | Valid NiMH detected, about to discharge | Solid green (brief) | No |
| **Discharging** | Active discharge in progress | Blinking blue | No |
| **Complete** | Discharge finished, capacity measured | Result color (see below) | Yes |
| **Error(WrongChemistry)** | Alkaline or other non-NiMH detected | Fast blink red | Yes |
| **Error(NotCharged)** | NiMH but OCV too low to test | Slow blink red | Yes |
| **Error(Timeout)** | Discharge exceeded MAX_DISCHARGE_S | Solid red | Yes |
| **Error(AdcFault)** | Repeated ADC read failures | Solid red | Yes |

**Complete result colors:**
- Good capacity: solid green
- Weak capacity: blinking green
- Dead: solid red

### Transitions

**Idle -> Scanning**: Polling reads voltage > VOLTAGE_NO_BATTERY on a previously-idle slot.

**Scanning -> Ready**: OCV is in valid NiMH range (VOLTAGE_NOT_CHARGED < V < VOLTAGE_ALKALINE).

**Scanning -> Error**: OCV indicates wrong chemistry or insufficient charge.

**Scanning -> Idle**: Voltage dropped back below VOLTAGE_NO_BATTERY (battery removed during scan).

**Ready -> Discharging**: Immediate. Enable MOSFET, begin sampling.

**Discharging -> Complete**: Voltage below VOLTAGE_CUTOFF for CUTOFF_CONSECUTIVE_READINGS consecutive samples.

**Discharging -> Error(Timeout)**: Elapsed time exceeds MAX_DISCHARGE_S.

**Discharging -> Error(AdcFault)**: 5 consecutive ADC read failures.

**Any terminal state -> Idle**: Voltage drops below VOLTAGE_NO_BATTERY (battery removed). Only checked while in a terminal state (Complete or Error). During Discharging, a voltage drop is a cutoff, not a removal.

## Notifications

Every terminal state sends a push notification (when WiFi is enabled):

- **Complete**: slot number, type (AA/AAA), capacity, result (good/weak/dead), duration
- **Error(WrongChemistry)**: slot number, "alkaline or non-NiMH detected"
- **Error(NotCharged)**: slot number, "battery not sufficiently charged", OCV reading
- **Error(Timeout)**: slot number, capacity so far, duration
- **Error(AdcFault)**: slot number, "ADC communication failure"

The `Notification` struct needs to be generalized to cover error cases, not just completions.

## Architecture Changes

### discharge_manager -> channel_manager

Rename to `channel_manager`. Runs a single loop polling all 8 channels every SAMPLE_INTERVAL_MS. No longer waits for a START command. Each iteration:

1. For each slot, read voltage from ADC
2. Based on current state + voltage reading, advance the state machine
3. On state transitions, update shared state, control MOSFET, log, send notifications

Per-channel mutable state (capacity accumulator, min voltage, error counts, start time, below-cutoff count) moves into a per-channel struct rather than parallel arrays.

### Per-channel context struct

```rust
struct ChannelContext {
    capacity_mah: f32,
    min_voltage: f32,
    below_cutoff_count: u8,
    adc_error_count: u8,
    start_time: Instant,
    last_sample_time: Instant,
    log_counter: u32,
}
```

Created fresh when a channel enters Discharging. Dropped when it exits.

### Battery removal detection

While in a terminal state (Complete or any Error), the channel manager periodically reads the ADC. If voltage < VOLTAGE_NO_BATTERY, transition back to Idle. This is the "remove battery to reset" mechanism.

### Serial commands

- **START**: Removed. No longer needed.
- **STATUS**: Unchanged. Prints the 8-channel table.
- **STOP**: Accepts optional slot number. `STOP` stops all, `STOP 3` stops slot 3 only. Stopped channels return to Idle.

### Notification struct

Generalize to cover all terminal states:

```rust
enum NotificationKind {
    Complete {
        capacity_mah: f32,
        result: BatteryResult,
        duration_s: u32,
    },
    WrongChemistry {
        ocv: f32,
    },
    NotCharged {
        ocv: f32,
    },
    Timeout {
        capacity_mah: f32,
        duration_s: u32,
    },
    AdcFault,
}

struct Notification {
    slot: u8,
    slot_type: SlotType,
    kind: NotificationKind,
}
```

### ChannelError rename

Replace `NoBattery` with removal detection (transition to Idle, not an error state). Replace `AdcFault` for the ADC failure case (currently reuses `NoBattery` which is confusing). Keep `WrongChemistry`, `NotCharged`, `Timeout`. Add `AdcFault`.

### Polling interval

The channel manager polls every SAMPLE_INTERVAL_MS (1s). For idle channels, this doubles as battery detection. No separate detection timer needed since ADC reads are fast.

### LED behavior

No changes to the LED task. It already reads shared state and maps to colors. The state machine changes are transparent to it, just make sure the new states map correctly.

### Scan settling time

When a battery is first detected (Idle -> Scanning), wait briefly (500ms) before reading OCV. This lets the voltage settle after contact is made. The channel stays in Scanning during this period (yellow LED).

## Module Responsibilities (post-refactor)

| Module | Owns | Responsibility |
|--------|------|---------------|
| **channel.rs** | State types | ChannelState enum, ChannelError enum, ChannelContext struct, classify_ocv(), state transition logic |
| **main.rs** | Task wiring | Peripheral init, task spawning. Channel manager loop lives here (or in its own module if large) |
| **led.rs** | LED output | Maps ChannelState to LED colors. No logic changes needed |
| **serial.rs** | UART I/O | STATUS table, STOP command. Remove START |
| **notify.rs** | Push notifications | Sends all terminal state notifications, not just completions |
| **config.rs** | Constants | Thresholds, timing. Add SCAN_SETTLE_MS, IDLE_POLL_INTERVAL_MS if different from sample interval |
| **adc.rs** | ADC hardware | Unchanged |

## What Doesn't Change

- Hardware wiring, GPIO assignments
- ADC driver (adc.rs)
- I2C bus sharing
- LED hardware/driver
- WiFi/notification transport
- Build toolchain, feature flags
- Wokwi simulation (update scenario to not send START, just wait for auto-detection)
