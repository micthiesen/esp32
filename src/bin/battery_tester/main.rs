#![no_std]
#![no_main]

esp_bootloader_esp_idf::esp_app_desc!();

mod channel;
mod config;
mod led;
#[cfg(feature = "wifi")]
mod notify;
mod serial;

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel, Sender};
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::gpio::{Level, Output, OutputConfig, Pin};
use esp_hal::i2c::master::{Config as I2cConfig, I2c};
use esp_hal::rmt::Rmt;
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::usb_serial_jtag::UsbSerialJtag;
use firmware::adc::{BatteryAdc, SharedI2cBus};
use static_cell::StaticCell;

#[cfg(feature = "wifi")]
use crate::channel::NotifyChannel;
use crate::channel::{Action, ChannelCtx, ChannelState, Notification, SharedState};
use crate::config::{SlotType, NUM_CHANNELS};
use crate::serial::SerialCommand;

type I2cBus = I2c<'static, esp_hal::Blocking>;

#[esp_rtos::main]
async fn main(spawner: embassy_executor::Spawner) {
    esp_println::logger::init_logger_from_env();

    let peripherals = esp_hal::init(esp_hal::Config::default());

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_int =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_int.software_interrupt0);

    log::info!("Battery capacity tester starting up");

    // Initialize WiFi and notifications (only when wifi feature is enabled)
    #[cfg(feature = "wifi")]
    let notify_tx = {
        esp_alloc::heap_allocator!(size: 72 * 1024);

        let stack = firmware::wifi::init(&spawner, peripherals.WIFI).await;

        static NOTIFY_CHAN: StaticCell<NotifyChannel> = StaticCell::new();
        let notify_chan: &'static NotifyChannel = NOTIFY_CHAN.init(Channel::new());

        spawner
            .spawn(notify::notify_task(stack, notify_chan.receiver()))
            .unwrap();

        Some(notify_chan.sender())
    };

    #[cfg(not(feature = "wifi"))]
    let notify_tx: Option<Sender<'static, CriticalSectionRawMutex, Notification, 8>> = None;

    // Initialize I2C bus for ADS1115 communication
    let i2c = I2c::new(peripherals.I2C0, I2cConfig::default())
        .unwrap()
        .with_sda(peripherals.GPIO6)
        .with_scl(peripherals.GPIO7);

    static I2C_BUS: StaticCell<SharedI2cBus<I2cBus>> = StaticCell::new();
    let i2c_bus: &'static SharedI2cBus<I2cBus> =
        I2C_BUS.init(critical_section::Mutex::new(core::cell::RefCell::new(i2c)));

    // Initialize MOSFET gate pins as output LOW (discharge disabled)
    let mosfet_pins: [Output<'static>; 8] = [
        Output::new(peripherals.GPIO2, Level::Low, OutputConfig::default()),
        Output::new(peripherals.GPIO3, Level::Low, OutputConfig::default()),
        Output::new(peripherals.GPIO4, Level::Low, OutputConfig::default()),
        Output::new(peripherals.GPIO5, Level::Low, OutputConfig::default()),
        Output::new(peripherals.GPIO8, Level::Low, OutputConfig::default()),
        Output::new(peripherals.GPIO9, Level::Low, OutputConfig::default()),
        Output::new(peripherals.GPIO10, Level::Low, OutputConfig::default()),
        Output::new(peripherals.GPIO20, Level::Low, OutputConfig::default()),
    ];

    static STATE: StaticCell<SharedState> = StaticCell::new();
    let state: &'static SharedState = STATE.init(embassy_sync::mutex::Mutex::new(
        [ChannelState::Idle; NUM_CHANNELS],
    ));

    static CMD_CHAN: StaticCell<Channel<CriticalSectionRawMutex, SerialCommand, 4>> =
        StaticCell::new();
    let cmd_chan: &'static Channel<CriticalSectionRawMutex, SerialCommand, 4> =
        CMD_CHAN.init(Channel::new());

    let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80)).unwrap();

    let usb_serial = UsbSerialJtag::new(peripherals.USB_DEVICE);

    spawner
        .spawn(led::led_task(
            rmt.channel0,
            peripherals.GPIO21.degrade(),
            state,
        ))
        .unwrap();
    spawner
        .spawn(serial::serial_task(usb_serial, state, cmd_chan.sender()))
        .unwrap();
    spawner
        .spawn(channel_manager(
            mosfet_pins,
            i2c_bus,
            state,
            cmd_chan.receiver(),
            notify_tx,
        ))
        .unwrap();

    log::info!("Battery capacity tester ready");
}

#[embassy_executor::task]
async fn channel_manager(
    mut mosfets: [Output<'static>; 8],
    i2c_bus: &'static SharedI2cBus<I2cBus>,
    state: &'static SharedState,
    cmd_rx: embassy_sync::channel::Receiver<'static, CriticalSectionRawMutex, SerialCommand, 4>,
    notify_tx: Option<Sender<'static, CriticalSectionRawMutex, Notification, 8>>,
) {
    let mut battery_adc = BatteryAdc::new(i2c_bus);
    let mut channels: [ChannelCtx; 8] = core::array::from_fn(|_| ChannelCtx::new());

    loop {
        Timer::after(Duration::from_millis(config::SAMPLE_INTERVAL_MS)).await;

        // Check for serial commands
        if let Ok(cmd) = cmd_rx.try_receive() {
            match cmd {
                SerialCommand::Stop => {
                    log::info!("STOP received, stopping all channels");
                    for slot in 0..NUM_CHANNELS {
                        let action = channels[slot].stop();
                        if matches!(action, Action::DisableMosfet) {
                            mosfets[slot].set_low();
                        }
                    }
                }
                SerialCommand::StopSlot(n) => {
                    if n < NUM_CHANNELS {
                        log::info!("STOP received for slot {}", n + 1);
                        let action = channels[n].stop();
                        if matches!(action, Action::DisableMosfet) {
                            mosfets[n].set_low();
                        }
                    }
                }
                _ => {}
            }
        }

        // Read all 8 ADC channels and update state machines
        for slot in 0..NUM_CHANNELS {
            let voltage = battery_adc.read_voltage(slot as u8).map_err(|_| ());
            let action = channels[slot].update(voltage, slot);

            match action {
                Action::EnableMosfet => {
                    mosfets[slot].set_high();
                }
                Action::DisableMosfet => {
                    mosfets[slot].set_low();
                }
                Action::Terminal(kind) => {
                    mosfets[slot].set_low();
                    if let Some(ref tx) = notify_tx {
                        let _ = tx.try_send(Notification {
                            slot: (slot + 1) as u8,
                            slot_type: SlotType::from_slot(slot),
                            kind,
                        });
                    }
                }
                Action::None => {}
            }
        }

        // Update shared state (single lock for all 8 slots)
        let mut shared = state.lock().await;
        for slot in 0..NUM_CHANNELS {
            shared[slot] = channels[slot].state();
        }
    }
}
