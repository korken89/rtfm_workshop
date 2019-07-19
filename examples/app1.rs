//! app1.rs
//!
//! Example of utilizing pend, the minimal RTFM example!

#![no_main]
#![no_std]
#![feature(asm, const_fn, core_intrinsics, naked_functions)]

use core::sync::atomic::{self, Ordering};
// use cortex_m::asm;
use cortex_m_rt::ExceptionFrame;

// pub use cortexm::nvic;
pub use cortex_m::register::psp;
// pub use cortexm::syscall;
// pub use cortexm::systick;

// panic handler
extern crate panic_semihosting;

use cortex_m_semihosting::hprintln;
use dwm1001::nrf52832_hal as hal;
// use hal::nrf52832_pac as pac;
// use pac::interrupt;
use rtfm::app;

// Return to Handler mode, exception return uses non-floating-point state
// from the MSP and execution uses MSP after return.
// const RETURN_TO_HANDER_MODE_NO_FP_MSP: u32 = 0xFFFFFFF1;

// Return to Thread mode, exception return uses non-floating-point state from
// MSP and execution uses MSP after return.
// const RETURN_TO_HANDER_MODE_FP_MSP: u32 = 0xFFFFFFF9;

// Return to Thread mode, exception return uses non-floating-point state from
// the PSP and execution uses PSP after return.
// const RETURN_TO_THREAD_MODE_NO_FP_PSP: u32 = 0xFFFFFFFD;

// Return to Handler mode, exception return uses floating-point-state from
// MSP and execution uses MSP after return.
// const RETURN_TO_THREAD_MODE_FP_MSP: u32 = 0xFFFFFFE1;

// Return to Thread mode, exception return uses floating-point state from
// MSP and execution uses MSP after return.
// const RETURN_TO_THREAD_MODE_NO_FP_MSP: u32 = 0xFFFFFFE9;

// Return to Thread mode, exception return uses floating-point state from PSP
// and execution uses PSP after return.
// const RETURN_TO_THREAD_MODE_FP_PSP: u32 = 0xFFFFFFED;

// http://infocenter.arm.com/help/topic/com.arm.doc.dui0553b/DUI0553.pdf
// exception stack frame
#[rustfmt::skip] 
#[derive(Copy, Clone, Debug)]
#[allow(non_snake_case)]
#[repr(C, align(16))]
pub struct stack_frame {
    // align: u32, // padding 00, aligned to 16 byte
    R0: u32,    // padding 04, lowest address
    R1: u32,    // padding 08
    R2: u32,    // padding 0c
    R3: u32,    // padding 10
    R12: u32,   // padding 14
    LR: u32,    // padding 18
    PC: u32,    // padding 1C
    xPSR: u32,  // padding 20 
}

impl stack_frame {
    const fn new() -> stack_frame {
        stack_frame {
            // align: 0,
            R0: 0,
            R1: 0,
            R2: 0,
            R3: 0,
            R12: 0,
            LR: 0,
            PC: 0,
            xPSR: 0,
        }
    }
}

// fn switch_to_user(
//     mut user_stack: *const usize,
//     process_regs: &mut [usize; 8],
// ) -> *const usize {
//     asm!("
//     /* Load bottom of stack into Process Stack Pointer */
//     msr psp, $0

//     /* Load non-hardware-stacked registers from Process stack */
//     /* Ensure that $2 is stored in a callee saved register */
//     ldmia $2, {r4-r11}

//     /* SWITCH */
//     svc 0xff /* It doesn't matter which SVC number we use here */
//     /* Push non-hardware-stacked registers into Process struct's */
//     /* regs field */
//     stmia $2, {r4-r11}

//     mrs $0, PSP /* PSP into r0 */"
//     : "={r0}"(user_stack)
//     : "{r0}"(user_stack), "{r1}"(process_regs)
//     : "r4","r5","r6","r7","r8","r9","r10","r11" : "volatile" );
//     user_stack
// }

static mut STACK: [u32; 1024] = [0; 1024];
const PERIOD: u32 = 64_000_000;

#[app(device = crate::hal::target)]
const APP: () = {
    // static mut STACK: [stack_frame; 10] = [stack_frame::new(); 10];

    #[init]
    fn init() {
        hprintln!("init").unwrap();
        // rtfm::pend(interrupt::SWI0_EGU0);
    }
    // #[idle(resources = [STACK])]
    #[idle]
    fn idle() -> ! {
        hprintln!("idle").unwrap();

        // let user_stack = stack_frame {
        //     align: 0,
        //     R0: 0,
        //     R1: 1,
        //     R2: 2,
        //     R3: 3,
        //     R12: 12,
        //     LR: 0x0000_0001, // not relevant I believe
        //     PC: tock_fn as u32,
        //     //xPSR: 0x0100_0000, // thumb mode on return
        //     xPSR: 0x0100_0000, // thumb mode on return
        //                        //   28   24   20   16   12    8    4    0
        //                        //    |    |    |    |    |    |    |    |
        //                        // NZCV ---T ---- ---- ---- ---- ---E EEEE
        // };

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

    #[exception]
    fn SVCall() {
        let psp = psp::read();
        let psp_stack = unsafe { &*(psp as *const stack_frame) };
        let pc = psp_stack.PC;
        // PC points to next thumb (16 bit) instruction
        // We read the previous instruction (SVC) from memory (first byte is immediate field)
        let syscall_nr = unsafe { core::ptr::read_volatile((pc - 2) as *const u8) }; 

        // unsafe {
        //     asm!("
        //         // Set thread mode to unprivileged
        //         // mov r0, #1
        //         mrs CONTROL, r0
        //         // msr psp, r1 // r1 will point to the top of the user stack
        //         svc #124"
        //          : "{r1}" (syscall_nr) : : : "volatile");
        // }

        hprintln!("SVCALL {}", syscall_nr).unwrap();
        hprintln!("Stack {:?}", psp_stack).unwrap();

        // unsafe {
        //     asm!(
        //         "
        //         // Return to Thread mode, exception return uses non-floating-point state from
        //         // the PSP and execution uses PSP after return.
        //         movw lr, #0xfffd
        //         movt lr, #0xffff // RETURN_TO_THREAD_MODE_NO_FP_PSP
        //         bx lr"
        //             : : : : "volatile"
        //     );
        // }
        // hprintln!("SVCALL ERROR").unwrap();
        // loop {}
    }

    #[interrupt]
    fn SWI0_EGU0() {
        static mut TIMES: u32 = 0;
        *TIMES += 1;
        hprintln!("SWIO_EGU0 {}", TIMES).unwrap();
    }
};

#[cortex_m_rt::exception]
unsafe fn HardFault(ef: &ExceptionFrame) -> ! {
    let _ = hprintln!("ef : {:?}", ef);

    loop {}
}

//#[naked]

// user land API

// borrowed from Tock
fn command(major: usize, minor: usize, arg1: usize, arg2: usize) -> isize {
    let res;
    unsafe {
    asm!("svc 2" : "={r0}"(res)
                 : "{r0}"(major) "{r1}"(minor) "{r2}"(arg1) "{r3}"(arg2)
                 : "memory"
                 : "volatile");
    }
    res
}

// user land application
#[inline(never)]
fn user_function(a: u32) {
    hprintln!("a = {}", a).unwrap();
    if a > 0 {
        user_function(a - 1);
    }
}

fn user_main() -> ! {
    hprintln!("user_main").unwrap();
    user_function(2);
    
    let ret = command(1,2,3,4);
    hprintln!("ret = {}", ret).unwrap();

    let ret = command(2,3,4,5);
    hprintln!("ret = {}", ret).unwrap();

    loop {}
}

fn user_init() {
    hprintln!("user_init").unwrap();
    user_main();
}
