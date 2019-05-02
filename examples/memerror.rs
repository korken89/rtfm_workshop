#![no_main]
#![no_std]

// panic handler
extern crate dwm1001;
extern crate panic_semihosting;

use cortex_m_semihosting::hprintln;
use dwm1001::nrf52832_hal as hal;
use hal::{DMAPool, TXQSize};
use heapless::{
    pool::singleton::Box,
    spsc::{Consumer, Producer, Queue},
};

use rtfm::app;

#[app(device = crate::hal::target)]
const APP: () = {
    static mut PRODUCER: Producer<'static, Box<DMAPool>, TXQSize> = ();
    static mut CONSUMER: Consumer<'static, Box<DMAPool>, TXQSize> = ();

    #[init(spawn = [])]
    fn init() -> init::LateResources {
        // for the producer/consumer of TX
        static mut TX_RB: Option<Queue<Box<DMAPool>, TXQSize>> = None;

        hprintln!("init").unwrap();

        *TX_RB = Some(Queue::new());
        let m = TX_RB.as_mut().take();

        // Should always be some
        match &m {
            Some(_) => hprintln!("Some ok").unwrap(),
            None => hprintln!("Danger danger").unwrap(),
        }

        let (txp, txc) = m.unwrap().split();

        init::LateResources {
            PRODUCER: txp,
            CONSUMER: txc,
        }
    }
};
