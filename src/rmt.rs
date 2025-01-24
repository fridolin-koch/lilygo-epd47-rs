use core::ops::DerefMut;

use esp_hal::{
    gpio::GpioPin,
    into_ref,
    peripheral::{Peripheral, PeripheralRef},
    peripherals,
    prelude::*,
    rmt,
    rmt::{Channel, PulseCode, TxChannel, TxChannelCreator},
    Blocking,
};

pub(crate) struct Rmt<'a> {
    tx_channel: Option<Channel<Blocking, 1>>,
    rmt: PeripheralRef<'a, peripherals::RMT>,
}

impl<'a> Rmt<'a> {
    pub(crate) fn new(rmt: impl Peripheral<P = peripherals::RMT> + 'a) -> Self {
        into_ref!(rmt);
        Rmt {
            tx_channel: None,
            rmt,
        }
    }

    fn ensure_channel(&mut self) -> Result<(), crate::Error> {
        if self.tx_channel.is_some() {
            return Ok(());
        }
        let rmt = rmt::Rmt::new(
            unsafe { self.rmt.deref_mut().clone_unchecked() }, // TODO: find better solution
            80.MHz(),
        )
        .map_err(crate::Error::Rmt)?;
        let tx_channel = rmt
            .channel1
            .configure(
                unsafe { GpioPin::<38>::steal() }, // TODO: find better solution
                rmt::TxChannelConfig {
                    clk_divider: 8,
                    idle_output_level: false,
                    idle_output: true,
                    carrier_modulation: false,
                    carrier_level: false,
                    ..Default::default()
                },
            )
            .map_err(crate::Error::Rmt)?;
        self.tx_channel = Some(tx_channel);
        Ok(())
    }

    pub(crate) fn pulse(&mut self, high: u16, low: u16, wait: bool) -> Result<(), crate::Error> {
        self.ensure_channel()?;
        let tx_channel = self.tx_channel.take().ok_or(crate::Error::Unknown)?;
        let data = if high > 0 {
            [PulseCode::new(true, high, false, low), PulseCode::empty()]
        } else {
            [PulseCode::new(true, low, false, 0), PulseCode::empty()]
        };
        let tx = tx_channel.transmit(&data).map_err(crate::Error::Rmt)?;
        // FIXME: This is the culprit.. We need the channel later again but can't wait
        // due to some time sensitive operations. Not sure how to solve this
        if wait {
            self.tx_channel = Some(
                tx.wait()
                    .map_err(|(err, _)| err)
                    .map_err(crate::Error::Rmt)?,
            );
        }
        Ok(())
    }
}
