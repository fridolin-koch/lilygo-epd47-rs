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
//!
//! extern crate alloc;
//! extern crate lilygo_epd47;
//!
//! use embedded_graphics::prelude::*;
//! use embedded_graphics::primitives::{Circle, PrimitiveStyle};
//! use embedded_graphics_core::pixelcolor::{Gray4, GrayColor};
//! use esp_backtrace as _;
//! use esp_hal::clock::ClockControl;
//! use esp_hal::delay::Delay;
//! use esp_hal::gpio::Io;
//! use esp_hal::peripherals::Peripherals;
//! use esp_hal::prelude::*;
//! use esp_hal::system::SystemControl;
//! use lilygo_epd47::{Display, DrawMode, PinConfig};
//!
//! #[entry]
//! fn main() -> ! {
//!     let peripherals = Peripherals::take();
//!     let system = SystemControl::new(peripherals.SYSTEM);
//!     let clocks = ClockControl::boot_defaults(system.clock_control).freeze();
//!     let delay = Delay::new(&clocks);
//!     let io = Io::new(peripherals.GPIO, peripherals.IO_MUX);
//!     // Create PSRAM allocator
//!     esp_alloc::psram_allocator!(peripherals.PSRAM, esp_hal::psram);
//!     // Initialise the display
//!     let mut display = Display::new(
//!         PinConfig {
//!             data0: io.pins.gpio6,
//!             data1: io.pins.gpio7,
//!             data2: io.pins.gpio4,
//!             data3: io.pins.gpio5,
//!             data4: io.pins.gpio2,
//!             data5: io.pins.gpio3,
//!             data6: io.pins.gpio8,
//!             data7: io.pins.gpio1,
//!             cfg_data: io.pins.gpio13,
//!             cfg_clk: io.pins.gpio12,
//!             cfg_str: io.pins.gpio0,
//!             lcd_dc: io.pins.gpio40,
//!             lcd_wrx: io.pins.gpio41,
//!             rmt: io.pins.gpio38,
//!         },
//!         peripherals.DMA,
//!         peripherals.LCD_CAM,
//!         peripherals.RMT,
//!         &clocks,
//!     );
//!     // Turn the display on
//!     display.power_on();
//!     delay.delay_millis(10);
//!     // clear the screen
//!     display.clear().unwrap();
//!     // Draw a circle with a 3px wide stroke in the center of the screen
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

mod ed047tc1;
mod rmt;

/// Errors
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Error {
    /// Pass-through
    Rmt(esp_hal::rmt::Error),
    /// Pass-through
    Dma(esp_hal::dma::DmaError),
    /// Provided pixel coordinates exceed the display boundary.
    OutOfBounds,
    /// Provided color exceeds the allowed range of 0x0 - 0x0F
    InvalidColor,
    Unknown,
}

type Result<T> = core::result::Result<T, Error>;

pub use crate::{
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
                data0: $name.pins.gpio6,
                data1: $name.pins.gpio7,
                data2: $name.pins.gpio4,
                data3: $name.pins.gpio5,
                data4: $name.pins.gpio2,
                data5: $name.pins.gpio3,
                data6: $name.pins.gpio8,
                data7: $name.pins.gpio1,
                cfg_data: $name.pins.gpio13,
                cfg_clk: $name.pins.gpio12,
                cfg_str: $name.pins.gpio0,
                lcd_dc: $name.pins.gpio40,
                lcd_wrx: $name.pins.gpio41,
                rmt: $name.pins.gpio38,
            }
        )*
    }
}
