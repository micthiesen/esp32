#![no_std]
#![no_main]

use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::i2c::master::{Config as I2cConfig, I2c};
use esp_hal::timer::timg::TimerGroup;
use firmware::adc::{BatteryAdc, SharedI2cBus};
use static_cell::StaticCell;

esp_bootloader_esp_idf::esp_app_desc!();

type I2cBus = I2c<'static, esp_hal::Blocking>;

#[esp_rtos::main]
async fn main(_spawner: embassy_executor::Spawner) {
    esp_println::logger::init_logger_from_env();

    let peripherals = esp_hal::init(esp_hal::Config::default());

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_int =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_int.software_interrupt0);

    log::info!("ADC test starting up");

    // Initialize I2C bus (same pins as battery_tester)
    let i2c = I2c::new(peripherals.I2C0, I2cConfig::default())
        .unwrap()
        .with_sda(peripherals.GPIO6)
        .with_scl(peripherals.GPIO7);

    static I2C_BUS: StaticCell<SharedI2cBus<I2cBus>> = StaticCell::new();
    let i2c_bus: &'static SharedI2cBus<I2cBus> =
        I2C_BUS.init(critical_section::Mutex::new(core::cell::RefCell::new(i2c)));

    let mut adc = BatteryAdc::new(i2c_bus);

    log::info!("Reading all 8 ADC channels every 2 seconds...");

    loop {
        for slot in 0..8u8 {
            match adc.read_voltage(slot) {
                Ok(v) => log::info!("Slot {}: {:.3}V", slot, v),
                Err(e) => log::error!("Slot {}: read error: {:?}", slot, e),
            }
        }
        log::info!("---");
        Timer::after(Duration::from_secs(2)).await;
    }
}
