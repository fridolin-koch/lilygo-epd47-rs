use esp_hal::{
    clock::Clocks,
    gpio::OutputPin,
    peripheral::Peripheral,
    peripherals,
    prelude::*,
    rmt,
    rmt::{Channel, PulseCode, TxChannel, TxChannelCreator},
    Blocking,
};

pub(crate) struct Rmt {
    tx_channel: Option<Channel<Blocking, 1>>,
}

impl Rmt {
    pub(crate) fn new(
        pin: impl Peripheral<P = impl OutputPin>,
        rmt: impl Peripheral<P = peripherals::RMT>,
        clocks: &Clocks,
    ) -> Result<Self, crate::Error> {
        //  into_ref!(rmt);
        let rmt = rmt::Rmt::new(rmt, 80.MHz(), clocks, None).map_err(crate::Error::Rmt)?;
        let tx_channel = rmt
            .channel1
            .configure(
                pin,
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
        Ok(Rmt {
            tx_channel: Some(tx_channel),
        })
    }

    pub(crate) fn pulse(&mut self, high: u16, low: u16, wait: bool) -> Result<(), crate::Error> {
        let tx_channel = self.tx_channel.take().unwrap();
        while tx_channel.is_busy() {}

        let data = if high > 0 {
            [
                PulseCode {
                    level1: true,
                    length1: high,
                    level2: false,
                    length2: low,
                },
                PulseCode::default(), // end of pulse indicator
            ]
        } else {
            [
                PulseCode {
                    level1: true,
                    length1: low,
                    level2: false,
                    length2: 0,
                },
                // FIXME: find more elegant solution
                PulseCode::default(), /* end of pulse indicator (redundant, but simplifies the
                                       * code) */
            ]
        };
        let tx = tx_channel
            .transmit_single_block(&data)
            .map_err(|(err, _)| err)
            .map_err(crate::Error::Rmt)?;

        let result = if wait { tx.wait() } else { tx.no_wait() };
        match result {
            Err((error, channel)) => {
                self.tx_channel = Some(channel);
                return Err(crate::Error::Rmt(error));
            }
            Ok(channel) => self.tx_channel = Some(channel),
        };
        Ok(())
    }
}
