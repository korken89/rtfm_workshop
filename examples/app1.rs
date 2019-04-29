//! app1.rs
//!
//! Example of utilizing pend, the minimal RTFM example!

#![no_main]
#![no_std]
#![feature(asm, const_fn, core_intrinsics, naked_functions)]

use core::sync::atomic::{self, Ordering};
use cortex_m::asm;
use cortex_m_rt::ExceptionFrame;


// pub use cortexm::nvic;
// pub use cortexm::scb;
// pub use cortexm::syscall;
// pub use cortexm::systick;

// panic handler
extern crate panic_semihosting;

use cortex_m_semihosting::hprintln;
use dwm1001::nrf52832_hal as hal;
use hal::nrf52832_pac as pac;
use pac::interrupt;
use rtfm::app;

// Return to Handler mode, exception return uses non-floating-point state
// from the MSP and execution uses MSP after return.
const RETURN_TO_HANDER_MODE_NO_FP_MSP: u32 = 0xFFFFFFF1;

// Return to Thread mode, exception return uses non-floating-point state from
// MSP and execution uses MSP after return.
const RETURN_TO_HANDER_MODE_FP_MSP: u32 = 0xFFFFFFF9;

// Return to Thread mode, exception return uses non-floating-point state from
// the PSP and execution uses PSP after return.
const RETURN_TO_THREAD_MODE_NO_FP_PSP: u32 = 0xFFFFFFFD;

// Return to Handler mode, exception return uses floating-point-state from
// MSP and execution uses MSP after return.
const RETURN_TO_THREAD_MODE_FP_MSP: u32 = 0xFFFFFFE1;

// Return to Thread mode, exception return uses floating-point state from
// MSP and execution uses MSP after return.
const RETURN_TO_THREAD_MODE_NO_FP_MSP: u32 = 0xFFFFFFE9;

// Return to Thread mode, exception return uses floating-point state from PSP
// and execution uses PSP after return.
const RETURN_TO_THREAD_MODE_FP_PSP: u32 = 0xFFFFFFED;

#[repr(C, align(16))]
#[derive(Copy, Clone)]
pub struct stack_frame {
    align: u32,
    //align1: u32,
    R0: u32,
    R1: u32,
    R2: u32,
    R3: u32,
    R12: u32,
    LR: u32,
    PC: u32,
    xPSR: u32,
    // aligned to 16
}

impl stack_frame {
    const fn new() -> stack_frame {
        stack_frame {
            align: 0,
            // align1: 0,
            xPSR: 0,
            PC: 0,
            LR: 0,
            R12: 0,
            R3: 0,
            R2: 0,
            R1: 0,
            R0: 0,
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

#[app(device = crate::hal::target)]
const APP: () = {
    static mut TOCKRAM: [stack_frame; 10] = [stack_frame::new(); 10];

    #[init]
    fn init() {
        hprintln!("init").unwrap();
        // rtfm::pend(interrupt::SWI0_EGU0);
    }
    #[idle(resources = [TOCKRAM])]
    fn idle() -> ! {
        hprintln!("idle").unwrap();

        let user_stack = stack_frame {
            R0: 0,   
            R1: 1,
            R2: 2,
            R3: 3,
            R12: 12,
            LR: 0x0000_0001,
            PC: tock_fn as u32,
            xPSR: 0x0100_0000,
            align: 0,
            // align1: 0,
        };

        resources.TOCKRAM[9] = user_stack;
        hprintln!("psp = {:8x?}", (&resources.TOCKRAM[9].xPSR) as *const u32);
        hprintln!("PC = {:8x?}", resources.TOCKRAM[9].PC);

        unsafe {
            asm!("
                // Set thread mode to unprivileged 
                // mov r0, #1
                // msr CONTROL, r0
                msr psp, r1
                svc #124"
                : : "{r1}" (&resources.TOCKRAM[9].R0) : : "volatile");
        }

        loop {}
    }

    #[exception]
    fn SVCall() {
        hprintln!("SVCALL");
        unsafe {
            asm!(
                "
                // Return to Thread mode, exception return uses non-floating-point state from
                // the PSP and execution uses PSP after return.
                movw lr, #0xfffd
                movt lr, #0xffff
                bx lr"
                    : : : : "volatile"
            );
        }

        loop {}
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
    hprintln!("ef : {:?}", ef);

    loop {}
}

//#[naked]
fn tock_fn() {
//    hprintln!("tock");
    //asm::bkpt();
    loop {
         hprintln!("tock");
         //atomic::compiler_fence(Ordering::SeqCst);
    }
}
