#![no_std]
#![no_main]

extern crate alloc;
extern crate lilygo_epd47;

use embedded_graphics::{
    image::Image,
    prelude::*,
    primitives::{
        Circle,
        PrimitiveStyle,
        PrimitiveStyleBuilder,
        Rectangle,
        StrokeAlignment,
        Triangle,
    },
    text::{Alignment, Text},
};
use embedded_graphics_core::pixelcolor::{Gray4, GrayColor};
use esp_backtrace as _;
use esp_hal::{delay::Delay, prelude::*};
use esp_println::println;
use lilygo_epd47::{pin_config, Display, DrawMode};
use tinybmp::Bmp;
use u8g2_fonts::U8g2TextStyle;

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

    delay.delay_millis(100);
    display.power_on();
    delay.delay_millis(10);
    display.clear().unwrap();

    // bmp drawin test
    let data_logo_rust = include_bytes!("./assets/rust.bmp");
    let data_logo_esphal = include_bytes!("./assets/esp-hal.bmp");

    let bmp_logo_rust = Bmp::from_slice(data_logo_rust).unwrap();
    let bmp_logo_esphal = Bmp::from_slice(data_logo_esphal).unwrap();

    // Create styles used by the drawing operations.
    let thin_stroke = PrimitiveStyle::with_stroke(Gray4::BLACK, 1);
    let thick_stroke = PrimitiveStyle::with_stroke(Gray4::BLACK, 3);
    let border_stroke = PrimitiveStyleBuilder::new()
        .stroke_color(Gray4::BLACK)
        .stroke_width(3)
        .stroke_alignment(StrokeAlignment::Inside)
        .build();
    let fill = PrimitiveStyle::with_fill(Gray4::BLACK);
    let character_style =
        U8g2TextStyle::new(u8g2_fonts::fonts::u8g2_font_spleen32x64_mr, Gray4::BLACK);

    let yoffset = 125;

    // Draw a 3px wide outline around the display.
    display
        .bounding_box()
        .into_styled(border_stroke)
        .draw(&mut display)
        .unwrap();

    let mut x_offset = 342;

    // Draw a triangle.
    Triangle::new(
        Point::new(x_offset, 75 + yoffset),
        Point::new(x_offset + 74, 75 + yoffset),
        Point::new(x_offset + 37, yoffset),
    )
    .into_styled(thin_stroke)
    .draw(&mut display)
    .unwrap();

    x_offset += 100;

    // Draw a filled square
    Rectangle::new(Point::new(x_offset, yoffset), Size::new(72, 72))
        .into_styled(fill)
        .draw(&mut display)
        .unwrap();

    x_offset += 100;

    // Draw a circle with a 3px wide stroke.
    Circle::new(Point::new(x_offset, yoffset), 75)
        .into_styled(thick_stroke)
        .draw(&mut display)
        .unwrap();

    // Draw centered text.
    let text = "embedded-graphics";
    let p = Text::with_alignment(
        text,
        display.bounding_box().center() + Point::new(0, 15),
        character_style,
        Alignment::Center,
    )
    .draw(&mut display)
    .unwrap();

    println!("Point {}", p);

    Image::new(&bmp_logo_rust, Point::new(250, p.y + 30))
        .draw(&mut display)
        .unwrap();

    Image::new(&bmp_logo_esphal, Point::new(510, p.y + 30))
        .draw(&mut display)
        .unwrap();

    display.flush(DrawMode::BlackOnWhite).unwrap();
    display.power_off();

    loop {}
}
