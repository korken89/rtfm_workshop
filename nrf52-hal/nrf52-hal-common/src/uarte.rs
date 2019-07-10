//! HAL interface to the UARTE peripheral
//!
//! See product specification:
//!
//! - nrf52832: Section 35
//! - nrf52840: Section 6.34
use core::fmt;
use core::ops::Deref;
use core::sync::atomic::{compiler_fence, Ordering::SeqCst};

use crate::target::{uarte0, UARTE0};

use crate::gpio::{Floating, Input, Output, Pin, PushPull};
use crate::prelude::*;
use crate::target_constants::EASY_DMA_SIZE;
use crate::timer::Timer;

use heapless::{
    consts::*,
    pool,
    pool::singleton::{Box, Pool},
    spsc::{Consumer, Queue, Producer},
};

// Re-export SVD variants to allow user to directly set values
pub use crate::target::uarte0::{baudrate::BAUDRATEW as Baudrate, config::PARITYW as Parity};

pub trait UarteExt: Deref<Target = uarte0::RegisterBlock> + Sized {
    fn constrain(self, pins: Pins, parity: Parity, baudrate: Baudrate) -> Uarte<Self>;
    fn ptr() -> *const uarte0::RegisterBlock;
}

impl UarteExt for UARTE0 {
    fn constrain(self, pins: Pins, parity: Parity, baudrate: Baudrate) -> Uarte<Self> {
        Uarte::new(self, pins, parity, baudrate)
    }

    fn ptr() -> *const uarte0::RegisterBlock {
        UARTE0::ptr()
    }
}

/// Interface to a UARTE instance
///
/// This is a very basic interface that comes with the following limitations:
/// - The UARTE instances share the same address space with instances of UART.
///   You need to make sure that conflicting instances
///   are disabled before using `Uarte`. See product specification:
///     - nrf52832: Section 15.2
///     - nrf52840: Section 6.1.2
pub struct Uarte<T>(T);

impl<T> Uarte<T>
where
    T: UarteExt,
{
    pub fn new(uarte: T, mut pins: Pins, parity: Parity, baudrate: Baudrate) -> Self {
        // Select pins
        uarte.psel.rxd.write(|w| {
            let w = unsafe { w.pin().bits(pins.rxd.pin) };
            #[cfg(feature = "52840")]
            let w = w.port().bit(pins.rxd.port);
            w.connect().connected()
        });
        pins.txd.set_high();
        uarte.psel.txd.write(|w| {
            let w = unsafe { w.pin().bits(pins.txd.pin) };
            #[cfg(feature = "52840")]
            let w = w.port().bit(pins.txd.port);
            w.connect().connected()
        });

        // Optional pins
        uarte.psel.cts.write(|w| {
            if let Some(ref pin) = pins.cts {
                let w = unsafe { w.pin().bits(pin.pin) };
                #[cfg(feature = "52840")]
                let w = w.port().bit(pin.port);
                w.connect().connected()
            } else {
                w.connect().disconnected()
            }
        });

        uarte.psel.rts.write(|w| {
            if let Some(ref pin) = pins.rts {
                let w = unsafe { w.pin().bits(pin.pin) };
                #[cfg(feature = "52840")]
                let w = w.port().bit(pin.port);
                w.connect().connected()
            } else {
                w.connect().disconnected()
            }
        });

        // Enable UARTE instance
        uarte.enable.write(|w| w.enable().enabled());

        // Configure
        let hardware_flow_control = pins.rts.is_some() && pins.cts.is_some();
        uarte
            .config
            .write(|w| w.hwfc().bit(hardware_flow_control).parity().variant(parity));

        // Configure frequency
        uarte.baudrate.write(|w| w.baudrate().variant(baudrate));

        Uarte(uarte)
    }

    /// Write via UARTE
    ///
    /// This method uses transmits all bytes in `tx_buffer`
    ///
    /// The buffer must have a length of at most 255 bytes on the nRF52832
    /// and at most 65535 bytes on the nRF52840.
    pub fn write(&mut self, tx_buffer: &[u8]) -> Result<(), Error> {
        if tx_buffer.len() > EASY_DMA_SIZE {
            return Err(Error::TxBufferTooLong);
        }

        // Conservative compiler fence to prevent optimizations that do not
        // take in to account actions by DMA. The fence has been placed here,
        // before any DMA action has started
        compiler_fence(SeqCst);

        // Set up the DMA write
        self.0.txd.ptr.write(|w|
            // We're giving the register a pointer to the stack. Since we're
            // waiting for the UARTE transaction to end before this stack pointer
            // becomes invalid, there's nothing wrong here.
            //
            // The PTR field is a full 32 bits wide and accepts the full range
            // of values.
            unsafe { w.ptr().bits(tx_buffer.as_ptr() as u32) });
        self.0.txd.maxcnt.write(|w|
            // We're giving it the length of the buffer, so no danger of
            // accessing invalid memory. We have verified that the length of the
            // buffer fits in an `u8`, so the cast to `u8` is also fine.
            //
            // The MAXCNT field is 8 bits wide and accepts the full range of
            // values.
            unsafe { w.maxcnt().bits(tx_buffer.len() as _) });

        // Start UARTE Transmit transaction
        self.0.tasks_starttx.write(|w|
            // `1` is a valid value to write to task registers.
            unsafe { w.bits(1) });

        // Wait for transmission to end
        while self.0.events_endtx.read().bits() == 0 {}

        // Reset the event, otherwise it will always read `1` from now on.
        self.0.events_endtx.write(|w| w);

        // Conservative compiler fence to prevent optimizations that do not
        // take in to account actions by DMA. The fence has been placed here,
        // after all possible DMA actions have completed
        compiler_fence(SeqCst);

        if self.0.txd.amount.read().bits() != tx_buffer.len() as u32 {
            return Err(Error::Transmit);
        }

        Ok(())
    }

    /// Read via UARTE
    ///
    /// This method fills all bytes in `rx_buffer`, and blocks
    /// until the buffer is full.
    ///
    /// The buffer must have a length of at most 255 bytes
    pub fn read(&mut self, rx_buffer: &mut [u8]) -> Result<(), Error> {
        self.start_read(rx_buffer)?;

        // Wait for transmission to end
        while self.0.events_endrx.read().bits() == 0 {}

        self.finalize_read();

        if self.0.rxd.amount.read().bits() != rx_buffer.len() as u32 {
            return Err(Error::Receive);
        }

        Ok(())
    }

    /// Read via UARTE
    ///
    /// This method fills all bytes in `rx_buffer`, and blocks
    /// until the buffer is full or the timeout expires, whichever
    /// comes first.
    ///
    /// If the timeout occurs, an `Error::Timeout(n)` will be returned,
    /// where `n` is the number of bytes read successfully.
    ///
    /// This method assumes the interrupt for the given timer is NOT enabled,
    /// and in cases where a timeout does NOT occur, the timer will be left running
    /// until completion.
    ///
    /// The buffer must have a length of at most 255 bytes
    pub fn read_timeout<I>(
        &mut self,
        rx_buffer: &mut [u8],
        timer: &mut Timer<I>,
        cycles: u32,
    ) -> Result<(), Error>
    where
        I: TimerExt,
    {
        // Start the read
        self.start_read(rx_buffer)?;

        // Start the timeout timer
        timer.start(cycles);

        // Wait for transmission to end
        let mut event_complete = false;
        let mut timeout_occured = false;

        loop {
            event_complete |= self.0.events_endrx.read().bits() != 0;
            timeout_occured |= timer.wait().is_ok();
            if event_complete || timeout_occured {
                break;
            }
        }

        // Cleanup, even in the error case
        self.finalize_read();

        let bytes_read = self.0.rxd.amount.read().bits() as usize;

        if timeout_occured {
            return Err(Error::Timeout(bytes_read));
        }

        if bytes_read != rx_buffer.len() as usize {
            return Err(Error::Receive);
        }

        Ok(())
    }

    /// Start a UARTE read transaction by setting the control
    /// values and triggering a read task
    fn start_read(&mut self, rx_buffer: &mut [u8]) -> Result<(), Error> {
        // This is overly restrictive. See (similar SPIM issue):
        // https://github.com/nrf-rs/nrf52/issues/17
        if rx_buffer.len() > u8::max_value() as usize {
            return Err(Error::TxBufferTooLong);
        }

        // Conservative compiler fence to prevent optimizations that do not
        // take in to account actions by DMA. The fence has been placed here,
        // before any DMA action has started
        compiler_fence(SeqCst);

        // Set up the DMA read
        self.0.rxd.ptr.write(|w|
            // We're giving the register a pointer to the stack. Since we're
            // waiting for the UARTE transaction to end before this stack pointer
            // becomes invalid, there's nothing wrong here.
            //
            // The PTR field is a full 32 bits wide and accepts the full range
            // of values.
            unsafe { w.ptr().bits(rx_buffer.as_ptr() as u32) });
        self.0.rxd.maxcnt.write(|w|
            // We're giving it the length of the buffer, so no danger of
            // accessing invalid memory. We have verified that the length of the
            // buffer fits in an `u8`, so the cast to `u8` is also fine.
            //
            // The MAXCNT field is at least 8 bits wide and accepts the full
            // range of values.
            unsafe { w.maxcnt().bits(rx_buffer.len() as _) });

        // Start UARTE Receive transaction
        self.0.tasks_startrx.write(|w|
            // `1` is a valid value to write to task registers.
            unsafe { w.bits(1) });

        Ok(())
    }

    /// Finalize a UARTE read transaction by clearing the event
    fn finalize_read(&mut self) {
        // Reset the event, otherwise it will always read `1` from now on.
        self.0.events_endrx.write(|w| w);

        // Conservative compiler fence to prevent optimizations that do not
        // take in to account actions by DMA. The fence has been placed here,
        // after all possible DMA actions have completed
        compiler_fence(SeqCst);
    }

    /// Return the raw interface to the underlying UARTE peripheral
    pub fn free(self) -> T {
        self.0
    }

    pub fn split(
        self,
        rxq: Queue<(Box<DMAPool>,usize), U2>,
        txc: Consumer<'static, (Box<DMAPool>,usize), TXQSize>,
        txp: Producer<'static, (Box<DMAPool>,usize), TXQSize>,
    ) -> (UarteRX<T>, UarteTX<T>) {
        let mut rx = UarteRX::<T>::new(rxq);
        rx.enable_interrupts();
        rx.prepare_read().unwrap();
        rx.start_read();

        let tx = UarteTX::<T>::new(txc, txp);
        tx.enable_interrupts();
        (rx, tx)
    }
}

// DMA block is 4 bytes
pub const DMA_SIZE: usize = 4;
pool!(DMAPool: [u8; DMA_SIZE]);

pub struct UarteRX<T> {
    rxq: Queue<(Box<DMAPool>,usize), U2>, // double buffering of DMA chunkns
    _marker: core::marker::PhantomData<T>,
}

#[derive(Debug)]
pub enum RXError {
    RxqOverflow,
    RxqUnderflow,
    OOM,
}

impl<T> UarteRX<T>
where
    T: UarteExt,
{
    fn new(rxq: Queue<(Box<DMAPool>,usize), U2>) -> Self {
        Self {
            rxq,
            _marker: core::marker::PhantomData,
        }
    }

    // we listen to RXSTARTED and ENDRX
    pub fn enable_interrupts(&self) {
        let uarte = unsafe { &*T::ptr() };
        uarte
            .inten
            .modify(|_, w| w.endrx().set_bit().rxstarted().set_bit());
    }

    /// Start a UARTE read transaction
    pub fn start_read(&mut self) {
        let uarte = unsafe { &*T::ptr() };

        // Start UARTE Receive transaction
        uarte.tasks_startrx.write(|w|
            // `1` is a valid value to write to task registers.
            unsafe { w.bits(1) });
    }

    /// Prepare UARTE read transaciton
    pub fn prepare_read(&mut self) -> Result<(), RXError> {
        let uarte = unsafe { &*T::ptr() };

        let b = DMAPool::alloc().ok_or(RXError::OOM)?.freeze();
        compiler_fence(SeqCst);
        // setup start address
        uarte
            .rxd
            .ptr
            .write(|w| unsafe { w.ptr().bits(b.as_ptr() as u32) });
        // setup length
        uarte
            .rxd
            .maxcnt
            .write(|w| unsafe { w.maxcnt().bits(b.len() as _) });

        let len = b.len();
        if self.rxq.enqueue((b,len)).is_err() {
            Err(RXError::RxqOverflow)
        } else {
            Ok(())
        }
    }

    pub fn process_interrupt(&mut self) -> Result<Option<Box<DMAPool>>, RXError> {
        let uarte = unsafe { &*T::ptr() };

        // check if dma rx transaction has started
        if uarte.events_rxstarted.read().bits() == 1 {
            // dma transacton has started
            self.prepare_read()?;

            // Reset the event, otherwise it will always read `1` from now on.
            uarte.events_rxstarted.write(|w| w);
        }

        // check id dma transaction finished
        if uarte.events_endrx.read().bits() == 1 {
            // our transaction has finished
            let ret_b = self.rxq.dequeue().ok_or(RXError::RxqUnderflow)?;

            // Reset the event, otherwise it will always read `1` from now on.
            uarte.events_endrx.write(|w| w);

            self.start_read();

            return Ok(Some(ret_b.0)); // ok to return, rx started will be caught later
        }

        // the interrupt was not RXSTARTED or ENDRX, so no action
        Ok(None)
    }
}

pub type TXQSize = U4;

pub struct UarteTX<T> {
    txc: Consumer<'static, (Box<DMAPool>,usize), TXQSize>, // chunks to transmit
    txp: Producer<'static, (Box<DMAPool>,usize), TXQSize>, // so we can send ourself
    current: Option<Box<DMAPool>>,
    _marker: core::marker::PhantomData<T>,
}

impl<T> UarteTX<T>
where
    T: UarteExt,
{
    fn new(txc: Consumer<'static, (Box<DMAPool>,usize), TXQSize>, txp: Producer<'static, (Box<DMAPool>,usize), TXQSize>) -> Self {
        Self {
            txc,
            txp,
            current: None,
            _marker: core::marker::PhantomData,
        }
    }

    // we listen to ENDTX
    pub fn enable_interrupts(&self) {
        let uarte = unsafe { &*T::ptr() };
        uarte.inten.modify(|_, w| w.endtx().set_bit());
    }

    pub fn enqueue(&mut self, b: (Box<DMAPool>,usize)) {
        self.txp.enqueue(b).unwrap();
    }

    pub fn start_write(&mut self, b: (Box<DMAPool>,usize)) {
        let uarte = unsafe { &*T::ptr() };
        compiler_fence(SeqCst);
        // setup start address
        uarte
            .txd
            .ptr
            .write(|w| unsafe { w.ptr().bits(b.0.as_ptr() as u32) });
        // setup length
        uarte
            .txd
            .maxcnt
            .write(|w| unsafe { w.maxcnt().bits(b.1 as _) });
        // Start UARTE transmit transaction
        uarte.tasks_starttx.write(|w| unsafe { w.bits(1) });
        self.current = Some(b.0); // drops the previous current package
    }

    pub fn process_interrupt(&mut self) {
        let uarte = unsafe { &*T::ptr() };

        // ENDTX event? (dma transaction finished)
        if uarte.events_endtx.read().bits() == 1 {
            // our transaction has finished
            match self.txc.dequeue() {
                None => {
                    // a ENDTX without an started transaction is an error
                    if self.current.is_none() {
                        panic!("Internal error, ENDTX without current transaction.")
                    }
                    // we don't have any more to send, so drop the current buffer
                    self.current = None;
                }
                Some(b) => {
                    
                    self.start_write(b);
                }
            }

            // Reset the event, otherwise it will always read `1` from now on.
            uarte.events_endtx.write(|w| w);
        } else {
            if self.current.is_none() {
                match self.txc.dequeue() {
                    Some(b) =>
                    // we were idle, so start a new transaction
                    {
                        
                        self.start_write(b)
                    }
                    None => (),
                }
            }
        }
    }
}

impl<T> fmt::Write for UarteTX<T>
where
    T: UarteExt,
{
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for block in s.as_bytes().chunks(DMA_SIZE) {
            let mut buf = DMAPool::alloc().ok_or(fmt::Error)?.freeze();
            buf[..block.len()].copy_from_slice(block);
            self.txp.enqueue((buf, block.len())).unwrap();
            //rtfm::pend(interrupt::UARTE0_UART0);
            self.process_interrupt();
        }
        Ok(())
    }
}

impl<T> fmt::Write for Uarte<T>
where
    T: UarteExt,
{
    fn write_str(&mut self, s: &str) -> fmt::Result {
        // Copy all data into an on-stack buffer so we never try to EasyDMA from
        // flash
        let buf = &mut [0; 16][..];
        for block in s.as_bytes().chunks(16) {
            buf[..block.len()].copy_from_slice(block);
            self.write(&buf[..block.len()]).map_err(|_| fmt::Error)?;
        }

        Ok(())
    }
}

pub struct Pins {
    pub rxd: Pin<Input<Floating>>,
    pub txd: Pin<Output<PushPull>>,
    pub cts: Option<Pin<Input<Floating>>>,
    pub rts: Option<Pin<Output<PushPull>>>,
}

#[derive(Debug)]
pub enum Error {
    TxBufferTooLong,
    RxBufferTooLong,
    Transmit,
    Receive,
    Timeout(usize),
}
