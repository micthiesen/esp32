use anyhow::Result;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::modem::Modem,
    nvs::EspDefaultNvsPartition,
    wifi::{BlockingWifi, ClientConfiguration, Configuration, EspWifi},
};
use log::info;

pub struct WifiConnection {
    wifi: BlockingWifi<EspWifi<'static>>,
}

impl WifiConnection {
    pub fn new(
        modem: Modem,
        sys_loop: EspSystemEventLoop,
        nvs: Option<EspDefaultNvsPartition>,
    ) -> Result<Self> {
        let wifi = BlockingWifi::wrap(EspWifi::new(modem, sys_loop.clone(), nvs)?, sys_loop)?;
        Ok(Self { wifi })
    }

    pub fn connect(&mut self, ssid: &str, password: &str) -> Result<()> {
        info!("Configuring WiFi for SSID: {}", ssid);

        self.wifi
            .set_configuration(&Configuration::Client(ClientConfiguration {
                ssid: ssid.try_into().unwrap(),
                password: password.try_into().unwrap(),
                ..Default::default()
            }))?;

        info!("Starting WiFi...");
        self.wifi.start()?;

        info!("Connecting to WiFi network...");
        self.wifi.connect()?;

        info!("Waiting for network interface to come up...");
        self.wifi.wait_netif_up()?;

        let ip_info = self.wifi.wifi().sta_netif().get_ip_info()?;
        info!("WiFi connected! IP: {}", ip_info.ip);

        Ok(())
    }

    pub fn is_connected(&self) -> Result<bool> {
        Ok(self.wifi.is_connected()?)
    }

    pub fn disconnect(&mut self) -> Result<()> {
        self.wifi.disconnect()?;
        self.wifi.stop()?;
        info!("WiFi disconnected");
        Ok(())
    }

    pub fn wifi(&self) -> &BlockingWifi<EspWifi<'static>> {
        &self.wifi
    }
}
