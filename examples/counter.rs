#![no_std]
#![no_main]

extern crate alloc;
extern crate lilygo_epd47;

use core::format_args;

use embedded_graphics::prelude::*;
use embedded_graphics_core::pixelcolor::{Gray4, GrayColor};
use esp_backtrace as _;
use esp_hal::{delay::Delay, main};
use lilygo_epd47::{pin_config, Display, DrawMode};
use u8g2_fonts::FontRenderer;

static FONT: FontRenderer = FontRenderer::new::<u8g2_fonts::fonts::u8g2_font_spleen32x64_mr>();

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
    .expect("to initialize display");

    let delay = Delay::new();

    delay.delay_millis(100);
    display.power_on();
    delay.delay_millis(10);
    display.clear().expect("to clear screen");

    let mut counter = 0;
    loop {
        let rect = FONT
            .render_aligned(
                format_args!("{}s", counter),
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
            .expect("to render font in framebuffer");

        display
            .flush(DrawMode::BlackOnWhite)
            .expect("to flush to display");
        counter += 1;
        delay.delay_millis(1000);
        // clear rect
        if let Some(rect) = rect {
            display
                .fill_solid(&rect, Gray4::WHITE)
                .expect("to draw rectangle to framebuffer");
            display
                .flush(DrawMode::WhiteOnBlack)
                .expect("to flush to display");
        }
    }
}
