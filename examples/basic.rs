#![no_std]
#![no_main]

extern crate lilygo_epd47;

use embedded_graphics::{
    mono_font::MonoTextStyleBuilder,
    prelude::*,
    primitives::{Circle, Line, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle},
    text::{Baseline, Text, TextStyleBuilder},
};
use embedded_graphics_core::pixelcolor::{Gray4, GrayColor};
use esp_backtrace as _;
use esp_hal::clock::ClockControl;
use esp_hal::delay::Delay;
use esp_hal::gpio::Io;
use esp_hal::peripherals::Peripherals;
use esp_hal::prelude::*;
use esp_hal::psram;
use esp_hal::system::SystemControl;
use esp_println::println;
use u8g2_fonts::FontRenderer;

use lilygo_epd47::Display;

#[global_allocator]
static ALLOCATOR: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

fn init_psram_heap() {
    unsafe {
        ALLOCATOR.init(psram::psram_vaddr_start() as *mut u8, psram::PSRAM_BYTES);
    }
}
// 0b0000000 1 10001011
#[entry]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();

    let peripherals = Peripherals::take();
    let system = SystemControl::new(peripherals.SYSTEM);
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze();
    let io = Io::new(peripherals.GPIO, peripherals.IO_MUX);

    // init psram
    psram::init_psram(peripherals.PSRAM);
    init_psram_heap();

    let mut display = Display::new(
        io,
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
    //display.clear();
    delay.delay_millis(500);
    display.clear().unwrap();
    //display.clear();
    delay.delay_millis(500);

    // draw a analog clock
    let _ = Circle::with_center(Point::new(200, 200), 80)
        .into_styled(PrimitiveStyle::with_stroke(Gray4::BLACK, 2))
        .draw(&mut display);
    let _ = Line::new(Point::new(200, 200), Point::new(30, 40))
        .into_styled(PrimitiveStyle::with_stroke(Gray4::BLACK, 4))
        .draw(&mut display);
    let _ = Line::new(Point::new(200, 200), Point::new(80, 40))
        .into_styled(PrimitiveStyle::with_stroke(Gray4::BLACK, 1))
        .draw(&mut display);

    // draw white on black background
    let style = MonoTextStyleBuilder::new()
        .font(&embedded_graphics::mono_font::ascii::FONT_6X10)
        .text_color(Gray4::BLACK)
        .background_color(Gray4::WHITE)
        .build();
    let text_style = TextStyleBuilder::new().baseline(Baseline::Top).build();

    let _ = Text::with_text_style("It's working-WoB!", Point::new(300, 10), style, text_style)
        .draw(&mut display);

    // use bigger/different font
    let style = MonoTextStyleBuilder::new()
        .font(&embedded_graphics::mono_font::ascii::FONT_10X20)
        .text_color(Gray4::BLACK)
        .background_color(Gray4::WHITE)
        .build();

    let _ = Text::with_text_style(
        "It's working\nWoB!",
        Point::new(300, 300),
        style,
        text_style,
    )
    .draw(&mut display);

    for shade in 0x0..0x0F {
        let style = PrimitiveStyleBuilder::new()
            //.stroke_color(Gray4::BLACK)
            // .stroke_width(4)
            .fill_color(Gray4::new(shade))
            .build();
        Rectangle::new(
            Point::new(200 + 50 * (shade as i32), 100),
            Size::new(50, 50),
        )
        .into_styled(style)
        .draw(&mut display)
        .unwrap();
    }

    for shade in 0x0..0x0F {
        let _ = Line::new(
            Point::new(300 + 15 * (shade as i32), 200),
            Point::new(300 + 15 * (shade as i32), 250),
        )
        .into_styled(PrimitiveStyle::with_stroke(Gray4::BLACK, 1))
        .draw(&mut display);
    }

    display.flush().unwrap();

    let font = FontRenderer::new::<u8g2_fonts::fonts::u8g2_font_spleen32x64_mr>();
    let text = "Embedded Graphics!";

    let rect = font
        .render_aligned(
            text,
            Point::new(display.bounding_box().center().x, 450),
            u8g2_fonts::types::VerticalPosition::Baseline,
            u8g2_fonts::types::HorizontalAlignment::Center,
            u8g2_fonts::types::FontColor::Transparent(Gray4::BLACK),
            &mut display,
        )
        .unwrap();
    println!("Text start. {:?}", rect);

    display.flush().unwrap();

    loop {
        log::info!("Hello world!");
        delay.delay_millis(50000);
    }
}
