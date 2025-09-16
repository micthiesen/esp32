use esp_idf_svc::hal::{
    delay::FreeRtos,
    gpio::{Level, PinDriver},
    peripherals::Peripherals,
};

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("Starting ESP32-C3 LED blink example!");

    // Initialize peripherals
    let peripherals = Peripherals::take()?;

    // Configure GPIO 8 as output (built-in LED on many ESP32-C3 boards)
    // Note: Some boards use different pins - common ones are GPIO 2, 8, or 10
    let mut led = PinDriver::output(peripherals.pins.gpio8)?;

    log::info!("LED initialized on GPIO 8");

    // Main blink loop
    let mut counter = 0u32;
    loop {
        // Turn LED on
        led.set_level(Level::High)?;
        log::info!("LED ON - cycle {}", counter);

        // Wait 500ms
        FreeRtos::delay_ms(500);

        // Turn LED off
        led.set_level(Level::Low)?;
        log::info!("LED OFF - cycle {}", counter);

        // Wait 500ms
        FreeRtos::delay_ms(500);

        counter = counter.wrapping_add(1);
    }
}
