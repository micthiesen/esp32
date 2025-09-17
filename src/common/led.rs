use anyhow::Result;
use esp_idf_svc::hal::gpio::{Level, PinDriver};
use log::info;
use std::sync::{Arc, Mutex};

pub struct LedController {
    led: Arc<
        Mutex<PinDriver<'static, esp_idf_svc::hal::gpio::Gpio8, esp_idf_svc::hal::gpio::Output>>,
    >,
}

impl LedController {
    pub fn new(
        led: PinDriver<'static, esp_idf_svc::hal::gpio::Gpio8, esp_idf_svc::hal::gpio::Output>,
    ) -> Self {
        info!("LED controller initialized on GPIO 8");
        Self {
            led: Arc::new(Mutex::new(led)),
        }
    }

    pub fn set_state(&self, on: bool) -> Result<()> {
        if let Ok(mut led) = self.led.lock() {
            led.set_level(if on { Level::High } else { Level::Low })?;
            info!("LED turned {}", if on { "ON" } else { "OFF" });
        }
        Ok(())
    }

    pub fn toggle(&self) -> Result<()> {
        if let Ok(mut led) = self.led.lock() {
            led.toggle()?;
            info!("LED toggled");
        }
        Ok(())
    }
}