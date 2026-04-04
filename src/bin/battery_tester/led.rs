use embassy_time::{Duration, Timer};
use esp_hal_smartled::SmartLedsAdapter;
use rgb::Rgba;
use smart_leds::SmartLedsWrite;

use crate::channel::{ChannelError, ChannelState, SharedState};
use crate::config::{BatteryResult, SlotType, NUM_CHANNELS};

const BRIGHTNESS: f32 = 0.1;
const UPDATE_PERIOD_MS: u64 = 100;
const OFF: Rgba<u8> = Rgba { r: 0, g: 0, b: 0, a: 0 };

/// Create an RGBW color with correct GRB channel order for SK6812W.
/// Inputs are logical R, G, B values; output swaps R/G for the hardware.
fn dim(r: u8, g: u8, b: u8) -> Rgba<u8> {
    Rgba {
        r: (g as f32 * BRIGHTNESS) as u8,
        g: (r as f32 * BRIGHTNESS) as u8,
        b: (b as f32 * BRIGHTNESS) as u8,
        a: 0,
    }
}

fn color_for_state(state: &ChannelState, frame: u32) -> Rgba<u8> {
    match state {
        ChannelState::Idle => OFF,
        ChannelState::Scanning => dim(255, 80, 0),
        ChannelState::Discharging { .. } => {
            if (frame / 5).is_multiple_of(2) {
                dim(0, 0, 255)
            } else {
                OFF
            }
        }
        ChannelState::Complete { .. } => unreachable!(),
        ChannelState::Error(err) => match err {
            ChannelError::WrongChemistry => {
                if frame.is_multiple_of(2) {
                    dim(255, 0, 0)
                } else {
                    OFF
                }
            }
            ChannelError::NotCharged => {
                if (frame / 5).is_multiple_of(2) {
                    dim(255, 0, 0)
                } else {
                    OFF
                }
            }
            ChannelError::Timeout => dim(255, 0, 0),
            ChannelError::AdcFault => dim(255, 0, 0),
        },
    }
}

fn color_for_complete(slot: usize, capacity_mah: f32, frame: u32) -> Rgba<u8> {
    let slot_type = SlotType::from_slot(slot);
    let result = BatteryResult::classify(slot_type, capacity_mah);

    match result {
        BatteryResult::Good => dim(0, 255, 0),
        BatteryResult::Weak => {
            if (frame / 5).is_multiple_of(2) {
                dim(0, 255, 0)
            } else {
                OFF
            }
        }
        BatteryResult::Dead => dim(255, 0, 0),
    }
}

#[embassy_executor::task]
pub async fn led_task(
    rmt_channel: esp_hal::rmt::ChannelCreator<'static, esp_hal::Blocking, 0>,
    led_pin: esp_hal::gpio::AnyPin<'static>,
    state: &'static SharedState,
) {
    let mut buf = esp_hal_smartled::smart_led_buffer!(NUM_CHANNELS; RGBW);
    let mut smartled: SmartLedsAdapter<
        '_,
        { esp_hal_smartled::buffer_size_rgbw(NUM_CHANNELS) },
        Rgba<u8>,
    > = SmartLedsAdapter::new_with_color(rmt_channel, led_pin, &mut buf);
    let mut frame: u32 = 0;

    loop {
        let states = state.lock().await;
        let snapshot = *states;
        drop(states);

        let mut colors = [OFF; NUM_CHANNELS];

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
