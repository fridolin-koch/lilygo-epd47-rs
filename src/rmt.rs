use alloc::{boxed::Box, rc::Rc};
use core::{cell::RefCell, marker::PhantomPinned, mem, ops::DerefMut, pin::Pin, ptr::NonNull};
use core::cell::Cell;
use esp_hal::{
    clock::Clocks,
    gpio::OutputPin,
    peripheral::Peripheral,
    peripherals,
    prelude::*,
    rmt,
    rmt::{Channel, PulseCode, SingleShotTxTransaction, TxChannel, TxChannelCreator},
    Blocking,
};

#[derive(Default)]
enum TxChannelContainer<'a, C>
where
    C: TxChannel,
{
    #[default]
    None,
    Channel(C),
    Tx(SingleShotTxTransaction<'a, C, PulseCode>),
    // Tx(Box<Transmission<'a, C>>),
}

impl<'a, C> TxChannelContainer<'a, C>
where
    C: TxChannel,
{
    fn take(&mut self) -> Result<C, (rmt::Error, C)> {
        match mem::take(self) {
            Self::None => panic!("very broken"),
            Self::Channel(ch) => Ok(ch),
            Self::Tx(tx) => tx.wait(),
        }
    }
}

pub(crate) struct Rmt<'a> {
    tx_channel: TxChannelContainer<'a, Channel<Blocking, 1>>,
    data: [PulseCode; 2],
}

impl<'a, 'b> Rmt<'a>
where
    'b: 'a,
{
    pub(crate) fn new(
        pin: impl Peripheral<P = impl OutputPin>,
        rmt: impl Peripheral<P = peripherals::RMT>,
        clocks: &Clocks,
    ) -> Result<Self, crate::Error> {
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
            tx_channel: TxChannelContainer::Channel(tx_channel),
            data: [PulseCode::default(); 2],
        })
    }

    pub(crate) fn pulse(mut self, high: u16, low: u16, wait: bool) -> Result<(), crate::Error> {
        let tx_channel = match self.tx_channel.take() {
            Ok(channel) => channel,
            Err((err, channel)) => {
                self.tx_channel.set(TxChannelContainer::Channel(channel));
                return Err(crate::Error::Rmt(err));
            }
        };
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
                PulseCode::default(), /* end of pulse indicator (redundant, but simplifies
                                       * the code) */
            ]
        };
        let Rmt {
            data
        }

        // let mut tx = Transmission::new(data, tx_channel);
        let tx = tx_channel.transmit(self.data.borrow().as_ref());
        // FIXME: This is the culprit.. We need the channel later again but can't wait
        // due to some time sensitive operations. Not sure how to solve this
        if wait {
            self.tx_channel.set(TxChannelContainer::Channel(
                tx.wait()
                    .map_err(|(err, _)| err)
                    .map_err(crate::Error::Rmt)?,
            ));
        } else {
            self.tx_channel.set(TxChannelContainer::Tx(tx));
        }
        Ok(())
    }
}

struct Transmission<C>
where
    C: TxChannel,
{
    data: [PulseCode; 2],
    channel: Channel<Blocking, 1>
}
impl<C> Transmission<C>
where
    C: TxChannel,
{
    fn wait<'a>(mut self) -> Result<Rmt<'a>, (crate::Error, Rmt<'a>)> {
        let tx = self.channel.transmit(&self.data);
        match tx.wait() {
            Ok(channel) => Ok(Rmt{
                tx_channel: TxChannelContainer::Channel(channel)
            }),
            Err((err, channel)) => Err((crate::Error::Rmt(err), Rmt{
                tx_channel: TxChannelContainer::Channel(channel)
            }))
        }
    }
    
    fn nowait<'a>
}
