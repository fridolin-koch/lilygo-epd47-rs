#![no_std]
#![no_main]

extern crate lilygo_epd47;

use embedded_graphics::prelude::*;
use embedded_graphics::primitives::PrimitiveStyleBuilder;
use embedded_graphics_core::geometry::Point;
use embedded_graphics_core::pixelcolor::Gray4;
use embedded_graphics_core::prelude::Dimensions;
use embedded_graphics_core::primitives::Rectangle;
#[allow(unused_imports)]
use esp_backtrace as _;
use esp_hal::clock::ClockControl;
use esp_hal::delay::Delay;
use esp_hal::gpio::Io;
use esp_hal::peripherals::Peripherals;
use esp_hal::prelude::*;
use esp_hal::system::SystemControl;

use lilygo_epd47::{Display, DrawMode};

#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take();
    let system = SystemControl::new(peripherals.SYSTEM);
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze();
    let io = Io::new(peripherals.GPIO, peripherals.IO_MUX);

    // Create PSRAM allocator
    esp_alloc::psram_allocator!(peripherals.PSRAM, esp_hal::psram);

    esp_println::logger::init_logger_from_env();

    let mut display = Display::new(
        io,
        peripherals.DMA,
        peripherals.LCD_CAM,
        peripherals.RMT,
        &clocks,
    );

    let delay = Delay::new(&clocks);
    display.power_on();
    delay.delay_millis(10);
    display.clear().unwrap();

    loop {
        let height = display.bounding_box().size.height / 16;
        for shade in 0x0..0x0F {
            Rectangle::new(
                Point::new(0, height as i32 * shade as i32),
                Size::new(display.bounding_box().size.width, height),
            )
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .fill_color(Gray4::new(shade))
                    .build(),
            )
            .draw(&mut display)
            .unwrap();
        }

        display.flush(DrawMode::BlackOnWhite).unwrap();

        delay.delay_millis(5000);

        display.clear().unwrap();

        let width = display.bounding_box().size.width / 16;
        for shade in 0x0..0x0F {
            Rectangle::new(
                Point::new(width as i32 * shade as i32, 0),
                Size::new(width, display.bounding_box().size.height),
            )
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .fill_color(Gray4::new(shade))
                    .build(),
            )
            .draw(&mut display)
            .unwrap();
        }

        display.flush(DrawMode::BlackOnWhite).unwrap();

        delay.delay_millis(5000);

        display.clear().unwrap();
    }
}
