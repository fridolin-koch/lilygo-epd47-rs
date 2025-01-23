use embedded_graphics_core::{pixelcolor::Gray4, prelude::*};

use crate::{display::Display, Error};

impl<'a> DrawTarget for Display<'a> {
    type Color = Gray4;

    type Error = Error;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(coord, color) in pixels.into_iter() {
            let result = self.set_pixel(coord.x as u16, coord.y as u16, color.luma());
            if matches!(result, Err(Error::OutOfBounds)) {
                continue;
            }
            result?;
        }
        Ok(())
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        self.fill(color.luma())
    }
}

impl<'a> OriginDimensions for Display<'a> {
    fn size(&self) -> Size {
        Size::new(Self::WIDTH as u32, Self::HEIGHT as u32)
    }
}

impl From<embedded_graphics_core::primitives::Rectangle> for crate::display::Rectangle {
    fn from(val: embedded_graphics_core::primitives::Rectangle) -> Self {
        crate::display::Rectangle {
            x: val.top_left.x as u16,
            y: val.top_left.y as u16,
            width: val.size.width as u16,
            height: val.size.height as u16,
        }
    }
}
