//! minimal.rs
//!
//! A minial example to run, prints hello over semihosting

#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate panic_semihosting;

use cortex_m_rt::entry;
use cortex_m_semihosting::hprintln;

extern crate dwm1001;

#[entry]
fn main() -> ! {
    hprintln!("hello").unwrap();
    loop {}
}
