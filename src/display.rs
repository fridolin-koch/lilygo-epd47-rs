use alloc::{boxed::Box, vec, vec::Vec};

use esp_hal::{delay::Delay, peripheral::Peripheral, peripherals};

use crate::{ed047tc1, Error, Result};

const CONTRAST_CYCLES_4BPP: &[u16; 15] = &[
    30, 30, 20, 20, 30, 30, 30, 40, 40, 50, 50, 50, 100, 200, 300,
];
const CONTRAST_CYCLES_4BPP_WHITE: &[u16; 15] =
    &[10, 10, 8, 8, 8, 8, 8, 10, 10, 15, 15, 20, 20, 100, 300];

#[derive(Clone, Copy, Debug)]
pub enum DrawMode {
    BlackOnWhite,
    WhiteOnWhite,
    WhiteOnBlack,
}

#[derive(Clone, Copy, Debug)]
pub struct Rectangle {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl DrawMode {
    fn lut_default(&self) -> u8 {
        match self {
            Self::BlackOnWhite => 0x55,
            Self::WhiteOnBlack | Self::WhiteOnWhite => 0xAA,
        }
    }

    fn contrast_cycles(&self) -> &[u16; 15] {
        match self {
            Self::WhiteOnBlack => CONTRAST_CYCLES_4BPP_WHITE,
            Self::BlackOnWhite | Self::WhiteOnWhite => CONTRAST_CYCLES_4BPP,
        }
    }
}

const TAINTED_ROWS_SIZE: usize = Display::HEIGHT as usize / 8 + 1;
const FRAMEBUFFER_SIZE: usize = (Display::WIDTH / 2) as usize * Display::HEIGHT as usize;
const BYTES_PER_LINE: usize = Display::WIDTH as usize / 4;
const LINE_BYTES_4BPP: usize = Display::WIDTH as usize / 2;

pub struct Display<'a> {
    epd: ed047tc1::ED047TC1<'a>,
    skipping: u16,
    framebuffer: Box<[u8; FRAMEBUFFER_SIZE]>,
    tainted_rows: [u8; TAINTED_ROWS_SIZE],
}

impl<'a> Display<'a> {
    /// Width of the screen.
    pub const WIDTH: u16 = 960;
    /// Height of the screen
    pub const HEIGHT: u16 = 540;
    /// Bounding Box of the screen.
    pub const BOUNDING_BOX: Rectangle = Rectangle {
        x: 0,
        y: 0,
        width: Self::WIDTH,
        height: Self::HEIGHT,
    };
    pub fn new(
        pins: ed047tc1::PinConfig,
        dma: impl Peripheral<P = peripherals::DMA> + 'a,
        lcd_cam: impl Peripheral<P = peripherals::LCD_CAM> + 'a,
        rmt: impl Peripheral<P = peripherals::RMT> + 'a,
    ) -> Result<Self> {
        Ok(Display {
            epd: ed047tc1::ED047TC1::new(pins, dma, lcd_cam, rmt)?,
            skipping: 0,
            framebuffer: Box::new([0xFF; FRAMEBUFFER_SIZE]),
            tainted_rows: [0; TAINTED_ROWS_SIZE],
        })
    }

    /// Turn the display on.
    pub fn power_on(&mut self) {
        self.epd.power_on()
    }

    /// Turn the display off.
    pub fn power_off(&mut self) {
        self.epd.power_off()
    }

    /// Sets a single pixel in the framebuffer without updating the display.
    ///
    /// If the provided coordinates are outside the screen, this method returns
    /// [Error::OutOfBounds]. If the provided color is greater than 0x0F,
    /// this method returns [Error::InvalidColor].
    pub fn set_pixel(&mut self, x: u16, y: u16, color: u8) -> Result<()> {
        if x > Self::WIDTH || y > Self::HEIGHT {
            return Err(Error::OutOfBounds);
        }
        if color > 0x0F {
            return Err(Error::InvalidColor);
        }
        // Calculate the index in the framebuffer.
        let index: usize = x as usize / 2 + y as usize * (Self::WIDTH as usize / 2);
        let value = self.framebuffer[index];
        if x % 2 == 1 {
            self.framebuffer[index] = (value & 0x0F) | ((color << 4) & 0xF0);
        } else {
            self.framebuffer[index] = (value & 0xF0) | (color & 0x0F);
        }
        // taint row
        let tainted_index = y as usize / TAINTED_ROWS_SIZE;
        self.tainted_rows[tainted_index] |= 1 << ((y - (tainted_index as u16 * 8)) % 8);
        Ok(())
    }

    /// Fill the whole framebuffer with the same color.
    pub fn fill(&mut self, color: u8) -> Result<()> {
        if color > 0x0F {
            return Err(Error::InvalidColor);
        }
        self.framebuffer.fill(color << 4 | color);
        self.tainted_rows.fill(0xFF);
        Ok(())
    }

    /// Flush updates the display with the contents of the framebuffer. The
    /// method clears the framebuffer. The provided mode should match the
    /// contents of your framebuffer.
    pub fn flush(&mut self, mode: DrawMode) -> Result<()> {
        self.draw(mode)?;
        self.tainted_rows.fill(0);
        self.framebuffer.fill(0xFF);
        Ok(())
    }

    /// Clears the screen.
    pub fn clear(&mut self) -> Result<()> {
        self.clear_area(Self::BOUNDING_BOX)
    }

    /// Performs the screen repair routine as described here
    /// https://github.com/Xinyuan-LilyGO/LilyGo-EPD47/blob/master/examples/screen_repair/screen_repair.ino
    pub fn repair(&mut self, delay: Delay) -> Result<()> {
        self.clear()?;
        for _ in 0..20 {
            self.push_pixels(Self::BOUNDING_BOX, 50, 0)?;
            delay.delay_millis(500);
        }
        self.clear()?;
        for _ in 0..40 {
            self.push_pixels(Self::BOUNDING_BOX, 50, 1)?;
            delay.delay_millis(500);
        }
        self.clear()
    }

    pub fn clear_area(&mut self, area: Rectangle) -> Result<()> {
        self.clear_cycles(area, 4, 50)
    }

    fn clear_cycles(&mut self, area: Rectangle, cycles: u16, cycle_time: u16) -> Result<()> {
        for _ in 0..cycles {
            for _ in 0..4 {
                self.push_pixels(area, cycle_time, 0)?;
            }
            for _ in 0..4 {
                self.push_pixels(area, cycle_time, 1)?;
            }
        }
        Ok(())
    }

    fn push_pixels(&mut self, area: Rectangle, time: u16, color: u16) -> Result<()> {
        let mut row = [0u8; BYTES_PER_LINE];

        for i in 0..area.width {
            let pos = i + area.x % 4;
            let mask = match color {
                1 => 0b10101010,
                _ => 0b01010101,
            } & (0b00000011 << (2 * (pos % 4)));
            row[(area.x / 4 + pos / 4) as usize] |= mask;
        }
        line_buffer_reorder(&mut row);
        self.epd.frame_start()?;

        for i in 0..Self::WIDTH {
            // before are of interest: skip
            if i < area.y {
                self.row_skip(time)?;
                continue;
            }
            if i == area.y {
                self.epd.set_buffer(&row)?;
                self.row_write(time)?;
                continue;
            }
            if i >= area.y + area.height {
                self.row_skip(time)?;
                continue;
            }
            self.row_write(time)?;
        }
        self.row_write(time)?;
        self.epd.frame_end()?;

        Ok(())
    }

    fn row_skip(&mut self, output_time: u16) -> Result<()> {
        match self.skipping {
            0 => {
                self.epd.set_buffer(&[0u8; BYTES_PER_LINE])?;
                self.epd.output_row(output_time)?;
            }
            i if i < 2 => {
                self.epd.output_row(10)?;
            }
            _ => {
                self.epd.skip()?;
            }
        }
        self.skipping += 1;

        Ok(())
    }

    fn row_write(&mut self, output_time: u16) -> Result<()> {
        self.skipping = 0;
        self.epd.output_row(output_time)?;

        Ok(())
    }

    fn is_tainted(&self, row: u16) -> bool {
        let index = row as usize / TAINTED_ROWS_SIZE;
        self.tainted_rows[index] & (1 << ((row - (index as u16 * 8)) % 8)) != 0
    }

    const DRAW_IMAGE_FRAME_COUNT: usize = 15;
    fn draw(&mut self, mode: DrawMode) -> Result<()> {
        // let start = esp_hal::time::current_time();

        // init lut
        let mut lut = vec![mode.lut_default(); 1 << 16];

        for k in 0..Self::DRAW_IMAGE_FRAME_COUNT {
            // update lut
            update_lut(&mut lut, k, mode);
            // start draw
            self.epd.frame_start()?;
            // build line
            for y in 0..Self::HEIGHT {
                if !self.is_tainted(y) {
                    self.epd.skip()?;
                    continue;
                }
                let start = y as usize * LINE_BYTES_4BPP;
                let end = start + LINE_BYTES_4BPP;
                // draw
                let buf = prepare_dma_buffer(&self.framebuffer[start..end], &lut);
                self.epd.set_buffer(buf.as_slice())?;
                self.epd.output_row(mode.contrast_cycles()[k])?;
            }
            if self.skipping == 0 {
                self.row_write(mode.contrast_cycles()[k])?;
            }
            self.epd.frame_end()?;
        }
        // println!(
        //     "draw_fb {}",
        //     (esp_hal::time::current_time() - start).to_millis()
        // );
        Ok(())
    }
}

fn line_buffer_reorder(data: &mut [u8]) {
    // Iterate over the data in chunks of 4 bytes (size of a u32)
    for chunk in data.chunks_exact_mut(4) {
        // Convert the 4-byte chunk to a u32, swap the high and low 16 bits, and then
        // write it back
        let val = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        let swapped = (val >> 16) | ((val & 0x0000FFFF) << 16);
        chunk.copy_from_slice(&swapped.to_le_bytes());
    }
}

fn prepare_dma_buffer(line_data: &[u8], conversion_lut: &[u8]) -> Vec<u8> {
    let mut epd_input = vec![0u8; BYTES_PER_LINE];
    let mut wide_epd_input: Vec<u32> = vec![0u32; Display::WIDTH as usize / 16];

    let line_data_16: Vec<u16> = line_data
        .chunks(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();

    for (j, chunk) in line_data_16.chunks(4).enumerate() {
        if let [v1, v2, v3, v4] = chunk {
            let pixel: u32 = (conversion_lut[*v1 as usize] as u32)
                | (conversion_lut[*v2 as usize] as u32) << 8
                | (conversion_lut[*v3 as usize] as u32) << 16
                | (conversion_lut[*v4 as usize] as u32) << 24;
            wide_epd_input[j] = pixel;
        }
    }

    for (i, &wide_pixel) in wide_epd_input.iter().enumerate() {
        epd_input[i * 4..(i + 1) * 4].copy_from_slice(&wide_pixel.to_le_bytes());
    }

    epd_input
}

fn update_lut(conversion_lut: &mut [u8], k: usize, mode: DrawMode) {
    let k = match mode {
        DrawMode::BlackOnWhite | DrawMode::WhiteOnWhite => Display::DRAW_IMAGE_FRAME_COUNT - k,
        DrawMode::WhiteOnBlack => k,
    };
    // reset the pixels which are not to be lightened / darkened
    // any longer in the current frame
    for l in (k..1 << 16).step_by(16) {
        conversion_lut[l] &= 0xFC;
    }
    for l in ((k << 4)..(1 << 16)).step_by(1 << 8) {
        for p in 0..16 {
            conversion_lut[l + p] &= 0xF3
        }
    }
    for l in ((k << 8)..(1 << 16)).step_by(1 << 12) {
        for p in 0..(1 << 8) {
            conversion_lut[l + p] &= 0xCF
        }
    }
    for l in (k << 12)..((k + 1) << 12) {
        conversion_lut[l] &= 0x3F;
    }
}
