use ads1x1x::{channel, Ads1x1x, DataRate16Bit, FullScaleRange, TargetAddr};
use core::cell::RefCell;
use critical_section::Mutex;
use embedded_hal_bus::i2c::CriticalSectionDevice;

/// Shared I2C bus type, used by both ADS1115 devices.
pub type SharedI2cBus<I2C> = Mutex<RefCell<I2C>>;

/// ADS1115 device type alias for readability.
type Ads<I2C> = Ads1x1x<
    CriticalSectionDevice<'static, I2C>,
    ads1x1x::ic::Ads1115,
    ads1x1x::ic::Resolution16Bit,
    ads1x1x::mode::OneShot,
>;

/// ADC error type wrapping the underlying I2C error.
#[derive(Debug)]
pub enum AdcError<E> {
    I2c(E),
    InvalidSlot,
    InvalidInputData,
}

impl<E: core::fmt::Debug> From<ads1x1x::Error<E>> for AdcError<E> {
    fn from(e: ads1x1x::Error<E>) -> Self {
        match e {
            ads1x1x::Error::I2C(e) => AdcError::I2c(e),
            ads1x1x::Error::InvalidInputData => AdcError::InvalidInputData,
        }
    }
}

/// Wraps two ADS1115 modules on a shared I2C bus, providing 8 voltage channels.
///
/// Slots 0-3 map to ADS1115 #1 (0x48, ADDR=GND) channels A0-A3.
/// Slots 4-7 map to ADS1115 #2 (0x49, ADDR=VDD) channels A0-A3.
pub struct BatteryAdc<I2C: embedded_hal::i2c::I2c + 'static> {
    adc1: Ads<I2C>,
    adc2: Ads<I2C>,
}

impl<I2C: embedded_hal::i2c::I2c + 'static> BatteryAdc<I2C> {
    /// Create a new BatteryAdc from a shared I2C bus.
    ///
    /// Both ADS1115 devices are configured with PGA +/-4.096V and 128 SPS data rate.
    pub fn new(i2c_bus: &'static SharedI2cBus<I2C>) -> Self {
        let i2c1 = CriticalSectionDevice::new(i2c_bus);
        let i2c2 = CriticalSectionDevice::new(i2c_bus);

        let mut adc1 = Ads1x1x::new_ads1115(i2c1, TargetAddr::default());
        let mut adc2 = Ads1x1x::new_ads1115(i2c2, TargetAddr::Vdd);

        // Configure PGA to +/-4.096V for both devices
        let _ = adc1.set_full_scale_range(FullScaleRange::Within4_096V);
        let _ = adc1.set_data_rate(DataRate16Bit::Sps128);

        let _ = adc2.set_full_scale_range(FullScaleRange::Within4_096V);
        let _ = adc2.set_data_rate(DataRate16Bit::Sps128);

        Self { adc1, adc2 }
    }

    /// Read voltage from the given slot (0-7).
    ///
    /// At PGA +/-4.096V, 1 LSB = 0.125 mV, so voltage = raw * 0.000125.
    pub fn read_voltage(&mut self, slot: u8) -> Result<f32, AdcError<I2C::Error>> {
        let raw = match slot {
            0 => nb::block!(self.adc1.read(channel::SingleA0))?,
            1 => nb::block!(self.adc1.read(channel::SingleA1))?,
            2 => nb::block!(self.adc1.read(channel::SingleA2))?,
            3 => nb::block!(self.adc1.read(channel::SingleA3))?,
            4 => nb::block!(self.adc2.read(channel::SingleA0))?,
            5 => nb::block!(self.adc2.read(channel::SingleA1))?,
            6 => nb::block!(self.adc2.read(channel::SingleA2))?,
            7 => nb::block!(self.adc2.read(channel::SingleA3))?,
            _ => return Err(AdcError::InvalidSlot),
        };

        // PGA +/-4.096V: 1 LSB = 0.125 mV = 0.000125 V
        let voltage = raw as f32 * 0.000_125;
        Ok(voltage)
    }
}
