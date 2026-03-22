use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Sender;
use embassy_time::{Duration, Timer};
use esp_println::println;
use heapless::String;

use crate::channel::{ChannelState, SharedState};
use crate::config::{BatteryResult, SlotType, NUM_CHANNELS};

#[derive(Clone, Copy, Debug)]
pub enum SerialCommand {
    Start,
    Status,
    Stop,
}

/// Parse a trimmed, case-insensitive command string.
fn parse_command(s: &str) -> Option<SerialCommand> {
    let trimmed = s.trim();
    if trimmed.eq_ignore_ascii_case("START") {
        Some(SerialCommand::Start)
    } else if trimmed.eq_ignore_ascii_case("STATUS") {
        Some(SerialCommand::Status)
    } else if trimmed.eq_ignore_ascii_case("STOP") {
        Some(SerialCommand::Stop)
    } else {
        None
    }
}

/// Print a status table for all channels.
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
            ChannelState::Ready { ocv } => {
                println!(
                    "│  {:>2}  │ {} │ Ready  OCV={:.3}V                │",
                    i + 1,
                    type_str,
                    ocv
                );
            }
            ChannelState::Discharging {
                capacity_mah,
                voltage,
                current_ma,
                elapsed_s,
            } => {
                println!(
                    "│  {:>2}  │ {} │ Disch  {:.0}mAh {:.3}V {:.0}mA {:>4}s │",
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
                    "│  {:>2}  │ {} │ Done   {:.0}mAh {:>4}s {}        │",
                    i + 1,
                    type_str,
                    capacity_mah,
                    duration_s,
                    result_str
                );
            }
            ChannelState::Error(err) => {
                let err_str = match err {
                    crate::channel::ChannelError::NoBattery => "No battery",
                    crate::channel::ChannelError::WrongChemistry => "Wrong chemistry",
                    crate::channel::ChannelError::NotCharged => "Not charged",
                };
                println!("│  {:>2}  │ {} │ Error: {:<24} │", i + 1, type_str, err_str);
            }
        }
    }
    println!("└──────┴──────┴──────────────────────────────────┘");
}

#[embassy_executor::task]
pub async fn serial_task(
    mut uart: esp_hal::uart::Uart<'static, esp_hal::Blocking>,
    state: &'static SharedState,
    command_tx: Sender<'static, CriticalSectionRawMutex, SerialCommand, 4>,
) {
    let mut line_buf: String<64> = String::new();
    let mut byte_buf = [0u8; 1];

    loop {
        // Poll UART for incoming bytes
        match uart.read(&mut byte_buf) {
            Ok(n) if n > 0 => {
                let b = byte_buf[0];

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
                                other => {
                                    let _ = command_tx.try_send(other);
                                    match other {
                                        SerialCommand::Start => println!(">> Starting..."),
                                        SerialCommand::Stop => println!(">> Stopping..."),
                                        _ => {}
                                    }
                                }
                            }
                        } else {
                            println!("Unknown command: {}", line_buf.as_str());
                            println!("Commands: START, STATUS, STOP");
                        }
                        line_buf.clear();
                    }
                } else if line_buf.push(b as char).is_err() {
                    // Buffer full, discard
                    println!("Input too long, discarding");
                    line_buf.clear();
                }
            }
            Ok(_) => {
                // No data available (0 bytes read), yield and retry
                Timer::after(Duration::from_millis(50)).await;
            }
            Err(_) => {
                // Read error, yield and retry
                Timer::after(Duration::from_millis(50)).await;
            }
        }
    }
}
