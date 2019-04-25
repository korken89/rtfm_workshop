//! app9.rs
//!
//! Lets blink a LED using the built in scheduling capabilities,
//! together with having a function get a resource through a trait
//! object.
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
use rtfm::app;

const PERIOD: u32 = 64_000_000;

#[app(device = crate::hal::target)]
const APP: () = {
    // Late resources
    // static mut LED: OutputPin = (); <= size not known
    // So unfortunately we cannot use that
    //
    // static mut LED: impl OutputPin = (); <= impl not allowed here
    // That may be possible in the future, hope so...
    //
    // In the meantime:
    // We can use the type alias (defined in a library)
    // to make it easier on the eye.
    static mut LED: LED1 = ();

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
        toggler(&mut *resources.LED, false);

        schedule.high(scheduled + PERIOD.cycles()).unwrap();
    }

    #[task(schedule = [low], resources = [LED])]
    fn high() {
        toggler(&mut *resources.LED, true);
        schedule.low(scheduled + PERIOD.cycles()).unwrap();
    }

    extern "C" {
        fn SWI1_EGU1();
    }
};

// toggler is generic over OutputPin implementers
fn toggler(led: &mut impl OutputPin, state: bool) {
    if state {
        led.set_high();
    } else {
        led.set_low();
    }
}

// the full type is...
// fn toggler<P>(led: &mut P, state: bool) where P: OutputPin
//
// Why is this important you may ask?
// Well, the Rust compiler/LLVM is very good at momonorphing generics
// while trait objects may be harder to optimize.
// In certain cases, we need dynamic dispatch but in this case not.
// So here (and similar cases) generics are preferable.
//
// Thanks to Jorge Aparicio for pointing me in the right direction :)
//
// -----------------------------------------------------------------
//
// Define the type alias in the library
type LED1 = P0_14<gpio::Output<PushPull>>;
// Notice,
// This is a poor mans solution, we cannot make resource
// definitions type generic with concrete type inferred.
//
// This is not specific to RTFM but a rather a
// shortcoming of the Rust type system, that requires
// explicit types for statics. A possible workaround would
// be to have a run once (at Init time) allocator.
// Drawback is that LLVM would not be able to optimize out
// the indirections, thus we stick with static allocation.
//
// Side note: The Timber language (a fore-runner to RTFM)
// has a static heap collected after init (as above),
// but Timber cannot compete with Rust-RTFM from a performance
// perspective.
