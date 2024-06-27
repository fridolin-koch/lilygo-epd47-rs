#![no_std]
#![no_main]

// Adapted from https://github.com/Xinyuan-LilyGO/LilyGo-EPD47/blob/master/examples/screen_repair/screen_repair.ino

extern crate lilygo_epd47;

use embedded_graphics_core::prelude::Dimensions;
#[allow(unused_imports)]
use esp_backtrace as _;
use esp_hal::clock::ClockControl;
use esp_hal::delay::Delay;
use esp_hal::gpio::Io;
use esp_hal::peripherals::Peripherals;
use esp_hal::prelude::*;
use esp_hal::psram;
use esp_hal::system::SystemControl;

use lilygo_epd47::Display;

#[global_allocator]
static ALLOCATOR: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

fn init_psram_heap() {
    unsafe {
        ALLOCATOR.init(psram::psram_vaddr_start() as *mut u8, psram::PSRAM_BYTES);
    }
}

#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take();
    let system = SystemControl::new(peripherals.SYSTEM);
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze();
    let io = Io::new(peripherals.GPIO, peripherals.IO_MUX);

    // init psram
    psram::init_psram(peripherals.PSRAM);
    init_psram_heap();

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
    display.clear().unwrap();
    for _ in 0..20 {
        display.push_pixels(display.bounding_box(), 50, 0).unwrap();
        delay.delay_millis(500);
    }
    display.clear().unwrap();
    for _ in 0..40 {
        display.push_pixels(display.bounding_box(), 50, 1).unwrap();
        delay.delay_millis(500);
    }
    display.clear().unwrap();
    display.power_off();

    loop {}
}
