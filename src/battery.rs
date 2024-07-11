use esp_hal::{
    analog::adc::{Adc, AdcChannel, AdcConfig, AdcPin, Attenuation},
    gpio::AnalogPin,
    peripheral::Peripheral,
    peripherals::ADC2,
    prelude::nb,
};

pub struct Battery<'a, PIN>
where
    PIN: AdcChannel + AnalogPin,
{
    adc: Adc<'a, ADC2>,
    adc_pin: AdcPin<PIN, ADC2, esp_hal::analog::adc::AdcCalCurve<ADC2>>,
    correction_factor: f32,
}

impl<'a, PIN> Battery<'a, PIN>
where
    PIN: AdcChannel + AnalogPin,
{
    /// Create a new battery voltage reader
    pub fn new(pin: PIN, adc: impl Peripheral<P = ADC2> + 'a) -> Self {
        let mut config = AdcConfig::new();
        let adc_pin = config.enable_pin_with_cal(pin, Attenuation::Attenuation11dB);
        Battery {
            adc: Adc::new(adc, config),
            adc_pin,
            correction_factor: Self::DEFAULT_CORRECTION_FACTOR,
        }
    }

    /// Default voltage correction factor. This factor has been experimentally
    /// determined. It might be device specific.
    pub const DEFAULT_CORRECTION_FACTOR: f32 = 1.144632;

    /// Set a correction factor other than [`DEFAULT_CORRECTION_FACTOR`]
    pub fn set_correction_factor(&mut self, factor: f32) {
        self.correction_factor = factor
    }

    /// Read the current voltage of the battery
    pub fn read(&mut self) -> f32 {
        let v = nb::block!(self.adc.read_oneshot(&mut self.adc_pin)).unwrap();

        (((v as f32) * 2.0) / 1000.0) * self.correction_factor
    }
}
