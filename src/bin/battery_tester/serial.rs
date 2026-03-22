use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Sender;
use embassy_time::{Duration, Timer};
use esp_println::println;
use heapless::String;

use crate::channel::{ChannelError, ChannelState, SharedState};
use crate::config::{BatteryResult, SlotType, NUM_CHANNELS};

#[derive(Clone, Copy, Debug)]
pub enum SerialCommand {
    Status,
    Stop,
    StopSlot(usize),
}

fn parse_command(s: &str) -> Option<SerialCommand> {
    let trimmed = s.trim();
    if trimmed.eq_ignore_ascii_case("STATUS") {
        Some(SerialCommand::Status)
    } else if trimmed.eq_ignore_ascii_case("STOP") {
        Some(SerialCommand::Stop)
    } else if trimmed.len() > 4 && trimmed[..4].eq_ignore_ascii_case("STOP") {
        // "STOP 3" or "STOP3"
        let num_str = trimmed[4..].trim();
        if let Ok(n) = num_str.parse::<usize>() {
            if (1..=8).contains(&n) {
                Some(SerialCommand::StopSlot(n - 1))
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    }
}

fn print_status(states: &[ChannelState; NUM_CHANNELS]) {
    println!("┌──────┬──────┬──────────────────────────────────┐");
    println!("│ Slot │ Type │ State                            │");
    println!("├──────┼──────┼──────────────────────────────────┤");

    for (i, state) in states.iter().enumerate() {
        let slot_type = SlotType::from_slot(i);
        let type_str = match slot_type {
            SlotType::AA => " AA ",
            SlotType::AAA => "AAA ",
        };

        // State column is exactly 34 chars wide (between │ delimiters)
        match state {
            ChannelState::Idle => {
                println!(
                    "│  {:>2}  │ {} │ Idle                             │",
                    i + 1,
                    type_str
                );
            }
            ChannelState::Scanning => {
                println!(
                    "│  {:>2}  │ {} │ Scanning                         │",
                    i + 1,
                    type_str
                );
            }
            ChannelState::Discharging {
                capacity_mah,
                voltage,
                current_ma,
                elapsed_s,
            } => {
                println!(
                    "│  {:>2}  │ {} │ Disch {:>4.0}mAh {:.3}V {:>3.0}mA {:>5}s│",
                    i + 1,
                    type_str,
                    capacity_mah,
                    voltage,
                    current_ma,
                    elapsed_s
                );
            }
            ChannelState::Complete {
                capacity_mah,
                min_voltage: _,
                duration_s,
            } => {
                let result = BatteryResult::classify(slot_type, *capacity_mah);
                let result_str = match result {
                    BatteryResult::Good => "GOOD",
                    BatteryResult::Weak => "WEAK",
                    BatteryResult::Dead => "DEAD",
                };
                println!(
                    "│  {:>2}  │ {} │ Done  {:>4.0}mAh {:>5}s {:<4}        │",
                    i + 1,
                    type_str,
                    capacity_mah,
                    duration_s,
                    result_str
                );
            }
            ChannelState::Error(err) => {
                let err_str = match err {
                    ChannelError::WrongChemistry => "Wrong chemistry",
                    ChannelError::NotCharged => "Not charged",
                    ChannelError::Timeout => "Timeout",
                    ChannelError::AdcFault => "ADC fault",
                };
                println!("│  {:>2}  │ {} │ Error: {:<26}│", i + 1, type_str, err_str);
            }
        }
    }
    println!("└──────┴──────┴──────────────────────────────────┘");
}

#[embassy_executor::task]
pub async fn serial_task(
    mut usb_serial: esp_hal::usb_serial_jtag::UsbSerialJtag<'static, esp_hal::Blocking>,
    state: &'static SharedState,
    command_tx: Sender<'static, CriticalSectionRawMutex, SerialCommand, 4>,
) {
    let mut line_buf: String<64> = String::new();

    loop {
        // Yield to let other tasks run
        Timer::after(Duration::from_millis(10)).await;

        if let Ok(b) = usb_serial.read_byte() {
            if b == b'\n' || b == b'\r' {
                if !line_buf.is_empty() {
                    if let Some(cmd) = parse_command(line_buf.as_str()) {
                        match cmd {
                            SerialCommand::Status => {
                                let states = state.lock().await;
                                let snapshot = *states;
                                drop(states);
                                print_status(&snapshot);
                            }
                            SerialCommand::Stop => {
                                println!(">> Stopping...");
                                let _ = command_tx.try_send(cmd);
                            }
                            SerialCommand::StopSlot(slot) => {
                                println!(">> Stopping slot {}...", slot + 1);
                                let _ = command_tx.try_send(cmd);
                            }
                        }
                    } else {
                        println!("Unknown command: {}", line_buf.as_str());
                        println!("Commands: STATUS, STOP [N]");
                    }
                    line_buf.clear();
                }
            } else if line_buf.push(b as char).is_err() {
                println!("Input too long, discarding");
                line_buf.clear();
            }
        }
    }
}
