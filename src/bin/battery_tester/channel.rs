use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::Instant;

use crate::config::{
    self, BatteryResult, SlotType, NUM_CHANNELS, VOLTAGE_ALKALINE, VOLTAGE_CUTOFF,
    VOLTAGE_NOT_CHARGED, VOLTAGE_NO_BATTERY,
};

// ---------------------------------------------------------------------------
// Channel state (shared between tasks via mutex)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub enum ChannelError {
    WrongChemistry,
    NotCharged,
    Timeout,
    AdcFault,
}

#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
pub enum ChannelState {
    Idle,
    Scanning,
    Discharging {
        capacity_mah: f32,
        voltage: f32,
        current_ma: f32,
        elapsed_s: u32,
    },
    Complete {
        capacity_mah: f32,
        min_voltage: f32,
        duration_s: u32,
    },
    Error(ChannelError),
}

pub type SharedState = Mutex<CriticalSectionRawMutex, [ChannelState; NUM_CHANNELS]>;

// ---------------------------------------------------------------------------
// Actions returned by the state machine
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum Action {
    /// No external side-effect needed.
    None,
    /// Turn on the MOSFET to begin discharging.
    EnableMosfet,
    /// Turn off the MOSFET (discharge ended or error).
    DisableMosfet,
    /// Channel reached a terminal state. Includes notification payload.
    Terminal(NotificationKind),
}

// ---------------------------------------------------------------------------
// Notification types (sent to push notification task)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
#[cfg_attr(not(feature = "wifi"), allow(dead_code))]
pub enum NotificationKind {
    Complete {
        capacity_mah: f32,
        result: BatteryResult,
        duration_s: u32,
    },
    WrongChemistry {
        ocv: f32,
    },
    NotCharged {
        ocv: f32,
    },
    Timeout {
        capacity_mah: f32,
        duration_s: u32,
    },
    AdcFault,
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(not(feature = "wifi"), allow(dead_code))]
pub struct Notification {
    pub slot: u8,
    pub slot_type: SlotType,
    pub kind: NotificationKind,
}

#[cfg(feature = "wifi")]
pub type NotifyChannel = embassy_sync::channel::Channel<CriticalSectionRawMutex, Notification, 8>;

// ---------------------------------------------------------------------------
// Per-channel context (owned by channel_manager, not shared)
// ---------------------------------------------------------------------------

pub struct ChannelCtx {
    state: ChannelState,
    // Discharge accumulators (only valid during Discharging)
    capacity_mah: f32,
    min_voltage: f32,
    below_cutoff_count: u8,
    adc_error_count: u8,
    start_time: Instant,
    last_sample_time: Instant,
    sample_count: u32,
    scan_time: Instant,
}

impl ChannelCtx {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            state: ChannelState::Idle,
            capacity_mah: 0.0,
            min_voltage: 5.0,
            below_cutoff_count: 0,
            adc_error_count: 0,
            start_time: now,
            last_sample_time: now,
            sample_count: 0,
            scan_time: now,
        }
    }

    pub fn state(&self) -> ChannelState {
        self.state
    }

    /// Returns true if this channel is actively discharging.
    pub fn is_discharging(&self) -> bool {
        matches!(self.state, ChannelState::Discharging { .. })
    }

    /// Returns true if this channel is in a terminal state (Complete or Error).

    /// Returns true every LOG_INTERVAL samples during discharge.
    pub fn should_log(&self) -> bool {
        self.sample_count > 0 && self.sample_count % config::LOG_INTERVAL_SAMPLES == 0
    }

    /// Drive the state machine with a new voltage reading (or ADC error).
    /// Returns an Action telling the caller what side-effect to perform.
    pub fn update(&mut self, voltage: Result<f32, ()>, slot: usize) -> Action {
        match self.state {
            ChannelState::Idle => self.update_idle(voltage),
            ChannelState::Scanning => self.update_scanning(voltage, slot),
            ChannelState::Discharging { .. } => self.update_discharging(voltage, slot),
            ChannelState::Complete { .. } | ChannelState::Error(_) => self.update_terminal(voltage),
        }
    }

    /// Force-stop this channel (from STOP command). Returns DisableMosfet if
    /// the channel was discharging, None otherwise.
    pub fn stop(&mut self) -> Action {
        if self.is_discharging() {
            self.state = ChannelState::Idle;
            self.reset();
            Action::DisableMosfet
        } else {
            self.state = ChannelState::Idle;
            self.reset();
            Action::None
        }
    }

    // -- State handlers -----------------------------------------------------

    fn update_idle(&mut self, voltage: Result<f32, ()>) -> Action {
        if let Ok(v) = voltage {
            if v >= VOLTAGE_NO_BATTERY {
                // Battery detected, start scanning
                self.state = ChannelState::Scanning;
                self.scan_time = Instant::now();
            }
        }
        Action::None
    }

    fn update_scanning(&mut self, voltage: Result<f32, ()>, slot: usize) -> Action {
        let v = match voltage {
            Ok(v) => v,
            Err(()) => {
                // ADC error during scan, back to idle
                self.state = ChannelState::Idle;
                return Action::None;
            }
        };

        // Battery removed during scan
        if v < VOLTAGE_NO_BATTERY {
            self.state = ChannelState::Idle;
            return Action::None;
        }

        // Wait for settle time
        let elapsed_ms = (Instant::now() - self.scan_time).as_millis();
        if elapsed_ms < config::SCAN_SETTLE_MS {
            return Action::None;
        }

        // Classify OCV
        let slot_type = SlotType::from_slot(slot);
        if v > VOLTAGE_ALKALINE {
            self.state = ChannelState::Error(ChannelError::WrongChemistry);
            log::warn!(
                "Slot {} ({:?}): {:.3}V - wrong chemistry",
                slot + 1,
                slot_type,
                v
            );
            return Action::Terminal(NotificationKind::WrongChemistry { ocv: v });
        }
        if v < VOLTAGE_NOT_CHARGED {
            self.state = ChannelState::Error(ChannelError::NotCharged);
            log::warn!(
                "Slot {} ({:?}): {:.3}V - not charged",
                slot + 1,
                slot_type,
                v
            );
            return Action::Terminal(NotificationKind::NotCharged { ocv: v });
        }

        // Valid NiMH, start discharging
        let now = Instant::now();
        self.capacity_mah = 0.0;
        self.min_voltage = v;
        self.below_cutoff_count = 0;
        self.adc_error_count = 0;
        self.start_time = now;
        self.last_sample_time = now;
        self.sample_count = 0;

        let slot_type = SlotType::from_slot(slot);
        log::info!(
            "Slot {} ({:?}): {:.3}V - discharge started",
            slot + 1,
            slot_type,
            v
        );

        self.state = ChannelState::Discharging {
            capacity_mah: 0.0,
            voltage: v,
            current_ma: v / config::DISCHARGE_RESISTOR_OHMS * 1000.0,
            elapsed_s: 0,
        };

        Action::EnableMosfet
    }

    fn update_discharging(&mut self, voltage: Result<f32, ()>, slot: usize) -> Action {
        let now = Instant::now();
        let dt_s = (now - self.last_sample_time).as_millis() as f32 / 1000.0;
        self.last_sample_time = now;
        self.sample_count += 1;

        let elapsed_s = (now - self.start_time).as_secs() as u32;

        let v = match voltage {
            Ok(v) => {
                self.adc_error_count = 0;
                v
            }
            Err(()) => {
                self.adc_error_count += 1;
                log::warn!(
                    "Slot {}: ADC error ({}/{})",
                    slot + 1,
                    self.adc_error_count,
                    config::ADC_ERROR_LIMIT
                );
                if self.adc_error_count >= config::ADC_ERROR_LIMIT {
                    self.state = ChannelState::Error(ChannelError::AdcFault);
                    return Action::Terminal(NotificationKind::AdcFault);
                }
                return Action::None;
            }
        };

        // Accumulate capacity
        let current_a = v / config::DISCHARGE_RESISTOR_OHMS;
        let current_ma = current_a * 1000.0;
        self.capacity_mah += (current_a * dt_s) / 3.6;

        if v < self.min_voltage {
            self.min_voltage = v;
        }

        // Update shared state
        self.state = ChannelState::Discharging {
            capacity_mah: self.capacity_mah,
            voltage: v,
            current_ma,
            elapsed_s,
        };

        // Check voltage cutoff
        if v < VOLTAGE_CUTOFF {
            self.below_cutoff_count += 1;
            if self.below_cutoff_count >= config::CUTOFF_CONSECUTIVE_READINGS {
                let slot_type = SlotType::from_slot(slot);
                let result = BatteryResult::classify(slot_type, self.capacity_mah);

                self.state = ChannelState::Complete {
                    capacity_mah: self.capacity_mah,
                    min_voltage: self.min_voltage,
                    duration_s: elapsed_s,
                };

                log::info!(
                    "Slot {}: COMPLETE - {:.0} mAh, {:?}, min {:.3}V, {}s",
                    slot + 1,
                    self.capacity_mah,
                    result,
                    self.min_voltage,
                    elapsed_s
                );

                return Action::Terminal(NotificationKind::Complete {
                    capacity_mah: self.capacity_mah,
                    result,
                    duration_s: elapsed_s,
                });
            }
        } else {
            self.below_cutoff_count = 0;
        }

        // Check timeout
        if elapsed_s >= config::MAX_DISCHARGE_S {
            self.state = ChannelState::Error(ChannelError::Timeout);

            log::warn!(
                "Slot {}: TIMEOUT after {}s ({:.0} mAh)",
                slot + 1,
                elapsed_s,
                self.capacity_mah
            );

            return Action::Terminal(NotificationKind::Timeout {
                capacity_mah: self.capacity_mah,
                duration_s: elapsed_s,
            });
        }

        // Periodic logging
        if self.should_log() {
            log::info!(
                "Slot {}: {:.3}V {:.0}mA {:.0}mAh {}s",
                slot + 1,
                v,
                current_ma,
                self.capacity_mah,
                elapsed_s
            );
        }

        Action::None
    }

    fn update_terminal(&mut self, voltage: Result<f32, ()>) -> Action {
        // In terminal state, watch for battery removal to reset
        if let Ok(v) = voltage {
            if v < VOLTAGE_NO_BATTERY {
                self.state = ChannelState::Idle;
                self.reset();
            }
        } else {
            // ADC error while in terminal state, treat as removal
            self.state = ChannelState::Idle;
            self.reset();
        }
        Action::None
    }

    fn reset(&mut self) {
        self.capacity_mah = 0.0;
        self.min_voltage = 5.0;
        self.below_cutoff_count = 0;
        self.adc_error_count = 0;
        self.sample_count = 0;
    }
}
