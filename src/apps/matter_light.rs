use crate::common::{init, led::LedController, wifi::WifiConnection};
use anyhow::Result;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::{gpio::PinDriver, peripherals::Peripherals},
    nvs::EspDefaultNvsPartition,
    timer::EspTaskTimerService,
};
use log::info;
use std::time::Duration;

// Configure your WiFi credentials here
// You can also set them as environment variables: WIFI_SSID and WIFI_PASS
const SSID: &str = "your-wifi-ssid";    // Replace with your WiFi SSID
const PASS: &str = "your-wifi-password"; // Replace with your WiFi password

pub fn run() -> Result<()> {
    init::init_esp()?;

    info!("Starting ESP32-C3 Matter Light Device!");

    // Initialize peripherals
    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;
    let _timer_service = EspTaskTimerService::new()?;

    // Configure LED (GPIO 8 on ESP32-C3)
    let led_pin = PinDriver::output(peripherals.pins.gpio8)?;
    let led = LedController::new(led_pin);

    // Initialize and connect WiFi
    let mut wifi = WifiConnection::new(peripherals.modem, sys_loop, Some(nvs))?;
    wifi.connect(SSID, PASS)?;

    // For now, we'll implement basic functionality without full Matter stack
    // This serves as a foundation that can be extended with Matter protocol later
    info!("ESP32-C3 ready - LED can be controlled");
    info!("Matter integration will be added when dependencies stabilize");

    // Test LED functionality in a loop
    let mut state = false;
    loop {
        led.set_state(state)?;
        state = !state;
        std::thread::sleep(Duration::from_secs(2));
    }
}