#![no_std]
#![no_main]

extern crate lilygo_epd47;

use embedded_graphics::{prelude::*, primitives::PrimitiveStyleBuilder};
use embedded_graphics_core::{
    geometry::Point,
    pixelcolor::Gray4,
    prelude::Dimensions,
    primitives::Rectangle,
};
#[allow(unused_imports)]
use esp_backtrace as _;
use esp_hal::{delay::Delay, prelude::*};
use lilygo_epd47::{pin_config, Display, DrawMode};

#[entry]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();

    let peripherals = esp_hal::init(esp_hal::Config::default());

    // Create PSRAM allocator
    esp_alloc::psram_allocator!(peripherals.PSRAM, esp_hal::psram);

    let mut display = Display::new(
        pin_config!(peripherals),
        peripherals.DMA,
        peripherals.LCD_CAM,
        peripherals.RMT,
    )
    .expect("Failed to initialize display");

    let delay = Delay::new();
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
