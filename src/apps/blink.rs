use crate::common::{init, led::LedController};
use anyhow::Result;
use esp_idf_svc::hal::{delay::FreeRtos, gpio::PinDriver, peripherals::Peripherals};
use log::info;

pub fn run() -> Result<()> {
    init::init_esp()?;

    info!("Starting ESP32-C3 LED blink example!");

    // Initialize peripherals
    let peripherals = Peripherals::take()?;

    // Configure GPIO 8 as output (built-in LED on many ESP32-C3 boards)
    // Note: Some boards use different pins - common ones are GPIO 2, 8, or 10
    let led_pin = PinDriver::output(peripherals.pins.gpio8)?;
    let led = LedController::new(led_pin);

    // Main blink loop
    let mut counter = 0u32;
    loop {
        // Turn LED on
        led.set_state(true)?;
        info!("Cycle {}", counter);

        // Wait 500ms
        FreeRtos::delay_ms(500);

        // Turn LED off
        led.set_state(false)?;

        // Wait 500ms
        FreeRtos::delay_ms(500);

        counter = counter.wrapping_add(1);
    }
}