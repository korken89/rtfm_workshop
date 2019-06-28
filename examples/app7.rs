//! app7.rs
//!
//! Lets blink a LED using the built in scheduling capabilities,
//! together with having a function get exclusive access.
//! The LED is considered a shared resource which need
//! initialization in this example,

#![no_main]
#![no_std]

// panic handler
extern crate panic_semihosting;

use cortex_m_semihosting::hprintln;
use dwm1001::nrf52832_hal as hal;
use embedded_hal::digital::OutputPin;
use hal::gpio;
use hal::gpio::p0::*;
use hal::gpio::*;
use hal::prelude::GpioExt;
use rtfm::{app, Exclusive};

const PERIOD: u32 = 64_000_000;

#[app(device = crate::hal::target)]
const APP: () = {
    // Late resources are resources that are not initialized at compile-time
    static mut LED: P0_14<gpio::Output<PushPull>> = ();

    // spawn attribute defines which tasks we are allowed to spawn from this function
    // spawn = run now
    // schedule = run at specific time
    #[init(spawn = [led_off_task])]
    fn init() -> init::LateResources {
        hprintln!("init").unwrap();
        // device is owned by RTFM, and represents the mcu peripherals, eg
        // let device = mcu::Peripherals::take().unwrap();

        let port0 = device.P0.split();
        let led = port0.p0_14.into_push_pull_output(Level::High);

        spawn.led_off_task().unwrap();

        // the late resources must be returned if there are any late resources in the system
        init::LateResources { LED: led }
    }

    #[task(schedule = [led_on_task], resources = [LED])]
    fn led_off_task() {
        toggler(resources.LED, false);
        // the schedule is unwrapped, since the system is not allowed to schedule a task twice
        schedule.led_on_task(scheduled + PERIOD.cycles()).unwrap();
    }

    // The resources variable is uniquely generated for each task by RTFM, and keeps track of which
    // shared resources are available to this task
    #[task(schedule = [led_off_task], resources = [LED])]
    fn led_on_task() {
        toggler(resources.LED, true);

        // scheduled is the time when something was actually scheduled by for example another task
        // or something, and thus something can be scheduled relative to the last scheduled time
        schedule.led_off_task(scheduled + PERIOD.cycles()).unwrap();
    }

    // here we define which free interrupt RTFM can use to schedule and run tasks
    extern "C" {
        fn SWI1_EGU1();
    }
};

// Arument is Exclusive access to a specific pin.
// Exclusive is a an RTFM specific type that
// guarantees that nothing else is trying to access this resource at the same time.
fn toggler<T>(mut led: Exclusive<T>, state: bool) where T: OutputPin {
    if state {
        led.set_high();
    } else {
        led.set_low();
    }
}
