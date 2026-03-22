// Discharge circuit
pub const DISCHARGE_RESISTOR_OHMS: f32 = 2.2;

// Voltage thresholds (volts)
pub const VOLTAGE_NO_BATTERY: f32 = 0.1;
pub const VOLTAGE_ALKALINE: f32 = 1.5;
pub const VOLTAGE_NOT_CHARGED: f32 = 1.1;
pub const VOLTAGE_CUTOFF: f32 = 1.0;
pub const CUTOFF_CONSECUTIVE_READINGS: u8 = 3;

// Timing
pub const SAMPLE_INTERVAL_MS: u64 = 1000;
pub const LOG_INTERVAL_S: u32 = 10;

// Capacity thresholds (mAh)
pub const AA_GOOD_MAH: f32 = 1600.0;
pub const AA_WEAK_MAH: f32 = 1000.0;
pub const AAA_GOOD_MAH: f32 = 600.0;
pub const AAA_WEAK_MAH: f32 = 400.0;

// Number of channels
pub const NUM_CHANNELS: usize = 8;

// WiFi credentials (set via env vars in .cargo/config.toml)
pub const WIFI_SSID: &str = env!("WIFI_SSID");
pub const WIFI_PASSWORD: &str = env!("WIFI_PASSWORD");

// Pushover notification (set via env vars in .cargo/config.toml)
pub const PUSHOVER_TOKEN: &str = env!("PUSHOVER_TOKEN");
pub const PUSHOVER_USER: &str = env!("PUSHOVER_USER");
pub const NOTIFY_URL: &str = env!("NOTIFY_URL");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotType {
    AA,
    AAA,
}

impl SlotType {
    pub fn from_slot(slot: usize) -> Self {
        if slot < 4 {
            SlotType::AA
        } else {
            SlotType::AAA
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BatteryResult {
    Good,
    Weak,
    Dead,
}

impl BatteryResult {
    pub fn classify(slot_type: SlotType, capacity_mah: f32) -> Self {
        let (good_threshold, weak_threshold) = match slot_type {
            SlotType::AA => (AA_GOOD_MAH, AA_WEAK_MAH),
            SlotType::AAA => (AAA_GOOD_MAH, AAA_WEAK_MAH),
        };

        if capacity_mah >= good_threshold {
            BatteryResult::Good
        } else if capacity_mah >= weak_threshold {
            BatteryResult::Weak
        } else {
            BatteryResult::Dead
        }
    }
}
