use serde::Deserialize;

// Configuration structures
#[derive(Debug, Deserialize)]
pub struct Config {
    pub wifi: WifiConfig,
    pub device: DeviceConfig,
    pub matter: MatterConfig,
}

#[derive(Debug, Deserialize)]
pub struct WifiConfig {
    pub ssid: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct DeviceConfig {
    pub name: String,
    pub led_pin: u8,
}

#[derive(Debug, Deserialize)]
pub struct MatterConfig {
    pub vendor_id: u16,
}

// Load configuration at compile time - will fail build if config.toml is missing
const CONFIG_STR: &str = include_str!("../../config.toml");

// Parse configuration using lazy_static to ensure it's only parsed once
use std::sync::OnceLock;

static CONFIG: OnceLock<Config> = OnceLock::new();

pub fn get_config() -> &'static Config {
    CONFIG.get_or_init(|| {
        toml::from_str(CONFIG_STR).expect(
            "Failed to parse config.toml. \
             Make sure you have copied config.toml.example to config.toml \
             and filled in your actual values.",
        )
    })
}
