//! app5.rs
//!
//! Lets blink a LED! The LED is considered a shared resource
//! which need initialization in this example.

#![no_main]
#![no_std]

// panic handler
extern crate panic_semihosting;

use cortex_m_semihosting::hprintln;

use embedded_hal::digital::OutputPin;
use hal::gpio;
use hal::gpio::p0::*;
use hal::gpio::*;
// use hal::prelude::GpioExt;
use nrf52832_hal as hal;
use rtfm::app;

#[app(device = crate::hal::target)]
const APP: () = {
    // Late resources
    static mut LED: P0_14<gpio::Output<PushPull>> = ();

    #[init]
    fn init() -> init::LateResources {
        hprintln!("init").unwrap();

        let port0 = device.P0.split();
        let led = port0.p0_14.into_push_pull_output(Level::High);

        init::LateResources { LED: led }
    }

    #[idle(resources = [LED])]
    fn idle() -> ! {
        let led = resources.LED;
        loop {
            hprintln!("low").unwrap();
            led.set_low();
            hprintln!("high").unwrap();
            led.set_high();
        }
    }
};
