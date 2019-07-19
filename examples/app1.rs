//! app1.rs
//!

#![no_main]
#![no_std]
#![feature(asm, const_fn, core_intrinsics, naked_functions)]

use cortex_m_rt::ExceptionFrame;
extern crate panic_semihosting;

use cortex_m_semihosting::hprintln;
use dwm1001::nrf52832_hal as hal;

pub use cortex_m::register::psp;
use embedded_hal::digital::OutputPin;
use hal::gpio;
use hal::gpio::p0::*;
use hal::gpio::*;
use hal::prelude::GpioExt;
use rtfm::app;

// http://infocenter.arm.com/help/topic/com.arm.doc.dui0553b/DUI0553.pdf
// exception stack frame
#[rustfmt::skip] 
#[derive(Copy, Clone, Debug)]
#[allow(non_snake_case)]
#[repr(C)]
pub struct stack_frame {
    R0:   u32,    // lowest address, top of stack frame
    R1:   u32,    
    R2:   u32,    
    R3:   u32,    
    R12:  u32,   
    LR:   u32,    
    PC:   u32,    
    xPSR: u32,   
}

// here we allocate the user stack
// we could think of using fixed location
// outside of the ram region for the RTFM kernel
static mut STACK: [u32; 1024] = [0; 1024];

// SVC definitions, maybe enum is better
const SVC_COMMAND: u8 = 2; // user command

// User land LED driver
// major, driver number, maybe enum is better
const LED_IO: usize = 1;
// minor, driver command, maybe enum is better
const LED_OFF: usize = 0;
const LED_ON: usize = 1;
const LED_START: usize = 2;
const LED_STOP: usize = 3;
const LED_GET_PERIOD: usize = 4;
const LED_SET_PERIOD: usize = 5;

#[app(device = crate::hal::target)]
const APP: () = {
    static mut LED_RUN: bool = false;
    static mut LED_PERIOD: u32 = 64_000_000;
    static mut LED: P0_14<gpio::Output<PushPull>> = ();

    #[init]
    fn init() -> init::LateResources {
        hprintln!("init").unwrap();

        let port0 = device.P0.split();
        let led = port0.p0_14.into_push_pull_output(Level::High);

        init::LateResources { LED: led }
    }

    #[idle]
    fn idle() -> ! {
        hprintln!("idle").unwrap();

        // setup PSP for entering user land
        unsafe {
            // set the PSP in the middle of the user land ram
            psp::write(&STACK[512] as *const _ as _);
            // Set thread mode to unprivileged and use PSP
            asm!("
                mov r0, #3 
                msr CONTROL, r0"
             : : : : "volatile"
            );
        };

        user_init();

        // should never happen
        hprintln!("Internal error").unwrap();
        loop {}
    }

    #[task(schedule = [high], resources = [LED_PERIOD, LED_RUN, LED])]
    fn low() {
        resources.LED.set_low();

        if *resources.LED_RUN {
            schedule
                .high(scheduled + resources.LED_PERIOD.cycles())
                .unwrap();
        }
    }

    #[task(schedule = [low], resources = [LED_PERIOD, LED_RUN, LED])]
    fn high() {
        resources.LED.set_high();
        if *resources.LED_RUN {
            schedule
                .low(scheduled + resources.LED_PERIOD.cycles())
                .unwrap();
        }
    }

    #[exception(spawn = [low],resources = [LED_PERIOD, LED_RUN, LED])]
    fn SVCall() {
        let psp = psp::read();
        let psp_stack = unsafe { &mut *(psp as *mut stack_frame) };
        let pc = psp_stack.PC;
        // PC points to next thumb (16 bit) instruction
        // We read the previous instruction (SVC) from memory (first byte is immediate field)
        let syscall_nr = unsafe { core::ptr::read_volatile((pc - 2) as *const u8) };

        hprintln!("SVCALL {}", syscall_nr).unwrap();
        // hprintln!("Stack {:?}", psp_stack).unwrap();

        // this should be factored out to driver
        match syscall_nr {
            SVC_COMMAND => match psp_stack.R0 as usize {
                LED_IO => {
                    hprintln!("led driver").unwrap();
                    match psp_stack.R1 as usize {
                        LED_ON => {
                            hprintln!("led-on").unwrap();
                            resources.LED.set_low();
                        }
                        LED_OFF => {
                            hprintln!("led-off").unwrap();
                            resources.LED.set_high();
                        }
                        LED_START => {
                            hprintln!("led-start").unwrap();
                            *resources.LED_RUN = true;
                            spawn.low().unwrap();
                        }
                        LED_STOP => {
                            hprintln!("led-stop").unwrap();
                            *resources.LED_RUN = false
                        }
                        LED_GET_PERIOD => {
                            hprintln!("led-get-period").unwrap();
                            psp_stack.R0 = *resources.LED_PERIOD;
                        }
                        LED_SET_PERIOD => {
                            hprintln!("led-set-period").unwrap();
                            *resources.LED_PERIOD = psp_stack.R2;
                        }
                        _ => {
                            hprintln!("unkown command").unwrap();
                        }
                    }
                }
                _ => {
                    hprintln!("unknown driver").unwrap();
                }
            },
            _ => {
                hprintln!("unknown SVC command").unwrap();
            }
        }
    }

    // a non used interrupt handler for the tasks
    extern "C" {
        fn SWI1_EGU1();
    }
};

#[cortex_m_rt::exception]
unsafe fn HardFault(ef: &ExceptionFrame) -> ! {
    let _ = hprintln!("ef : {:?}", ef);

    loop {}
}

// user land API

// borrowed from Tock
unsafe fn command(major: usize, minor: usize, arg1: usize, arg2: usize) -> isize {
    let res;
    asm!("svc 2" : "={r0}"(res)
                 : "{r0}"(major) "{r1}"(minor) "{r2}"(arg1) "{r3}"(arg2)
                 : "memory"
                 : "volatile");

    res
}

fn led_on() {
    unsafe {
        command(LED_IO, LED_ON, 0, 0);
    }
}

fn led_off() {
    unsafe {
        command(LED_IO, LED_OFF, 0, 0);
    }
}

fn led_start() {
    unsafe {
        command(LED_IO, LED_START, 0, 0);
    }
}

fn led_stop() {
    unsafe {
        command(LED_IO, LED_STOP, 0, 0);
    }
}

fn led_get_period() -> u32 {
    unsafe { command(LED_IO, LED_GET_PERIOD, 0, 0) as u32 }
}

fn led_set_period(period: u32) {
    unsafe {
        command(LED_IO, LED_SET_PERIOD, period as usize, 0);
    }
}

// user land application
// notice, stepping the code wont progress systic, thus no blinking
fn user_main() -> ! {
    hprintln!("user_main").unwrap();

    led_on();
    led_off();
    let period = led_get_period();
    hprintln!("period {}", period).unwrap();
    led_start();
    cortex_m::asm::delay(period * 10);
    led_set_period(period / 10);
    cortex_m::asm::delay(period * 10);
    led_stop();
    loop {}
}

fn user_init() {
    hprintln!("user_init").unwrap();
    user_main();
}
