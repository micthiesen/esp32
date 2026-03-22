# Battery Tester Firmware

8-channel NiMH battery capacity tester. Insert batteries, get results. No buttons, no commands needed for normal operation.

## How It Works

Each slot operates independently. Insert a battery into any slot and it auto-detects, scans the open-circuit voltage, and begins discharging through a 2.2 ohm resistor. Voltage is sampled every second via an ADS1115 ADC. Current is integrated over time to calculate capacity in mAh. When voltage drops below 1.0V (3 consecutive readings), the test is complete. The LED shows the result and a push notification is sent.

Remove the battery and the slot resets, ready for the next one.

## Channel State Machine

Each of the 8 slots runs this state machine independently:

```
                        +-----------+
            +---------->|   Idle    |<-----------+
            |           +-----+-----+            |
            |                 |                  |
            |          voltage > 0.1V            |
            |          (battery detected)        |
            |                 v                  |
            |           +-----------+            |
            |           | Scanning  |            |
            |           +-----+-----+            |
            |                 |                  |
            |        +--------+--------+         |
            |        |        |        |         |
            |     >1.5V   <1.1V   1.1-1.5V      |
            |        |        |        |         |
            |        v        v        v         |
            |   +--------+ +------+ +----------+ |
            |   | Error: | |Error:| |Discharging| |
            |   | Wrong  | | Not  | +-----+----+ |
            |   | Chem   | |Chrgd |       |      |
            |   +---+----+ +--+---+  +----+----+ |
            |       |         |      |    |     | |
            |       |         |   <1.0V  >8h  ADC |
            |       |         |   (x3)   T/O  err |
            |       |         |      |    |     | |
            |       |         |      v    v     v |
            |       |         | +------+ +---+ +-+--+
            |       |         | | Done | |T/O| |ADC |
            |       |         | +--+---+ +-+-+ |Err |
            |       |         |    |       |   +--+-+
            |       |         |    |       |      |
            +-------+---------+----+-------+------+
                    battery removed (voltage < 0.1V)
```

### States

| State | What's happening | LED | Notifies |
|---|---|---|---|
| Idle | Empty slot, polling for battery | Off | No |
| Scanning | Battery detected, waiting 500ms for voltage to settle | Yellow | No |
| Discharging | MOSFET on, sampling voltage, accumulating mAh | Blinking blue | No |
| Complete | Discharge finished | Green (good), blink green (weak), red (dead) | Yes |
| Error: WrongChemistry | OCV > 1.5V, likely alkaline | Fast blink red | Yes |
| Error: NotCharged | OCV < 1.1V, NiMH not sufficiently charged | Slow blink red | Yes |
| Error: Timeout | Discharge exceeded 8 hours | Solid red | Yes |
| Error: AdcFault | 5 consecutive ADC read failures | Solid red | Yes |

Every terminal state (Complete or any Error) sends a push notification and holds its LED color until the battery is physically removed.

## Architecture

```
channel_manager task          serial task           led task
(owns MOSFETs + ADC)         (owns UART)          (owns LEDs)
        |                         |                     |
        |--- shared state -----+-+---------------------+
        |    (mutex)            |
        |                       |
        +<-- STOP/STATUS ------+
             commands
```

### Why one task, not eight

The ADC is shared. Two ADS1115 chips sit on one I2C bus. Running 8 tasks would mean 8 tasks contending on the same bus mutex. A single `channel_manager` task polls all 8 channels round-robin each second, which is simpler and avoids contention.

### Why `Action` return values

The state machine (`ChannelCtx`) is pure logic. It takes a voltage reading, returns an `Action` enum telling the caller what to do (enable/disable MOSFET, send notification). This separates state transitions from hardware side-effects, making the state machine testable in isolation and keeping I/O in one place.

### Why `nb::block!` (and its tradeoff)

The ADS1115 driver uses `nb::block!()` for I2C reads, which busy-spins for ~7.8ms per channel at 128 SPS. With 8 channels, that's ~62ms of CPU spinning per second where the Embassy executor can't schedule other tasks. This works because the other tasks (LEDs at 100ms, serial polling at 10ms) are tolerant of short delays. If a timing-sensitive task is added later, this should be revisited with interrupt-driven I2C.

## Serial Commands

Normal operation needs no commands. For debugging:

- `STATUS` - print a table of all 8 channel states
- `STOP` - emergency stop all channels, reset to idle
- `STOP N` - stop slot N (1-8) only

## Voltage Thresholds

| Threshold | Value | Purpose |
|---|---|---|
| No battery | < 0.1V | Slot is empty |
| Not charged | < 1.1V | NiMH OCV too low to test |
| Alkaline | > 1.5V | Wrong chemistry, reject |
| Cutoff | < 1.0V | Discharge complete (3 consecutive) |

## Capacity Classification

| Type | Good | Weak | Dead |
|---|---|---|---|
| AA (slots 1-4) | >= 1600 mAh | 1000-1599 mAh | < 1000 mAh |
| AAA (slots 5-8) | >= 600 mAh | 400-599 mAh | < 400 mAh |
