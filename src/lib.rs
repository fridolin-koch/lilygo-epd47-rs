//! Simple driver for the LilyGo T5 4.7 inch E-Ink Display.
//! The driver only supports the V2.3 hardware variant (ESP32-S3)
//!
//! This library depends on alloc and requires you to set up an global allocator
//! for the PSRAM.
//!
//!
//! Built using [`esp-hal`] and [`embedded-graphics`]
//!
//! [`esp-hal`]: https://github.com/esp-rs/esp-hal
//! [`embedded-graphics`]: https://docs.rs/embedded-graphics/

//! # Example
//!
//! Simple example that draws a circle to the screen
//!
//! ```rust no_run
//! #![no_std]
//! #![no_main]
//! extern crate alloc;
//!
//! use embedded_graphics::{
//!     prelude::*,
//!     primitives::{Circle, PrimitiveStyle},
//! };
//! use embedded_graphics_core::pixelcolor::{Gray4, GrayColor};
//! use esp_backtrace as _;
//! use esp_hal::{
//!     delay::Delay,
//!     prelude::*,
//! };
//! use lilygo_epd47::{pin_config, Display, DrawMode};
//!
//! #[entry]
//! fn main() -> ! {
//!     let peripherals = esp_hal::init(esp_hal::Config::default());
//!     let delay = Delay::new();
//!     // Create PSRAM allocator
//!     esp_alloc::psram_allocator!(peripherals.PSRAM, esp_hal::psram);
//!     // Initialise the display
//!     let mut display = Display::new(
//!         pin_config!(peripherals),
//!         peripherals.DMA,
//!         peripherals.LCD_CAM,
//!         peripherals.RMT,
//!     )
//!     .expect("Failed to initialize display");
//!     // Turn the display on
//!     display.power_on();
//!     delay.delay_millis(10);
//!     // clear the screen
//!     display.clear().unwrap();
//!     // Draw a circle with a 3px wide stroke in the center of the screen
//!     // TODO: Adapt to your requirements (i.e. draw whatever you want)
//!     Circle::new(display.bounding_box().center() - Point::new(100, 100), 200)
//!         .into_styled(PrimitiveStyle::with_stroke(Gray4::BLACK, 3))
//!         .draw(&mut display)
//!         .unwrap();
//!     // Flush the framebuffer to the screen
//!     display.flush(DrawMode::BlackOnWhite).unwrap();
//!     // Turn the display of again
//!     display.power_off();
//!     // do nothing
//!     loop {}
//! }
#![no_std]

extern crate alloc;

pub mod display;

#[cfg(feature = "embedded-graphics")]
pub mod graphics;

mod battery;
mod ed047tc1;
mod rmt;

/// Errors
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Error {
    /// Pass-through
    Rmt(esp_hal::rmt::Error),
    /// Pass-through
    Dma(esp_hal::dma::DmaError),
    /// Pass-through
    DmaBuffer(esp_hal::dma::DmaBufError),
    /// Provided pixel coordinates exceed the display boundary.
    OutOfBounds,
    /// Provided color exceeds the allowed range of 0x0 - 0x0F
    InvalidColor,
    Unknown,
}

type Result<T> = core::result::Result<T, Error>;

pub use crate::{
    battery::Battery,
    display::{Display, DrawMode},
    ed047tc1::PinConfig,
};

/// Convenience macro to build the pin config struct.
#[macro_export]
macro_rules! pin_config {
    ($($name:ident),*) => {
        $(
            #[allow(unused_mut)]
            lilygo_epd47::PinConfig {
                data0: $name.GPIO6,
                data1: $name.GPIO7,
                data2: $name.GPIO4,
                data3: $name.GPIO5,
                data4: $name.GPIO2,
                data5: $name.GPIO3,
                data6: $name.GPIO8,
                data7: $name.GPIO1,
                cfg_data: $name.GPIO13,
                cfg_clk: $name.GPIO12,
                cfg_str: $name.GPIO0,
                lcd_dc: $name.GPIO40,
                lcd_wrx: $name.GPIO41,
                rmt: $name.GPIO38,
            }
        )*
    }
}
