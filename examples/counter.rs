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
use lilygo_epd47::{Display, DrawMode, PinConfig};
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
        PinConfig {
            data0: io.pins.gpio6,
            data1: io.pins.gpio7,
            data2: io.pins.gpio4,
            data3: io.pins.gpio5,
            data4: io.pins.gpio2,
            data5: io.pins.gpio3,
            data6: io.pins.gpio8,
            data7: io.pins.gpio1,
            cfg_data: io.pins.gpio13,
            cfg_clk: io.pins.gpio12,
            cfg_str: io.pins.gpio0,
            lcd_dc: io.pins.gpio40,
            lcd_wrx: io.pins.gpio41,
            rmt: io.pins.gpio38,
        },
        peripherals.DMA,
        peripherals.LCD_CAM,
        peripherals.RMT,
        &clocks,
    );

    let delay = Delay::new(&clocks);

    delay.delay_millis(100);
    display.power_on();
    delay.delay_millis(10);
    display.clear().unwrap();

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
            .unwrap();

        display.flush(DrawMode::BlackOnWhite).unwrap();
        counter += 1;
        delay.delay_millis(1000);
        // clear rect
        if let Some(rect) = rect {
            display.fill_solid(&rect, Gray4::WHITE).unwrap();
            display.flush(DrawMode::WhiteOnBlack).unwrap();
        }
    }
}
