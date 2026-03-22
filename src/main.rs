#![no_std]
#![no_main]

mod adc;
mod channel;
mod config;
mod led;
mod notify;
mod serial;

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel, Sender};
use embassy_time::{Duration, Instant, Timer};
use esp_backtrace as _;
use esp_hal::gpio::{Level, Output, OutputConfig, Pin};
use esp_hal::i2c::master::{Config as I2cConfig, I2c};
use esp_hal::rmt::Rmt;
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::uart::{Config as UartConfig, Uart};
use static_cell::StaticCell;

use crate::adc::SharedI2cBus;
use crate::channel::{ChannelError, ChannelState, SharedState};
use crate::config::NUM_CHANNELS;
use crate::notify::{Notification, NotifyChannel};
use crate::serial::SerialCommand;

/// Concrete I2C type used throughout the project.
type I2cBus = I2c<'static, esp_hal::Blocking>;

#[esp_rtos::main]
async fn main(spawner: embassy_executor::Spawner) {
    // Initialize logging from ESP_LOG env var (default INFO)
    esp_println::logger::init_logger_from_env();

    // Initialize heap allocator (needed for WiFi firmware)
    esp_alloc::heap_allocator!(size: 72 * 1024);

    // Initialize HAL and peripherals
    let peripherals = esp_hal::init(esp_hal::Config::default());

    // Initialize Embassy RTOS (timer + software interrupt for task scheduling)
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_int =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_int.software_interrupt0);

    log::info!("Battery capacity tester starting up");

    // Initialize WiFi and get network stack
    let stack = notify::init_wifi(&spawner, peripherals.WIFI).await;

    // Initialize notification channel
    static NOTIFY_CHAN: StaticCell<NotifyChannel> = StaticCell::new();
    let notify_chan: &'static NotifyChannel =
        NOTIFY_CHAN.init(embassy_sync::channel::Channel::new());

    // Spawn notification task
    spawner
        .spawn(notify::notify_task(stack, notify_chan.receiver()))
        .unwrap();

    // Initialize I2C bus for ADS1115 communication
    let i2c = I2c::new(peripherals.I2C0, I2cConfig::default())
        .unwrap()
        .with_sda(peripherals.GPIO0)
        .with_scl(peripherals.GPIO1);

    // Store I2C bus in a static mutex for sharing between ADS1115 instances
    static I2C_BUS: StaticCell<SharedI2cBus<I2cBus>> = StaticCell::new();
    let i2c_bus: &'static SharedI2cBus<I2cBus> =
        I2C_BUS.init(critical_section::Mutex::new(core::cell::RefCell::new(i2c)));

    // Initialize MOSFET gate pins as output LOW (discharge disabled)
    let mosfet_pins: [Output<'static>; 8] = [
        Output::new(peripherals.GPIO2, Level::Low, OutputConfig::default()),
        Output::new(peripherals.GPIO3, Level::Low, OutputConfig::default()),
        Output::new(peripherals.GPIO4, Level::Low, OutputConfig::default()),
        Output::new(peripherals.GPIO5, Level::Low, OutputConfig::default()),
        Output::new(peripherals.GPIO6, Level::Low, OutputConfig::default()),
        Output::new(peripherals.GPIO7, Level::Low, OutputConfig::default()),
        Output::new(peripherals.GPIO8, Level::Low, OutputConfig::default()),
        Output::new(peripherals.GPIO9, Level::Low, OutputConfig::default()),
    ];

    // Initialize shared channel state
    static STATE: StaticCell<SharedState> = StaticCell::new();
    let state: &'static SharedState = STATE.init(embassy_sync::mutex::Mutex::new(
        [ChannelState::Idle; NUM_CHANNELS],
    ));

    // Initialize command channel for serial -> discharge_manager communication
    static CMD_CHAN: StaticCell<Channel<CriticalSectionRawMutex, SerialCommand, 4>> =
        StaticCell::new();
    let cmd_chan: &'static Channel<CriticalSectionRawMutex, SerialCommand, 4> =
        CMD_CHAN.init(Channel::new());

    // Initialize RMT peripheral for driving addressable LEDs
    let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80)).unwrap();

    // Initialize UART0 for serial command input.
    let uart = Uart::new(peripherals.UART0, UartConfig::default())
        .unwrap()
        .with_rx(peripherals.GPIO20)
        .with_tx(peripherals.GPIO21);

    // Spawn tasks
    spawner
        .spawn(led::led_task(
            rmt.channel0,
            peripherals.GPIO10.degrade(),
            state,
        ))
        .unwrap();
    spawner
        .spawn(serial::serial_task(uart, state, cmd_chan.sender()))
        .unwrap();
    spawner
        .spawn(discharge_manager(
            mosfet_pins,
            i2c_bus,
            state,
            cmd_chan.receiver(),
            notify_chan.sender(),
        ))
        .unwrap();

    log::info!("All tasks spawned. Send START to begin discharge test.");
}

/// Main discharge management task.
#[embassy_executor::task]
async fn discharge_manager(
    mut mosfets: [Output<'static>; 8],
    i2c_bus: &'static SharedI2cBus<I2cBus>,
    state: &'static SharedState,
    cmd_rx: embassy_sync::channel::Receiver<'static, CriticalSectionRawMutex, SerialCommand, 4>,
    notify_tx: Sender<'static, CriticalSectionRawMutex, Notification, 8>,
) {
    let mut battery_adc = adc::BatteryAdc::new(i2c_bus);

    loop {
        // Wait for START command
        loop {
            let cmd = cmd_rx.receive().await;
            if matches!(cmd, SerialCommand::Start) {
                break;
            }
        }

        log::info!("Starting battery scan...");

        // Mark all channels as scanning
        {
            let mut channels = state.lock().await;
            for slot in 0..NUM_CHANNELS {
                channels[slot] = ChannelState::Scanning;
            }
        }

        // Brief pause so LED task can show scanning state
        Timer::after(Duration::from_millis(500)).await;

        // Read OCV and classify each slot
        {
            let mut channels = state.lock().await;
            for slot in 0..NUM_CHANNELS {
                match battery_adc.read_voltage(slot as u8) {
                    Ok(voltage) => {
                        channels[slot] = channel::classify_ocv(voltage);
                        log::info!("Slot {}: {:.3}V -> {:?}", slot + 1, voltage, channels[slot]);
                    }
                    Err(_) => {
                        channels[slot] = ChannelState::Error(ChannelError::NoBattery);
                        log::warn!("Slot {}: ADC read error", slot + 1);
                    }
                }
            }
        }

        // Enable MOSFET discharge on slots that are Ready
        let mut active = [false; NUM_CHANNELS];
        let mut capacity_mah = [0.0f32; NUM_CHANNELS];
        let mut min_voltage = [5.0f32; NUM_CHANNELS];
        let mut below_cutoff_count = [0u8; NUM_CHANNELS];
        let mut adc_error_count = [0u8; NUM_CHANNELS];
        let mut start_time = [Instant::now(); NUM_CHANNELS];

        {
            let channels = state.lock().await;
            let now = Instant::now();
            for slot in 0..NUM_CHANNELS {
                if matches!(channels[slot], ChannelState::Ready { .. }) {
                    active[slot] = true;
                    start_time[slot] = now;
                    mosfets[slot].set_high();
                    log::info!("Slot {}: discharge started", slot + 1);
                }
            }
        }

        // Discharge loop: sample, accumulate, check cutoff
        let mut log_counter = 0u32;
        let mut last_sample_time = Instant::now();
        while active.iter().any(|&a| a) {
            Timer::after(Duration::from_millis(config::SAMPLE_INTERVAL_MS)).await;

            // Check for STOP command (non-blocking)
            if let Ok(cmd) = cmd_rx.try_receive() {
                if matches!(cmd, SerialCommand::Stop) {
                    log::info!("STOP received, aborting all channels");
                    for slot in 0..NUM_CHANNELS {
                        if active[slot] {
                            mosfets[slot].set_low();
                            active[slot] = false;
                        }
                    }
                    let mut channels = state.lock().await;
                    for slot in 0..NUM_CHANNELS {
                        if matches!(channels[slot], ChannelState::Discharging { .. }) {
                            channels[slot] = ChannelState::Idle;
                        }
                    }
                    break;
                }
            }

            let now = Instant::now();
            let dt_s = (now - last_sample_time).as_millis() as f32 / 1000.0;
            last_sample_time = now;

            log_counter += 1;

            for slot in 0..NUM_CHANNELS {
                if !active[slot] {
                    continue;
                }

                let elapsed_s = (Instant::now() - start_time[slot]).as_secs() as u32;

                match battery_adc.read_voltage(slot as u8) {
                    Ok(voltage) => {
                        adc_error_count[slot] = 0;

                        let current_a = voltage / config::DISCHARGE_RESISTOR_OHMS;
                        let current_ma = current_a * 1000.0;

                        // Accumulate capacity: mAh = sum(I_a * dt_s) / 3.6
                        capacity_mah[slot] += (current_a * dt_s) / 3.6;

                        if voltage < min_voltage[slot] {
                            min_voltage[slot] = voltage;
                        }

                        // Update shared state
                        {
                            let mut channels = state.lock().await;
                            channels[slot] = ChannelState::Discharging {
                                capacity_mah: capacity_mah[slot],
                                voltage,
                                current_ma,
                                elapsed_s,
                            };
                        }

                        // Check voltage cutoff
                        if voltage < config::VOLTAGE_CUTOFF {
                            below_cutoff_count[slot] += 1;
                            if below_cutoff_count[slot] >= config::CUTOFF_CONSECUTIVE_READINGS {
                                mosfets[slot].set_low();
                                active[slot] = false;

                                let slot_type = config::SlotType::from_slot(slot);
                                let result =
                                    config::BatteryResult::classify(slot_type, capacity_mah[slot]);

                                let mut channels = state.lock().await;
                                channels[slot] = ChannelState::Complete {
                                    capacity_mah: capacity_mah[slot],
                                    min_voltage: min_voltage[slot],
                                    duration_s: elapsed_s,
                                };

                                log::info!(
                                    "Slot {}: COMPLETE - {:.0} mAh, {:?}, min {:.3}V, {}s",
                                    slot + 1,
                                    capacity_mah[slot],
                                    result,
                                    min_voltage[slot],
                                    elapsed_s
                                );

                                // Send push notification
                                let _ = notify_tx.try_send(Notification {
                                    slot: (slot + 1) as u8,
                                    slot_type,
                                    capacity_mah: capacity_mah[slot],
                                    result,
                                    duration_s: elapsed_s,
                                });
                            }
                        } else {
                            below_cutoff_count[slot] = 0;
                        }

                        // Periodic per-channel logging
                        if log_counter % config::LOG_INTERVAL_S == 0 {
                            log::info!(
                                "Slot {}: {:.3}V {:.0}mA {:.0}mAh {}s",
                                slot + 1,
                                voltage,
                                current_ma,
                                capacity_mah[slot],
                                elapsed_s
                            );
                        }
                    }
                    Err(_) => {
                        adc_error_count[slot] += 1;
                        log::warn!(
                            "Slot {}: ADC read error during discharge ({}/5)",
                            slot + 1,
                            adc_error_count[slot]
                        );

                        if adc_error_count[slot] >= 5 {
                            mosfets[slot].set_low();
                            active[slot] = false;
                            log::error!(
                                "Slot {}: too many consecutive ADC errors, stopping discharge",
                                slot + 1
                            );

                            let mut channels = state.lock().await;
                            channels[slot] = ChannelState::Error(ChannelError::NoBattery);
                        }
                    }
                }
            }
        }

        // Print summary
        log::info!("=== Discharge Complete ===");
        let channels = state.lock().await;
        for slot in 0..NUM_CHANNELS {
            match channels[slot] {
                ChannelState::Complete {
                    capacity_mah,
                    min_voltage,
                    duration_s,
                } => {
                    let slot_type = config::SlotType::from_slot(slot);
                    let result = config::BatteryResult::classify(slot_type, capacity_mah);
                    log::info!(
                        "Slot {} ({:?}): {:.0} mAh - {:?} (min {:.3}V, {}s)",
                        slot + 1,
                        slot_type,
                        capacity_mah,
                        result,
                        min_voltage,
                        duration_s
                    );
                }
                ChannelState::Error(e) => {
                    log::info!("Slot {}: {:?}", slot + 1, e);
                }
                _ => {}
            }
        }
    }
}
