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
use esp_hal::{delay::Delay, prelude::*};
use lilygo_epd47::{pin_config, Display, DrawMode};

#[entry]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();

    let peripherals = esp_hal::init(esp_hal::Config::default());
    let delay = Delay::new();

    // Create PSRAM allocator
    esp_alloc::psram_allocator!(peripherals.PSRAM, esp_hal::psram);
    // Initialise the display
    let mut display = Display::new(
        pin_config!(peripherals),
        peripherals.DMA,
        peripherals.LCD_CAM,
        peripherals.RMT,
    )
    .expect("Failed to initialize display");
    // Turn the display on
    display.power_on();
    delay.delay_millis(10);
    // clear the screen
    display.clear().unwrap();
    // Draw a circle with a 3px wide stroke in the center of the screen
    Circle::new(display.bounding_box().center() - Point::new(100, 100), 200)
        .into_styled(PrimitiveStyle::with_stroke(Gray4::BLACK, 3))
        .draw(&mut display)
        .unwrap();
    // Flush the framebuffer to the screen
    display.flush(DrawMode::BlackOnWhite).unwrap();
    // Turn the display of again
    display.power_off();
    // do nothing
    loop {}
}
