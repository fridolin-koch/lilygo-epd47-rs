#![no_std]
#![no_main]

// Adapted from https://github.com/Xinyuan-LilyGO/LilyGo-EPD47/blob/master/examples/screen_repair/screen_repair.ino

extern crate lilygo_epd47;

use esp_backtrace as _;
use esp_hal::{delay::Delay, prelude::*};
use lilygo_epd47::{pin_config, Display};

#[entry]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    // Create PSRAM allocator
    esp_alloc::psram_allocator!(peripherals.PSRAM, esp_hal::psram);

    esp_println::logger::init_logger_from_env();

    let mut display = Display::new(
        pin_config!(peripherals),
        peripherals.DMA,
        peripherals.LCD_CAM,
        peripherals.RMT,
    )
    .expect("Failed to initialize display");

    let delay = Delay::new();
    display.power_on();
    delay.delay_millis(10);
    display.repair(delay).unwrap();
    display.power_off();

    loop {}
}
