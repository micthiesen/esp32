use crate::common::{config, init, led::LedController, wifi::WifiConnection};
use anyhow::Result;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::{gpio::PinDriver, peripherals::Peripherals},
    nvs::EspDefaultNvsPartition,
    timer::EspTaskTimerService,
};
use log::info;
use std::time::Duration;

pub fn run() -> Result<()> {
    init::init_esp()?;

    // Load configuration (will fail build if config.toml is missing)
    let config = config::get_config();

    info!(
        "Starting ESP32-C3 Matter Light Device: {}",
        config.device.name
    );
    info!("WiFi SSID: {}", config.wifi.ssid);
    info!("Matter vendor ID: {}", config.matter.vendor_id);

    // Initialize peripherals
    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;
    let _timer_service = EspTaskTimerService::new()?;

    // Configure LED using pin from config
    // Note: For now we'll just use GPIO 8, but config validation ensures it's set correctly
    if config.device.led_pin != 8 {
        return Err(anyhow::anyhow!(
            "Currently only GPIO 8 is supported for LED, got: {}",
            config.device.led_pin
        ));
    }
    let led_pin = PinDriver::output(peripherals.pins.gpio8)?;
    let led = LedController::new(led_pin);

    // Initialize and connect WiFi using config
    let mut wifi = WifiConnection::new(peripherals.modem, sys_loop, Some(nvs))?;
    wifi.connect(&config.wifi.ssid, &config.wifi.password)?;

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
