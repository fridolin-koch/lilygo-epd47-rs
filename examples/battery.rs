#![no_std]
#![no_main]

extern crate alloc;
extern crate lilygo_epd47;

use core::format_args;

use embedded_graphics::prelude::*;
use embedded_graphics_core::pixelcolor::{Gray4, GrayColor};
use esp_backtrace as _;
use esp_hal::{delay::Delay, gpio::Io, prelude::*};
use lilygo_epd47::{pin_config, Battery, Display, DrawMode};
use u8g2_fonts::FontRenderer;

static FONT: FontRenderer = FontRenderer::new::<u8g2_fonts::fonts::u8g2_font_spleen32x64_mr>();

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

    let mut battery = Battery::new(peripherals.GPIO14, peripherals.ADC2);

    let delay = Delay::new();

    display.power_on();
    delay.delay_millis(10);

    loop {
        display.clear().unwrap();
        FONT.render_aligned(
            format_args!("Voltage: {}V", battery.read()),
            Point::new(
                display.bounding_box().center().x,
                display.bounding_box().center().y,
            ),
            u8g2_fonts::types::VerticalPosition::Baseline,
            u8g2_fonts::types::HorizontalAlignment::Center,
            u8g2_fonts::types::FontColor::WithBackground {
                fg: Gray4::BLACK,
                bg: Gray4::WHITE,
            },
            &mut display,
        )
        .unwrap();

        display.flush(DrawMode::BlackOnWhite).unwrap();
        delay.delay_millis(5000);
    }
}
