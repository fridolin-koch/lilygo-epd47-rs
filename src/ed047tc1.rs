use esp_hal::{
    dma::DmaTxBuf,
    dma_tx_buffer,
    gpio::{Level, Output, OutputConfig, OutputPin},
    lcd_cam::{
        lcd::{i8080, i8080::Command},
        LcdCam,
    },
    peripherals,
    rmt::PulseCode,
    time::Rate,
    Blocking,
};

use crate::rmt;

macro_rules! pulse {
    ($high:expr, $low:expr) => {
        if $high > 0 {
            [
                PulseCode::new(Level::High, $high, Level::Low, $low),
                PulseCode::end_marker(),
            ]
        } else {
            [
                PulseCode::new(Level::High, $low, Level::Low, 0),
                PulseCode::end_marker(),
            ]
        }
    };
}

const DMA_BUFFER_SIZE: usize = 248;

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
    fn new(data: impl OutputPin + 'a, clk: impl OutputPin + 'a, str: impl OutputPin + 'a) -> Self {
        ConfigWriter {
            pin_data: Output::new(data, Level::High, OutputConfig::default()),
            pin_clk: Output::new(clk, Level::High, OutputConfig::default()),
            pin_str: Output::new(str, Level::Low, OutputConfig::default()),
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

pub struct PinConfig<'a> {
    pub data0: peripherals::GPIO8<'a>,
    pub data1: peripherals::GPIO1<'a>,
    pub data2: peripherals::GPIO2<'a>,
    pub data3: peripherals::GPIO3<'a>,
    pub data4: peripherals::GPIO4<'a>,
    pub data5: peripherals::GPIO5<'a>,
    pub data6: peripherals::GPIO6<'a>,
    pub data7: peripherals::GPIO7<'a>,
    pub cfg_data: peripherals::GPIO13<'a>,
    pub cfg_clk: peripherals::GPIO12<'a>,
    pub cfg_str: peripherals::GPIO0<'a>,
    pub lcd_dc: peripherals::GPIO40<'a>,
    pub lcd_wrx: peripherals::GPIO41<'a>,
    pub rmt: peripherals::GPIO38<'a>,
}

pub(crate) struct ED047TC1<'a> {
    i8080: Option<i8080::I8080<'a, Blocking>>,
    cfg_writer: ConfigWriter<'a>,
    rmt: rmt::Rmt<'a>,
    dma_buf: Option<DmaTxBuf>,
}

impl<'a> ED047TC1<'a> {
    pub(crate) fn new(
        pins: PinConfig<'a>,
        dma: peripherals::DMA_CH0<'a>,
        lcd_cam: peripherals::LCD_CAM<'a>,
        rmt: peripherals::RMT<'a>,
    ) -> crate::Result<Self> {
        // init lcd
        let lcd_cam = LcdCam::new(lcd_cam);

        // init panel config writer (?)
        let mut cfg_writer = ConfigWriter::new(pins.cfg_data, pins.cfg_clk, pins.cfg_str);
        cfg_writer.write();

        let dma_buf = Some(dma_tx_buffer!(DMA_BUFFER_SIZE).map_err(crate::Error::DmaBuffer)?);

        let config = i8080::Config::default()
            .with_frequency(Rate::from_mhz(10))
            .with_cd_idle_edge(false)
            .with_cd_cmd_edge(true)
            .with_cd_dummy_edge(false)
            .with_cd_data_edge(false);
        let ctrl = ED047TC1 {
            i8080: Some(
                i8080::I8080::new(lcd_cam.lcd, dma, config)
                    .expect("to create i8080 device")
                    .with_dc(pins.lcd_dc)
                    .with_wrx(pins.lcd_wrx)
                    .with_data0(pins.data6)
                    .with_data1(pins.data7)
                    .with_data2(pins.data4)
                    .with_data3(pins.data5)
                    .with_data4(pins.data2)
                    .with_data5(pins.data3)
                    .with_data6(pins.data0)
                    .with_data7(pins.data1),
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
        self.cfg_writer.config.pos_power_enable = false;
        self.cfg_writer.write();
        busy_delay(10 * 240);
        self.cfg_writer.config.neg_power_enable = false;
        self.cfg_writer.write();
        busy_delay(100 * 240);
        self.cfg_writer.config.power_disable = true;
        self.cfg_writer.write();
        self.cfg_writer.config.stv = false;
        self.cfg_writer.write();
    }

    pub(crate) fn frame_start(&mut self) -> crate::Result<()> {
        self.cfg_writer.config.mode = true;
        self.cfg_writer.write();

        let data = pulse!(10, 10);
        self.rmt.pulse(&data, true)?;

        self.cfg_writer.config.stv = false;
        self.cfg_writer.write();

        busy_delay(240);
        let data = pulse!(100, 100);
        let rmt_tx = self.rmt.pulse(&data, false)?;

        self.cfg_writer.config.stv = true;
        self.cfg_writer.write();

        if let Some(rmt_tx) = rmt_tx {
            self.rmt.reclaim_channel(rmt_tx)?;
        }

        let data = pulse!(0, 100);
        self.rmt.pulse(&data, true)?;

        self.cfg_writer.config.output_enable = true;
        self.cfg_writer.write();

        let data = pulse!(10, 10);
        self.rmt.pulse(&data, true)?;

        Ok(())
    }

    pub(crate) fn latch_row(&mut self) {
        self.cfg_writer.config.latch_enable = true;
        self.cfg_writer.write();

        self.cfg_writer.config.latch_enable = false;
        self.cfg_writer.write();
    }

    pub(crate) fn skip(&mut self) -> crate::Result<()> {
        let data = pulse!(45, 5);
        self.rmt.pulse(&data, false)?;
        Ok(())
    }

    pub(crate) fn output_row(&mut self, output_time: u16) -> crate::Result<()> {
        self.latch_row();

        let data = pulse!(output_time, 50);
        let rmt_tx = self.rmt.pulse(&data, false)?;
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
        if let Some(rmt_tx) = rmt_tx {
            self.rmt.reclaim_channel(rmt_tx)?;
        }
        r.map_err(crate::Error::Dma)?;
        self.i8080 = Some(i8080);
        self.dma_buf = Some(dma_buf);

        Ok(())
    }

    pub(crate) fn frame_end(&mut self) -> crate::Result<()> {
        self.cfg_writer.config.output_enable = false;
        self.cfg_writer.write();
        self.cfg_writer.config.mode = false;
        self.cfg_writer.write();
        let data = pulse!(10, 10);
        self.rmt.pulse(&data, true)?;
        self.rmt.pulse(&data, true)?;

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
