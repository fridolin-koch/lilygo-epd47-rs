#![no_std]
#![no_main]

// Adapted from https://github.com/Xinyuan-LilyGO/LilyGo-EPD47/blob/master/examples/screen_repair/screen_repair.ino

extern crate lilygo_epd47;

use esp_backtrace as _;
use esp_hal::{clock::CpuClock, delay::Delay, main, psram::PsramConfig};
use esp_println::{logger::init_logger_from_env, println};
use lilygo_epd47::{pin_config, Display};

esp_bootloader_esp_idf::esp_app_desc!();

#[main]
fn main() -> ! {
    init_logger_from_env();

    let config = esp_hal::Config::default()
        .with_cpu_clock(CpuClock::max())
        .with_psram(PsramConfig::default());
    let peripherals = esp_hal::init(config);

    // Create PSRAM allocator
    esp_alloc::psram_allocator!(peripherals.PSRAM, esp_hal::psram);

    println!("initializing display...");

    let mut display = Display::new(
        pin_config!(peripherals),
        peripherals.DMA_CH0,
        peripherals.LCD_CAM,
        peripherals.RMT,
    )
    .expect("Failed to initialize display");

    let delay = Delay::new();
    println!("set power on...");
    display.power_on();
    println!("delay 10ms...");
    delay.delay_millis(10);
    println!("start repair...");
    display.repair(delay).unwrap();
    println!("set power off...");
    display.power_off();

    loop {}
}
