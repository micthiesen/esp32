use embassy_time::{Duration, Timer};
use esp_hal_smartled::SmartLedsAdapter;
use smart_leds::{SmartLedsWrite, RGB8};

use crate::channel::{ChannelError, ChannelState, SharedState};
use crate::config::{BatteryResult, SlotType, NUM_CHANNELS};

/// Brightness scale factor (0.0 - 1.0). These LEDs are very bright.
const BRIGHTNESS: f32 = 0.3;

/// LED update period in milliseconds.
const UPDATE_PERIOD_MS: u64 = 100;

/// Scale an RGB value by the brightness factor.
fn dim(r: u8, g: u8, b: u8) -> RGB8 {
    RGB8 {
        r: (r as f32 * BRIGHTNESS) as u8,
        g: (g as f32 * BRIGHTNESS) as u8,
        b: (b as f32 * BRIGHTNESS) as u8,
    }
}

/// Map a channel state (and frame counter) to an LED color.
fn color_for_state(state: &ChannelState, frame: u32) -> RGB8 {
    match state {
        ChannelState::Idle => RGB8 { r: 0, g: 0, b: 0 },

        ChannelState::Scanning => dim(255, 80, 0), // yellow

        ChannelState::Ready { .. } => dim(0, 255, 0), // solid green

        ChannelState::Discharging { .. } => {
            // Blink blue at 1 Hz to distinguish from Weak result (blink green)
            if (frame / 5) % 2 == 0 {
                dim(0, 0, 255)
            } else {
                RGB8 { r: 0, g: 0, b: 0 }
            }
        }

        ChannelState::Complete { .. } => dim(0, 255, 0), // handled by color_for_complete in task loop

        ChannelState::Error(err) => match err {
            ChannelError::NoBattery => dim(255, 0, 0), // solid red

            ChannelError::WrongChemistry => {
                // Fast blink red at 4 Hz (about 1.25 frames per toggle at 100ms)
                if (frame / 1) % 2 == 0 {
                    dim(255, 0, 0)
                } else {
                    RGB8 { r: 0, g: 0, b: 0 }
                }
            }

            ChannelError::NotCharged => {
                // Slow blink red at 1 Hz
                if (frame / 5) % 2 == 0 {
                    dim(255, 0, 0)
                } else {
                    RGB8 { r: 0, g: 0, b: 0 }
                }
            }
        },
    }
}

/// Map a Complete state to an LED color, using slot index for battery type classification.
fn color_for_complete(slot: usize, capacity_mah: f32, frame: u32) -> RGB8 {
    let slot_type = SlotType::from_slot(slot);
    let result = BatteryResult::classify(slot_type, capacity_mah);

    match result {
        BatteryResult::Good => dim(0, 255, 0), // solid green

        BatteryResult::Weak => {
            // Blink green at 1 Hz
            if (frame / 5) % 2 == 0 {
                dim(0, 255, 0)
            } else {
                RGB8 { r: 0, g: 0, b: 0 }
            }
        }

        BatteryResult::Dead => dim(255, 0, 0), // solid red
    }
}

#[embassy_executor::task]
pub async fn led_task(
    rmt_channel: esp_hal::rmt::ChannelCreator<'static, esp_hal::Blocking, 0>,
    led_pin: esp_hal::gpio::AnyPin<'static>,
    state: &'static SharedState,
) {
    let mut buf = esp_hal_smartled::smart_led_buffer!(NUM_CHANNELS);
    let mut smartled = SmartLedsAdapter::new(rmt_channel, led_pin, &mut buf);
    let mut frame: u32 = 0;

    loop {
        let states = state.lock().await;
        let snapshot = *states;
        drop(states);

        let mut colors = [RGB8 { r: 0, g: 0, b: 0 }; NUM_CHANNELS];

        for (i, ch_state) in snapshot.iter().enumerate() {
            colors[i] = match ch_state {
                ChannelState::Complete { capacity_mah, .. } => {
                    color_for_complete(i, *capacity_mah, frame)
                }
                other => color_for_state(other, frame),
            };
        }

        let _ = smartled.write(colors.iter().copied());

        frame = frame.wrapping_add(1);
        Timer::after(Duration::from_millis(UPDATE_PERIOD_MS)).await;
    }
}
