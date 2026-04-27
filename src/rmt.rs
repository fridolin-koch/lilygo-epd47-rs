use esp_hal::{
    gpio::Level,
    peripherals,
    rmt,
    rmt::{Channel, PulseCode, Tx, TxChannelCreator, TxTransaction},
    time::Rate,
    Blocking,
};

pub(crate) struct Rmt<'a> {
    tx_channel: Option<Channel<'a, Blocking, Tx>>,
    _rmt: peripherals::RMT<'a>,
}

impl<'a> Rmt<'a> {
    pub(crate) fn new(_rmt: peripherals::RMT<'a>) -> Self {
        Rmt {
            tx_channel: None,
            _rmt,
        }
    }

    fn ensure_channel(&mut self) -> Result<(), crate::Error> {
        if self.tx_channel.is_some() {
            return Ok(());
        }
        let freq = Rate::from_mhz(80);
        let rmt = rmt::Rmt::new(unsafe { peripherals::RMT::steal() }, freq)
            .map_err(crate::Error::RmtConfig)?;
        let config = rmt::TxChannelConfig::default()
            .with_clk_divider(8)
            .with_idle_output_level(Level::Low)
            .with_idle_output(true)
            .with_carrier_modulation(false)
            .with_carrier_level(Level::Low);
        let tx_channel = rmt
            .channel1
            .configure_tx(&config)
            .map_err(crate::Error::RmtConfig)?
            .with_pin(unsafe { peripherals::GPIO38::steal() });
        self.tx_channel = Some(tx_channel);
        Ok(())
    }

    pub(crate) fn pulse<'b>(
        &mut self,
        data: &'b [PulseCode],
        wait: bool,
    ) -> Result<Option<TxTransaction<'a, 'b>>, crate::Error> {
        self.ensure_channel()?;
        let tx_channel = self.tx_channel.take().ok_or(crate::Error::Unknown)?;
        let tx = tx_channel
            .transmit(data)
            .map_err(|(err, _)| crate::Error::Rmt(err))?;
        if wait {
            // if false {
            self.tx_channel = Some(
                tx.wait()
                    .map_err(|(err, _)| err)
                    .map_err(crate::Error::Rmt)?,
            );
            Ok(None)
        } else {
            Ok(Some(tx))
        }
    }

    pub fn reclaim_channel<'b>(
        &mut self,
        tx: TxTransaction<'a, 'b>,
    ) -> Result<(), crate::Error> {
        let channel = tx.wait().map_err(|(err, _)| crate::Error::Rmt(err))?;
        self.tx_channel = Some(channel);
        Ok(())
    }
}
