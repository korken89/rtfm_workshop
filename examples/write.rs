#![no_main]
#![no_std]

// panic handler
extern crate panic_semihosting;

use cortex_m_semihosting::hprintln;
use dwm1001::{new_usb_uarte, nrf52832_hal as hal, UsbUarteConfig};

use hal::prelude::*;
use hal::target::{interrupt, UARTE0};
use hal::{DMAPool, RXError, TXQSize, UarteRX, UarteTX, DMA_SIZE};

use core::fmt::Write;

use heapless::{
    // consts::*,
    // pool,
    pool::singleton::{Box, Pool},
    spsc::{Queue},
};

use rtfm::app;

const NR_PACKAGES: usize = 10;
const DMA_MEM: usize = DMA_SIZE * NR_PACKAGES + 16;

const PERIOD: u32 = 64_000_000;

#[app(device = crate::hal::target)]
const APP: () = {
    static mut RX: UarteRX<UARTE0> = ();
    static mut TX: UarteTX<UARTE0> = ();
    //static mut PRODUCER: Producer<'static, Box<DMAPool>, TXQSize> = ();

    #[init(spawn = [write_test])]
    fn init() -> init::LateResources {
        // for the actual DMA buffers
        static mut MEMORY: [u8; DMA_MEM] = [0; DMA_MEM];
        // for the producer/consumer of TX
        static mut TX_RB: Option<Queue<(Box<DMAPool>,usize), TXQSize>> = None;

        hprintln!("init").unwrap();
        // move MEMORY to P (the DMA buffer allocator)
        DMAPool::grow(MEMORY);

        let port0 = device.P0.split();
        let uarte0 = new_usb_uarte(
            device.UARTE0,             // the actual UART
            port0.p0_05,               // txd
            port0.p0_11,               // rxd
            UsbUarteConfig::default(), // 115200
        );

        *TX_RB = Some(Queue::new());
        let (txp, txc) = TX_RB.as_mut().unwrap().split();
        let (rx, tx) = uarte0.split(Queue::new(), txc, txp);

        spawn.write_test().unwrap();

        init::LateResources {
            RX: rx,
            TX: tx,
            //PRODUCER: txp,
        }
    }

    #[task(schedule=[write_test], resources=[TX])]
    fn write_test() {
      resources.TX.lock(|tx|{
        writeln!(*tx, "Hello write trait\r").unwrap();
      });
      schedule.write_test(scheduled + PERIOD.cycles()).unwrap();
    }

    // // we can get Box<P> us being now the owner
    #[task(capacity = 2, resources = [TX])]
    fn printer(data: Box<DMAPool>) {
        // enqueue a test message
        // let mut b = DMAPool::alloc().unwrap().freeze();
        // b.copy_from_slice(&[0, 1, 2, 3]);

        // hprintln!("{:?}", &data).unwrap();
        // just do the buffer dance without copying
        resources.TX.lock(|tx|{
          let len = data.len();
          tx.enqueue((data, len));
        });
        rtfm::pend(interrupt::UARTE0_UART0);
    }

    #[task]
    fn rx_error(err: RXError) {
        hprintln!("rx_error {:?}", err).unwrap();
    }

    #[interrupt(priority = 2, resources = [RX, TX], spawn = [printer,rx_error])]
    fn UARTE0_UART0() {
        // probe RX
        match resources.RX.process_interrupt() {
            Ok(Some(b)) => {
              match spawn.printer(b) {
                  Err(_) => spawn.rx_error(RXError::OOM).unwrap(),
                  _ => (),
              };
            }
            Ok(None) => (), // no
            Err(err) => spawn.rx_error(err).unwrap(),
        }

        resources.TX.process_interrupt();
    }

    extern "C" {
        fn SWI1_EGU1();
        fn SWI2_EGU2();
    }
};
