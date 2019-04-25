//! app4.rs
//!
//! Example of sharing data via lock-free data structures (Single
//! Producer, Single Consumer Queue), together with resources that
//! need to be initialized during `init`.

#![no_main]
#![no_std]

// panic handler
extern crate panic_semihosting;

use cortex_m_semihosting::hprintln;
use dwm1001::nrf52832_hal as hal;
use hal::nrf52832_pac as pac;
use heapless::{
    consts::*,
    spsc::{Consumer, Producer, Queue},
};
use pac::interrupt;
use rtfm::app;

#[app(device = crate::hal::target)]
const APP: () = {
    // The Producer and Consumer of the data (generated from Queue)
    static mut P: Producer<'static, u32, U4> = ();
    static mut C: Consumer<'static, u32, U4> = ();

    #[init]
    fn init() -> init::LateResources {
        // NOTE: we use `Option` here to work around the lack of
        // a stable `const` constructor
        static mut Q: Option<Queue<u32, U4>> = None;

        *Q = Some(Queue::new());
        let (p, c) = Q.as_mut().unwrap().split();

        // Initialization of late resources
        init::LateResources { P: p, C: c }
    }

    #[idle(resources = [C])]
    fn idle() -> ! {
        loop {
            if let Some(data) = resources.C.dequeue() {
                hprintln!("received message: {}", data).unwrap();
            } else {
                rtfm::pend(interrupt::SWI0_EGU0);
            }
        }
    }

    #[interrupt(resources = [P])]
    fn SWI0_EGU0() {
        static mut NUMBER: u32 = 0;
        hprintln!("SWI0_EGU0: {}", NUMBER).unwrap();
        resources.P.enqueue(*NUMBER).unwrap();
        *NUMBER += 1;
    }
};
