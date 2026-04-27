#![no_std]
#![no_main]

extern crate alloc;
extern crate lilygo_epd47;

use embedded_graphics::{
    prelude::*,
    primitives::{Circle, PrimitiveStyle},
};
use embedded_graphics_core::pixelcolor::{Gray4, GrayColor};
use esp_backtrace as _;
use esp_hal::{delay::Delay, main};
use lilygo_epd47::{pin_config, Display, DrawMode};
use log::*;

esp_bootloader_esp_idf::esp_app_desc!();

#[main]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();

    let peripherals = esp_hal::init(esp_hal::Config::default());
    let delay = Delay::new();

    info!("Create PSRAM allocator");
    let psram_config = esp_hal::psram::PsramConfig {
        mode: esp_hal::psram::PsramMode::OctalSpi,
        ..Default::default()
    };
    esp_alloc::psram_allocator!(peripherals.PSRAM, esp_hal::psram, psram_config);

    info!("Initialise the display");
    let mut display = Display::new(
        pin_config!(peripherals),
        peripherals.DMA_CH0,
        peripherals.LCD_CAM,
        peripherals.RMT,
    )
    .expect("to initialize display");

    info!("Turn the display on");
    display.power_on();
    delay.delay_millis(10);

    info!("clear the screen");
    display.clear().expect("to clear screen");

    info!("Draw a circle with a 3px wide stroke in the center of the screen");
    Circle::new(display.bounding_box().center() - Point::new(100, 100), 200)
        .into_styled(PrimitiveStyle::with_stroke(Gray4::BLACK, 3))
        .draw(&mut display)
        .expect("to draw in the framebuffer");
    info!("Flush the framebuffer to the screen");
    display
        .flush(DrawMode::BlackOnWhite)
        .expect("to flush to display");

    info!("Turn the display off again");
    display.power_off();

    info!("do nothing");
    loop {}
}
