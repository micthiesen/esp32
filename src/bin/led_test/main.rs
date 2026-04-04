#![no_std]
#![no_main]

use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::gpio::Pin;
use esp_hal::rmt::Rmt;
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use esp_hal_smartled::SmartLedsAdapter;
use smart_leds::{SmartLedsWrite, RGB8};

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(_spawner: embassy_executor::Spawner) {
    esp_println::logger::init_logger_from_env();

    let peripherals = esp_hal::init(esp_hal::Config::default());

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_int =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_int.software_interrupt0);

    let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80)).unwrap();
    let mut buf = esp_hal_smartled::smart_led_buffer!(4; RGBW);
    let mut smartled: SmartLedsAdapter<'_, { esp_hal_smartled::buffer_size_rgbw(4) }, rgb::Rgba<u8>> =
        SmartLedsAdapter::new_with_color(rmt.channel0, peripherals.GPIO21.degrade(), &mut buf);

    log::info!("LED RGBW test: LED1=R, LED2=G, LED3=B, LED4=W");
    log::info!("Tell me what colors you see");

    let level = 25u8;

    let colors = [
        RGB8 { r: level, g: 0, b: 0 }.with_alpha(0),      // LED 1: RED
        RGB8 { r: 0, g: level, b: 0 }.with_alpha(0),       // LED 2: GREEN
        RGB8 { r: 0, g: 0, b: level }.with_alpha(0),       // LED 3: BLUE
        RGB8 { r: 0, g: 0, b: 0 }.with_alpha(level),       // LED 4: WHITE
    ];

    loop {
        let _ = smartled.write(colors.iter().copied());
        Timer::after(Duration::from_secs(1)).await;
    }
}
