#![no_std]
#![no_main]

extern crate alloc;
extern crate lilygo_epd47;

use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Circle, PrimitiveStyle};
use embedded_graphics_core::pixelcolor::{Gray4, GrayColor};
use esp_backtrace as _;
use esp_hal::clock::ClockControl;
use esp_hal::delay::Delay;
use esp_hal::gpio::Io;
use esp_hal::peripherals::Peripherals;
use esp_hal::prelude::*;
use esp_hal::system::SystemControl;
use lilygo_epd47::{Display, DrawMode};

#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take();
    let system = SystemControl::new(peripherals.SYSTEM);
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze();
    let delay = Delay::new(&clocks);
    // Create PSRAM allocator
    esp_alloc::psram_allocator!(peripherals.PSRAM, esp_hal::psram);
    // Initialise the display
    let mut display = Display::new(
        Io::new(peripherals.GPIO, peripherals.IO_MUX),
        peripherals.DMA,
        peripherals.LCD_CAM,
        peripherals.RMT,
        &clocks,
    );
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
