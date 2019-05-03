#![no_main]
#![no_std]

// panic handler
extern crate panic_semihosting;

use cortex_m_semihosting::hprintln;
use dwm1001::{new_usb_uarte, nrf52832_hal as hal, UsbUarteConfig};

use hal::gpio::*;
use hal::target::{interrupt, UARTE0};
use hal::{DMAPool, RXError, UarteRX, UarteTX, DMA_SIZE};

use heapless::{
    consts::*,
    pool::singleton::{Box, Pool},
    spsc::{Producer, Queue},
};

use rtfm::app;

const NR_PACKAGES: usize = 10;
const DMA_MEM: usize = DMA_SIZE * NR_PACKAGES + 16;

// number of output buffers that the driver should provide
type TXQSize = U3;

#[app(device = crate::hal::target)]
const APP: () = {
    static mut RX: UarteRX<UARTE0> = ();
    static mut TX: UarteTX<UARTE0, TXQSize> = ();
    static mut PRODUCER: Producer<'static, Box<DMAPool>, TXQSize> = ();

    #[init(spawn = [])]
    fn init() -> init::LateResources {
        // for the actual DMA buffers
        static mut MEMORY: [u8; DMA_MEM] = [0; DMA_MEM];
        // for the producer/consumer of TX
        static mut TX_RB: Option<Queue<Box<DMAPool>, TXQSize>> = None;

        hprintln!("init").unwrap();
        // move MEMORY to P (the DMA buffer allocator)
        DMAPool::grow(MEMORY);

        let port0 = p0::Parts::new(device.P0);
        let uarte0 = new_usb_uarte(
            device.UARTE0,             // the actual UART
            port0.p0_05,               // txd
            port0.p0_11,               // rxd
            UsbUarteConfig::default(), // 115200
        );

        *TX_RB = Some(Queue::new());
        let (txp, txc) = TX_RB.as_mut().unwrap().split();
        let (rx, tx) = uarte0.split(Queue::new(), txc);

        init::LateResources {
            RX: rx,
            TX: tx,
            PRODUCER: txp,
        }
    }

    // // we can get Box<P> us being now the owner
    #[task(capacity = 2, resources = [PRODUCER])]
    fn printer(data: Box<DMAPool>) {
        // enqueue a test message
        // let mut b = DMAPool::alloc().unwrap().freeze();
        // b.copy_from_slice(&[0, 1, 2, 3]);

        // hprintln!("{:?}", &data).unwrap();
        // just do the buffer dance without copying
        if resources.PRODUCER.enqueue(data).is_err() {
            hprintln!("outgoing queue is overfull").unwrap();
        } else {
            // we need to run the interrupt handler to tell
            // the driver that there is data to send
            // TODO, have that built in, for the PRODUCER
            rtfm::pend(interrupt::UARTE0_UART0);
        }
    }

    #[task]
    fn rx_error(err: RXError) {
        hprintln!("rx_error {:?}", err).unwrap();
    }

    #[interrupt(priority = 2, resources = [RX, TX], spawn = [printer, rx_error])]
    fn UARTE0_UART0() {
        // probe RX
        match resources.RX.process_interrupt() {
            Ok(Some(b)) => {
                // delegate data to printer
                match spawn.printer(b) {
                    Err(_) => spawn.rx_error(RXError::OOM).unwrap(),
                    _ => (),
                };
            }
            Ok(None) => (), // no
            Err(err) => spawn.rx_error(err).unwrap(),
        }

        // probe TX
        resources.TX.process_interrupt();
    }

    extern "C" {
        fn SWI1_EGU1();
        fn SWI2_EGU2();
    }
};
