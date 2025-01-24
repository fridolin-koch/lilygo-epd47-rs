#![no_std]
#![no_main]

extern crate alloc;
extern crate lilygo_epd47;

use core::{format_args, time::Duration};

use embedded_graphics::prelude::*;
use embedded_graphics_core::{
    pixelcolor::{Gray4, GrayColor},
    primitives::Rectangle,
};
use esp_backtrace as _;
use esp_hal::{
    delay::Delay,
    prelude::*,
    rtc_cntl::{
        reset_reason,
        sleep::{RtcSleepConfig, TimerWakeupSource},
        wakeup_cause,
        Rtc,
        SocResetReason,
    },
    Cpu,
};
use lilygo_epd47::{pin_config, Display, DrawMode};
use u8g2_fonts::FontRenderer;

static FONT: FontRenderer = FontRenderer::new::<u8g2_fonts::fonts::u8g2_font_spleen16x32_mr>();

#[ram(rtc_fast)]
static mut CYCLE: u16 = 0;

#[ram(rtc_fast)]
static mut LAST_RECT: Rectangle = Rectangle {
    top_left: Point { x: 0, y: 0 },
    size: Size {
        width: 0,
        height: 0,
    },
};

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
    let mut rtc = Rtc::new(peripherals.LPWR);

    let reason = reset_reason(Cpu::ProCpu).unwrap_or(SocResetReason::ChipPowerOn);
    let wake_reason = wakeup_cause();

    // turn screen on
    display.power_on();
    delay.delay_millis(20);
    // clear
    let cycle = unsafe { CYCLE };
    let last_rect = unsafe { LAST_RECT };

    if cycle > 0 && cycle % 5 != 0 {
        display.fill_solid(&last_rect, Gray4::WHITE).unwrap();
        display.flush(DrawMode::WhiteOnBlack).unwrap();
    } else {
        display.clear().unwrap();
    }
    // write out reset and wake reason
    let rect = FONT
        .render_aligned(
            format_args!(
                "Reset Reason: {:?}\nWake reason: {:?}\nCycle: {}\nRect: ({}, {}, {}, {})",
                reason,
                wake_reason,
                cycle,
                last_rect.top_left.x,
                last_rect.top_left.y,
                last_rect.size.width,
                last_rect.size.height,
            ),
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
    // turn screen off
    display.power_off();
    unsafe {
        if let Some(rect) = rect {
            LAST_RECT = rect;
        }
        CYCLE += 1;
    }

    delay.delay_millis(100);

    let mut rtc_cfg = RtcSleepConfig::deep();
    rtc_cfg.set_rtc_fastmem_pd_en(false);
    rtc_cfg.set_rtc_slowmem_pd_en(false);

    let timer = TimerWakeupSource::new(Duration::from_secs(30));
    rtc.sleep(&rtc_cfg, &[&timer]);

    loop {}
}
