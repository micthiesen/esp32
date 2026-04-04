#![no_std]
#![no_main]

use embassy_time::{Duration, Timer};
use esp_backtrace as _;
esp_bootloader_esp_idf::esp_app_desc!();
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::timer::timg::TimerGroup;

#[esp_rtos::main]
async fn main(_spawner: embassy_executor::Spawner) {
    esp_println::logger::init_logger_from_env();

    let peripherals = esp_hal::init(esp_hal::Config::default());

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_int =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_int.software_interrupt0);

    let mut led = Output::new(peripherals.GPIO8, Level::Low, OutputConfig::default());

    log::info!("Blink started on GPIO8");

    loop {
        led.set_high();
        Timer::after(Duration::from_millis(500)).await;
        led.set_low();
        Timer::after(Duration::from_millis(500)).await;
    }
}
