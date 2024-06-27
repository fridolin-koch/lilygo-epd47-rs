#![no_std]

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;

use embedded_graphics_core::pixelcolor::Gray4;
use embedded_graphics_core::primitives::Rectangle;
use embedded_graphics_core::{pixelcolor::GrayColor, prelude::*};
use esp_hal::clock::Clocks;
use esp_hal::gpio::Io;
use esp_hal::peripheral::Peripheral;
use esp_hal::peripherals;

mod ed047tc1;
mod rmt;
mod waveform;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Error {
    Rmt(esp_hal::rmt::Error),
    Dma(esp_hal::dma::DmaError),
    Unknown,
}

type Result<T> = core::result::Result<T, Error>;

const EPD_WIDTH: u32 = 960;
const EPD_HEIGHT: u32 = 540;

const FRAMEBUFFER_SIZE: usize = (EPD_WIDTH / 2) as usize * EPD_HEIGHT as usize;

const BYTES_PER_LINE: usize = EPD_WIDTH as usize / 4;

const LINE_BYTES_4BPP: usize = EPD_WIDTH as usize / 2;

const CLEAR_BYTE: u8 = 0b10101010;
const DARK_BYTE: u8 = 0b01010101;

const CONTRAST_CYCLES_4BPP: &[u16; 15] = &[
    30, 30, 20, 20, 30, 30, 30, 40, 40, 50, 50, 50, 100, 200, 300,
];
const CONTRAST_CYCLES_4BPP_WHITE: &[u16; 15] =
    &[10, 10, 8, 8, 8, 8, 8, 10, 10, 15, 15, 20, 20, 100, 300];

#[derive(Clone, Copy, Debug)]
enum DrawMode {
    BlackOnWhite,
    WhiteOnWhite,
    WhiteOnBlack,
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

const TAINTED_ROWS_SIZE: usize = EPD_HEIGHT as usize / 8 + 1;

pub struct Display<'a> {
    epd: ed047tc1::ED047TC1<'a>,
    skipping: u8,
    framebuffer: Box<[u8; FRAMEBUFFER_SIZE]>,
    tainted_rows: [u8; TAINTED_ROWS_SIZE],
}

impl<'a> Display<'a> {
    pub fn new(
        io: Io,
        dma: impl Peripheral<P = peripherals::DMA> + 'a,
        lcd_cam: impl Peripheral<P = peripherals::LCD_CAM> + 'a,
        rmt: impl Peripheral<P = peripherals::RMT> + 'a,
        clocks: &'a Clocks,
    ) -> Self {
        Display {
            epd: ed047tc1::ED047TC1::new(io, dma, lcd_cam, rmt, clocks),
            skipping: 0,
            framebuffer: Box::new([0xFF; FRAMEBUFFER_SIZE]),
            tainted_rows: [0; TAINTED_ROWS_SIZE],
        }
    }

    pub fn power_on(&mut self) {
        self.epd.power_on()
    }

    pub fn power_off(&mut self) {
        self.epd.power_off()
    }

    pub fn clear(&mut self) -> Result<()> {
        self.clear_area(self.bounding_box())
    }

    pub fn clear_area(&mut self, area: Rectangle) -> Result<()> {
        self.clear_cycles(area, 4, 50)
    }

    pub fn flush(&mut self) -> Result<()> {
        self.draw_framebuffer(self.bounding_box(), DrawMode::BlackOnWhite)?;
        self.tainted_rows.fill(0);
        Ok(())
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

    pub fn push_pixels(&mut self, area: Rectangle, time: u16, color: u16) -> Result<()> {
        let mut row = [0u8; BYTES_PER_LINE];

        for i in 0..area.size.width as i32 {
            let pos = i + area.top_left.x % 4;
            let mask = match color {
                1 => CLEAR_BYTE,
                _ => DARK_BYTE,
            } & (0b00000011 << (2 * (pos % 4)));
            row[(area.top_left.x / 4 + pos / 4) as usize] |= mask;
        }
        line_buffer_reorder(&mut row);
        self.epd.frame_start()?;

        for i in 0..EPD_HEIGHT as i32 {
            // before are of interest: skip
            if i < area.top_left.y {
                self.row_skip(time)?;
                continue;
            }
            if i == area.top_left.y {
                self.epd.set_buffer(&row);
                self.row_write(time)?;
                continue;
            }
            if i >= area.top_left.y + area.size.height as i32 {
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
                self.epd.set_buffer(&[0u8; BYTES_PER_LINE]);
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

    fn is_tainted(&self, row: u32) -> bool {
        let index = row as usize / TAINTED_ROWS_SIZE;
        self.tainted_rows[index] & (1 << ((row as u8) - (index as u8 * 8)) % 8) != 0
    }

    const DRAW_IMAGE_FRAME_COUNT: usize = 15;
    fn draw_framebuffer(&mut self, area: Rectangle, mode: DrawMode) -> Result<()> {
        // let start = esp_hal::time::current_time();

        // init lut
        let mut lut = vec![mode.lut_default(); 1 << 16];

        for k in 0..Self::DRAW_IMAGE_FRAME_COUNT {
            // update lut

            update_lut(&mut lut, k, mode);

            // start draw
            self.epd.frame_start()?;
            // build line
            for y in 0..EPD_HEIGHT {
                if y < area.top_left.y as u32
                    || y >= area.top_left.y as u32 + area.size.height
                    || !self.is_tainted(y)
                {
                    self.epd.skip()?;
                    continue;
                }
                // full wide draw
                if area.top_left.x == 0 && area.size.width == EPD_WIDTH {
                    // let start = y as usize * LINE_BYTES_4BPP;
                    // let end = start + LINE_BYTES_4BPP;
                    // line_buf.copy_from_slice(&data[start..end]);
                }
                let start = y as usize * LINE_BYTES_4BPP;
                let end = start + LINE_BYTES_4BPP;
                // TODO handle other case

                // draw
                let buf = calc_epd_input_4bpp(&self.framebuffer[start..end], &lut);
                self.epd.set_buffer(buf.as_slice());
                self.epd.output_row(mode.contrast_cycles()[k])?;
            }
            // Since we "pipeline" row output, we still have to latch out the last row.
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

impl<'a> DrawTarget for Display<'a> {
    type Color = Gray4;

    type Error = Error;

    fn draw_iter<I>(&mut self, pixels: I) -> core::result::Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(coord, color) in pixels.into_iter() {
            // Check if the pixel coordinates are out of bounds (negative or greater than
            // (EPD_WIDTH,EPD_HEIGHT)). `DrawTarget` implementation are required to discard any out of bounds
            // pixels without returning an error or causing a panic.
            if let Ok((x @ 0..=EPD_WIDTH, y @ 0..=EPD_HEIGHT)) = coord.try_into() {
                // Calculate the index in the framebuffer.
                let index: u32 = x / 2 + y * (EPD_WIDTH / 2);
                let value = self.framebuffer[index as usize];
                if x % 2 == 1 {
                    self.framebuffer[index as usize] =
                        (value & 0x0F) | ((color.luma() << 4) & 0xF0);
                } else {
                    self.framebuffer[index as usize] = (value & 0xF0) | (color.luma() & 0x0F);
                }
                // taint row
                let tainted_index = y as usize / TAINTED_ROWS_SIZE;
                self.tainted_rows[tainted_index] |= 1 << ((y as u8) - (tainted_index as u8 * 8)) % 8
            }
        }
        Ok(())
    }

    fn clear(&mut self, color: Self::Color) -> core::result::Result<(), Self::Error> {
        let color = match color {
            Self::Color::BLACK => 0,
            Self::Color::WHITE => 1,
            // TODO: not yet supported
            _ => return self.fill_solid(&self.bounding_box(), color),
        };
        self.push_pixels(self.bounding_box(), 50, color)?;
        Ok(())
    }
}

impl<'a> OriginDimensions for Display<'a> {
    fn size(&self) -> Size {
        Size::new(EPD_WIDTH, EPD_HEIGHT)
    }
}

fn line_buffer_reorder(data: &mut [u8]) {
    // Iterate over the data in chunks of 4 bytes (size of a u32)
    for chunk in data.chunks_exact_mut(4) {
        // Convert the 4-byte chunk to a u32, swap the high and low 16 bits, and then write it back
        let val = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        let swapped = (val >> 16) | ((val & 0x0000FFFF) << 16);
        chunk.copy_from_slice(&swapped.to_le_bytes());
    }
}

fn calc_epd_input_4bpp(line_data: &[u8], conversion_lut: &Vec<u8>) -> Vec<u8> {
    let mut epd_input = vec![0u8; BYTES_PER_LINE];
    let mut wide_epd_input: Vec<u32> = vec![0u32; EPD_WIDTH as usize / 16];

    let line_data_16: Vec<u16> = line_data
        .chunks(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();

    for (j, chunk) in line_data_16.chunks(4).enumerate() {
        if let [v1, v2, v3, v4] = chunk {
            let pixel: u32 = (conversion_lut[*v1 as usize] as u32) << 0
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

fn update_lut(conversion_lut: &mut Vec<u8>, k: usize, mode: DrawMode) {
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
