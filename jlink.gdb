target extended-remote :2331

set print asm-demangle on

load

monitor reset

# detect unhandled exceptions, hard faults and panics
break HardFault
break rust_begin_unwind

# *try* to stop at the user entry point (it might be gone due to inlining)
break main

monitor semihosting enable
monitor semihosting ioclient 3

# start the process but immediately halt the processor
stepi
