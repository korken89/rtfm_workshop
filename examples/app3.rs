//! app4.rs
//!
//! Example of defining and utilizing tasks

#![no_main]
#![no_std]

// panic handler
extern crate panic_semihosting;

use cortex_m_semihosting::hprintln;
use dwm1001::nrf52832_hal as hal;
use rtfm::app;

#[app(device = crate::hal::target)]
const APP: () = {
    // May spawn foo
    #[init(spawn = [foo])]
    fn init() {
        hprintln!("init").unwrap();
        spawn.foo().unwrap();
    }

    // May spawn bar and baz
    #[task(spawn = [bar, baz])]
    fn foo() {
        hprintln!("foo").unwrap();
        spawn.bar().unwrap();
        spawn.baz().unwrap();
    }

    #[task]
    fn bar() {
        hprintln!("bar").unwrap();
    }

    #[task(priority = 2)]
    fn baz() {
        hprintln!("baz").unwrap();
    }

    extern "C" {
        fn SWI0_EGU0();
        fn SWI1_EGU1();
    }
};
