use esp_hal::{
    dma::{self, DmaTxBuf},
    dma_buffers,
    gpio::{GpioPin, Level, Output, OutputPin},
    lcd_cam::{
        lcd::{i8080, i8080::Command},
        LcdCam,
    },
    peripheral::Peripheral,
    peripherals,
    prelude::*,
    Blocking,
};

use crate::rmt;

const DMA_BUFFER_SIZE: usize = 240;

struct ConfigRegister {
    latch_enable: bool,
    power_disable: bool,
    pos_power_enable: bool,
    neg_power_enable: bool,
    stv: bool,
    power_enable: bool, /* scan_direction, see https://github.com/vroland/epdiy/blob/main/src/board/epd_board_lilygo_t5_47.c#L199 */
    mode: bool,
    output_enable: bool,
}

impl Default for ConfigRegister {
    fn default() -> Self {
        ConfigRegister {
            latch_enable: false,
            power_disable: true,
            pos_power_enable: false,
            neg_power_enable: false,
            stv: true,
            power_enable: false,
            mode: false,
            output_enable: false,
        }
    }
}

struct ConfigWriter<'a> {
    pin_data: Output<'a>,
    pin_clk: Output<'a>,
    pin_str: Output<'a>,
    config: ConfigRegister,
}

impl<'a> ConfigWriter<'a> {
    fn new(
        data: impl Peripheral<P = impl OutputPin> + 'a,
        clk: impl Peripheral<P = impl OutputPin> + 'a,
        str: impl Peripheral<P = impl OutputPin> + 'a,
    ) -> Self {
        ConfigWriter {
            pin_data: Output::new(data, Level::High),
            pin_clk: Output::new(clk, Level::High),
            pin_str: Output::new(str, Level::Low),
            config: ConfigRegister::default(),
        }
    }

    fn write(&mut self) {
        self.pin_str.set_low();
        self.write_bool(self.config.output_enable);
        self.write_bool(self.config.mode);
        self.write_bool(self.config.power_enable);
        self.write_bool(self.config.stv);
        self.write_bool(self.config.neg_power_enable);
        self.write_bool(self.config.pos_power_enable);
        self.write_bool(self.config.power_disable);
        self.write_bool(self.config.latch_enable);
        self.pin_str.set_high();
    }

    #[inline(always)]
    fn write_bool(&mut self, v: bool) {
        self.pin_clk.set_low();
        self.pin_data.set_level(match v {
            true => Level::High,
            false => Level::Low,
        });
        self.pin_clk.set_high();
    }
}

pub struct PinConfig {
    pub data0: GpioPin<6>,
    pub data1: GpioPin<7>,
    pub data2: GpioPin<4>,
    pub data3: GpioPin<5>,
    pub data4: GpioPin<2>,
    pub data5: GpioPin<3>,
    pub data6: GpioPin<8>,
    pub data7: GpioPin<1>,
    pub cfg_data: GpioPin<13>,
    pub cfg_clk: GpioPin<12>,
    pub cfg_str: GpioPin<0>,
    pub lcd_dc: GpioPin<40>,
    pub lcd_wrx: GpioPin<41>,
    pub rmt: GpioPin<38>,
}

pub(crate) struct ED047TC1<'a> {
    i8080: Option<i8080::I8080<'a, Blocking>>,
    cfg_writer: ConfigWriter<'a>,
    rmt: rmt::Rmt<'a>,
    dma_buf: Option<DmaTxBuf>,
}

impl<'a> ED047TC1<'a> {
    pub(crate) fn new(
        pins: PinConfig,
        dma: impl Peripheral<P = peripherals::DMA> + 'a,
        lcd_cam: impl Peripheral<P = peripherals::LCD_CAM> + 'a,
        rmt: impl Peripheral<P = peripherals::RMT> + 'a,
    ) -> crate::Result<Self> {
        // configure data pins
        let tx_pins = i8080::TxEightBits::new(
            pins.data0, pins.data1, pins.data2, pins.data3, pins.data4, pins.data5, pins.data6,
            pins.data7,
        );

        // configure dma
        let dma = dma::Dma::new(dma);
        let channel = dma.channel0.configure(false, dma::DmaPriority::Priority0);

        // init lcd
        let lcd_cam = LcdCam::new(lcd_cam);

        // init panel config writer (?)
        let mut cfg_writer = ConfigWriter::new(pins.cfg_data, pins.cfg_clk, pins.cfg_str);
        cfg_writer.write();

        let (_, _, tx_buffer, tx_descriptors) = dma_buffers!(0, DMA_BUFFER_SIZE);
        let dma_buf =
            Some(DmaTxBuf::new(tx_descriptors, tx_buffer).map_err(crate::Error::DmaBuffer)?);

        let ctrl = ED047TC1 {
            i8080: Some(
                i8080::I8080::new(
                    lcd_cam.lcd,
                    channel.tx,
                    tx_pins,
                    10.MHz(),
                    i8080::Config {
                        cd_idle_edge: false,  // dc_idle_level
                        cd_cmd_edge: true,    // dc_cmd_level
                        cd_dummy_edge: false, // dc_dummy_level
                        cd_data_edge: false,  // dc_data_level
                        ..Default::default()
                    },
                )
                .with_ctrl_pins(pins.lcd_dc, pins.lcd_wrx),
            ),
            cfg_writer,
            rmt: rmt::Rmt::new(rmt),
            dma_buf,
        };
        Ok(ctrl)
    }

    pub(crate) fn power_on(&mut self) {
        self.cfg_writer.config.power_enable = true;
        self.cfg_writer.config.power_disable = false;
        self.cfg_writer.write();
        busy_delay(100 * 240);
        self.cfg_writer.config.neg_power_enable = true;
        self.cfg_writer.write();
        busy_delay(500 * 240);
        self.cfg_writer.config.pos_power_enable = true;
        self.cfg_writer.write();
        busy_delay(100 * 240);
        self.cfg_writer.config.stv = true;
        self.cfg_writer.write();
    }

    pub(crate) fn power_off(&mut self) {
        self.cfg_writer.config.power_enable = false;
        self.cfg_writer.config.pos_power_enable = false;
        self.cfg_writer.write();
        busy_delay(10 * 240);
        self.cfg_writer.config.neg_power_enable = false;
        self.cfg_writer.write();
        busy_delay(100 * 240);
        self.cfg_writer.config.power_disable = true;
        self.cfg_writer.config.mode = false;
        // self.cfg_writer.write();
        self.cfg_writer.config.stv = false;
        self.cfg_writer.write();
    }

    pub(crate) fn frame_start(&mut self) -> crate::Result<()> {
        self.cfg_writer.config.mode = true;
        self.cfg_writer.write();

        self.rmt.pulse(10, 10, true)?;

        self.cfg_writer.config.stv = false;
        self.cfg_writer.write();

        self.rmt.pulse(10000, 1000, false)?;
        self.cfg_writer.config.stv = true;
        self.cfg_writer.write();
        // self.rmt.pulse(0, 100, true)?;
        self.rmt.pulse(10, 10, true)?;
        self.rmt.pulse(10, 10, true)?;
        self.rmt.pulse(10, 10, true)?;
        self.rmt.pulse(10, 10, true)?;

        self.cfg_writer.config.output_enable = true;
        self.cfg_writer.write();
        self.rmt.pulse(10, 10, true)?;

        Ok(())
    }

    pub(crate) fn latch_row(&mut self) {
        self.cfg_writer.config.latch_enable = true;
        self.cfg_writer.write();

        self.cfg_writer.config.latch_enable = false;
        self.cfg_writer.write();
    }

    pub(crate) fn skip(&mut self) -> crate::Result<()> {
        self.rmt.pulse(45, 5, false)?;
        Ok(())
    }

    pub(crate) fn output_row(&mut self, output_time: u16) -> crate::Result<()> {
        self.latch_row();
        self.rmt.pulse(output_time, 50, false)?;
        let i8080 = self.i8080.take().ok_or(crate::Error::Unknown)?;
        let dma_buf = self.dma_buf.take().ok_or(crate::Error::Unknown)?;
        let tx = i8080
            .send(Command::<u8>::One(0), 0, dma_buf)
            .map_err(|(err, i8080, buf)| {
                self.dma_buf = Some(buf);
                self.i8080 = Some(i8080);
                crate::Error::Dma(err)
            })?;
        let (r, i8080, dma_buf) = tx.wait();
        r.map_err(crate::Error::Dma)?;
        self.i8080 = Some(i8080);
        self.dma_buf = Some(dma_buf);

        Ok(())
    }

    pub(crate) fn frame_end(&mut self) -> crate::Result<()> {
        self.cfg_writer.config.output_enable = false;
        self.cfg_writer.write();
        self.cfg_writer.config.mode = true;
        self.cfg_writer.write();
        self.rmt.pulse(10, 10, true)?;
        self.rmt.pulse(10, 10, true)?;

        Ok(())
    }

    pub(crate) fn set_buffer(&mut self, data: &[u8]) -> crate::Result<()> {
        let mut dma_buf = self.dma_buf.take().ok_or(crate::Error::Unknown)?;
        dma_buf.as_mut_slice().fill(0);
        dma_buf.as_mut_slice()[..data.len()].copy_from_slice(data);
        self.dma_buf = Some(dma_buf);
        Ok(())
    }
}

#[inline(always)]
fn busy_delay(wait_cycles: u32) {
    let target = cycles() + wait_cycles as u64;
    while cycles() < target {}
}

#[inline(always)]
fn cycles() -> u64 {
    esp_hal::xtensa_lx::timer::get_cycle_count() as u64
}
