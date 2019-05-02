//! app6.rs
//!
//! Lets blink a LED using the built in scheduling capabilities.
//! The LED is considered a shared resource which need
//! initialization in this example,

#![no_main]
#![no_std]

// panic handler
extern crate panic_semihosting;

use cortex_m_semihosting::hprintln;
use embedded_hal::digital::OutputPin;
use hal::gpio;
use hal::gpio::p0::*;
use hal::gpio::*;
use hal::prelude::GpioExt;
use nrf52832_hal as hal;
use rtfm::app;

const PERIOD: u32 = 64_000_000;

#[app(device = crate::hal::target)]
const APP: () = {
    // Late resources
    static mut LED: P0_14<gpio::Output<PushPull>> = ();

    #[init(spawn = [low])]
    fn init() -> init::LateResources {
        hprintln!("init").unwrap();

        let port0 = device.P0.split();
        let led = port0.p0_14.into_push_pull_output(Level::High);

        spawn.low().unwrap();

        init::LateResources { LED: led }
    }

    #[task(schedule = [high], resources = [LED])]
    fn low() {
        resources.LED.set_low();

        schedule.high(scheduled + PERIOD.cycles()).unwrap();
    }

    #[task(schedule = [low], resources = [LED])]
    fn high() {
        resources.LED.set_high();
        schedule.low(scheduled + PERIOD.cycles()).unwrap();
    }

    extern "C" {
        fn SWI1_EGU1();
    }
};
