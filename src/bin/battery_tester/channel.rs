use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;

use crate::config::{NUM_CHANNELS, VOLTAGE_ALKALINE, VOLTAGE_NOT_CHARGED, VOLTAGE_NO_BATTERY};

#[derive(Clone, Copy, Debug)]
pub enum ChannelError {
    NoBattery,
    WrongChemistry,
    NotCharged,
}

#[derive(Clone, Copy, Debug)]
pub enum ChannelState {
    Idle,
    Scanning,
    Ready {
        ocv: f32,
    },
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

#[allow(dead_code)]
impl ChannelState {
    pub fn is_active(&self) -> bool {
        matches!(self, ChannelState::Discharging { .. })
    }

    pub fn is_complete(&self) -> bool {
        matches!(self, ChannelState::Complete { .. } | ChannelState::Error(_))
    }

    pub fn is_idle(&self) -> bool {
        matches!(self, ChannelState::Idle)
    }
}

pub type SharedState = Mutex<CriticalSectionRawMutex, [ChannelState; NUM_CHANNELS]>;

pub fn classify_ocv(voltage: f32) -> ChannelState {
    if voltage < VOLTAGE_NO_BATTERY {
        ChannelState::Error(ChannelError::NoBattery)
    } else if voltage > VOLTAGE_ALKALINE {
        ChannelState::Error(ChannelError::WrongChemistry)
    } else if voltage < VOLTAGE_NOT_CHARGED {
        ChannelState::Error(ChannelError::NotCharged)
    } else {
        ChannelState::Ready { ocv: voltage }
    }
}
