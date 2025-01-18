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
//!             data0: io.GPIO6,
//!             data1: io.GPIO7,
//!             data2: io.GPIO4,
//!             data3: io.GPIO5,
//!             data4: io.GPIO2,
//!             data5: io.GPIO3,
//!             data6: io.GPIO8,
//!             data7: io.GPIO1,
//!             cfg_data: io.GPIO13,
//!             cfg_clk: io.GPIO12,
//!             cfg_str: io.GPIO0,
//!             lcd_dc: io.GPIO40,
//!             lcd_wrx: io.GPIO41,
//!             rmt: io.GPIO38,
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
    /// Pass-through
    I8080(esp_hal::lcd_cam::lcd::i8080::ConfigError),
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
