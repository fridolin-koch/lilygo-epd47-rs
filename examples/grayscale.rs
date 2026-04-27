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
use esp_hal::{delay::Delay, main};
use lilygo_epd47::{pin_config, Display, DrawMode};

esp_bootloader_esp_idf::esp_app_desc!();

#[main]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();

    let config = esp_hal::Config::default();
    let config = config.with_cpu_clock(esp_hal::clock::CpuClock::_240MHz);
    let peripherals = esp_hal::init(config);

    // Create PSRAM allocator
    let psram_config = esp_hal::psram::PsramConfig {
        mode: esp_hal::psram::PsramMode::OctalSpi,
        ..Default::default()
    };
    esp_alloc::psram_allocator!(peripherals.PSRAM, esp_hal::psram, psram_config);

    let mut display = Display::new(
        pin_config!(peripherals),
        peripherals.DMA_CH0,
        peripherals.LCD_CAM,
        peripherals.RMT,
    )
    .expect("to initialize correctly");

    let delay = Delay::new();
    display.power_on();
    delay.delay_millis(10);
    display.clear().expect("to clear display");

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
            .expect("to draw in framebuffer");
        }

        display
            .flush(DrawMode::BlackOnWhite)
            .expect("to flush to display");

        delay.delay_millis(5000);

        display.clear().expect("to clear display");

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
            .expect("to draw in framebuffer");
        }

        display
            .flush(DrawMode::BlackOnWhite)
            .expect("to flush to display");

        delay.delay_millis(5000);

        display.clear().expect("to clear display");
    }
}
