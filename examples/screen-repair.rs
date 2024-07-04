#![no_std]
#![no_main]

// Adapted from https://github.com/Xinyuan-LilyGO/LilyGo-EPD47/blob/master/examples/screen_repair/screen_repair.ino

extern crate lilygo_epd47;

use esp_backtrace as _;
use esp_hal::clock::ClockControl;
use esp_hal::delay::Delay;
use esp_hal::gpio::Io;
use esp_hal::peripherals::Peripherals;
use esp_hal::prelude::*;
use esp_hal::system::SystemControl;
use lilygo_epd47::Display;

#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take();
    let system = SystemControl::new(peripherals.SYSTEM);
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze();
    let io = Io::new(peripherals.GPIO, peripherals.IO_MUX);

    // Create PSRAM allocator
    esp_alloc::psram_allocator!(peripherals.PSRAM, esp_hal::psram);

    esp_println::logger::init_logger_from_env();

    let mut display = Display::new(
        io,
        peripherals.DMA,
        peripherals.LCD_CAM,
        peripherals.RMT,
        &clocks,
    );

    let delay = Delay::new(&clocks);
    display.power_on();
    delay.delay_millis(10);
    display.repair(delay).unwrap();
    display.power_off();

    loop {}
}
