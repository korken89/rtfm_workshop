//! app2.rs
//!
//! Example of utilizing pend and shared resources when the tasks have different priority

#![no_main]
#![no_std]

// panic handler
extern crate panic_semihosting;

use cortex_m_semihosting::hprintln;
use hal::nrf52832_pac as pac;
use nrf52832_hal as hal;
use pac::Interrupt;
use rtfm::app;

#[app(device = crate::hal::target)]
const APP: () = {
    // A resource named SHARED
    static mut SHARED: u64 = 0;

    #[init]
    fn init() {
        hprintln!("init").unwrap();
        rtfm::pend(Interrupt::SWI0_EGU0);
        rtfm::pend(Interrupt::SWI1_EGU1);
    }

    #[idle]
    fn idle() -> ! {
        hprintln!("idle").unwrap();

        loop {
            // hprintln!(".").unwrap();
        }
    }

    // defaults to priority = 1
    #[interrupt(resources = [SHARED])]
    fn SWI0_EGU0() {
        hprintln!("SWI0_EGU0 start").unwrap();
        resources.SHARED.lock(|shared| {
            *shared += 1;
        });
        hprintln!("SWI0_EGU0 end").unwrap();
    }

    #[interrupt(priority = 2, resources = [SHARED])]
    fn SWI1_EGU1() {
        hprintln!("SWI1_EGU1 {:?}", resources.SHARED).unwrap();
        *resources.SHARED += 1;
    }
};
