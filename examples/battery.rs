#![no_std]
#![no_main]

extern crate alloc;
extern crate lilygo_epd47;

use core::format_args;

use embedded_graphics::prelude::*;
use embedded_graphics_core::pixelcolor::{Gray4, GrayColor};
use esp_backtrace as _;
use esp_hal::{
    clock::ClockControl,
    delay::Delay,
    gpio::Io,
    peripherals::Peripherals,
    prelude::*,
    system::SystemControl,
};
use lilygo_epd47::{pin_config, Battery, Display, DrawMode};
use u8g2_fonts::FontRenderer;

static FONT: FontRenderer = FontRenderer::new::<u8g2_fonts::fonts::u8g2_font_spleen32x64_mr>();

#[entry]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();

    let peripherals = Peripherals::take();
    let system = SystemControl::new(peripherals.SYSTEM);
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze();
    let io = Io::new(peripherals.GPIO, peripherals.IO_MUX);

    // Create PSRAM allocator
    esp_alloc::psram_allocator!(peripherals.PSRAM, esp_hal::psram);

    let mut display = Display::new(
        pin_config!(io),
        peripherals.DMA,
        peripherals.LCD_CAM,
        peripherals.RMT,
        &clocks,
    );

    let mut battery = Battery::new(io.pins.gpio14, peripherals.ADC2);

    let delay = Delay::new(&clocks);

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
