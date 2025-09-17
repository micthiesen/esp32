use crate::common::{config, init, wifi::WifiConnection};
use anyhow::Result;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop, hal::peripherals::Peripherals, nvs::EspDefaultNvsPartition,
};
use log::info;
use std::time::Duration;

pub fn run() -> Result<()> {
    init::init_esp()?;

    // Load configuration
    let config = config::get_config();

    info!("=== WiFi Test App for {} ===", config.device.name);
    info!("Target SSID: {}", config.wifi.ssid);

    // Initialize peripherals
    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    // Initialize and connect WiFi
    let mut wifi = WifiConnection::new(peripherals.modem, sys_loop, Some(nvs))?;

    info!("Attempting to connect to WiFi...");
    wifi.connect(&config.wifi.ssid, &config.wifi.password)?;

    info!("✅ WiFi connected successfully!");

    // Main loop - display useful WiFi information
    loop {
        match display_wifi_info(&wifi) {
            Ok(_) => {}
            Err(e) => info!("❌ Error getting WiFi info: {}", e),
        }

        std::thread::sleep(Duration::from_secs(10));
    }
}

fn display_wifi_info(wifi: &WifiConnection) -> Result<()> {
    info!("=== WiFi Status ===");

    // Connection status
    let connected = wifi.is_connected()?;
    info!("Connected: {}", if connected { "✅ Yes" } else { "❌ No" });

    if !connected {
        return Ok(());
    }

    // Get network interface info through the WiFi driver
    let wifi_driver = wifi.wifi().wifi();
    let ip_info = wifi_driver.sta_netif().get_ip_info()?;

    info!("IP Address: {}", ip_info.ip);
    info!("Subnet Mask: {}", ip_info.subnet.mask);
    info!("Gateway: {}", ip_info.subnet.gateway);

    // Get DNS info
    let dns = wifi_driver.sta_netif().get_dns();
    info!("Primary DNS: {}", dns);

    // Get MAC address
    match wifi_driver.get_mac(esp_idf_svc::wifi::WifiDeviceId::Sta) {
        Ok(mac) => {
            info!(
                "MAC Address: {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
            );
        }
        Err(e) => {
            info!("⚠️  Could not get MAC address: {}", e);
        }
    }

    // Try to get basic WiFi configuration info
    match wifi_driver.get_configuration() {
        Ok(config) => {
            if let esp_idf_svc::wifi::Configuration::Client(client_config) = config {
                info!("Configured SSID: {}", client_config.ssid.as_str());
                info!("Auth Method: {:?}", client_config.auth_method);
            }
        }
        Err(e) => {
            info!("⚠️  Could not get WiFi configuration: {}", e);
        }
    }

    info!(
        "Network interface name: {}",
        wifi_driver.sta_netif().get_key()
    );
    info!("==================");

    Ok(())
}
