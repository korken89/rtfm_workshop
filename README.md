# `RTFM Workshop @ Oxidize 2019`

> Examples and exercises for the DecaWave DWM1001-DEV devkits utilized at the
> RTFM Workshop held at Oxidize 2019, Berlin.

---

## Installation instructions

For installation of all required software to run the `DWM1001-DEV` board,
follow the instruction here: [Installation instruction], and the verify with the
[Verification instructions].

[Installation instruction]: https://github.com/ferrous-systems/embedded-trainings/blob/master/INSTALL.md
[Verification instructions]: https://github.com/ferrous-systems/embedded-trainings/blob/master/VERIFY.md

The `DWM1001-DEV` uses a JLink debugger onboard.

---

## Example using JLink GDB Server

### Hello World! Building and Debugging an Application

1. Connect your devkit using USB. To check that it is found you can run:

``` console
$ lsusb
...
Bus 001 Device 005: ID 1366:0105 SEGGER
...
```

(Bus/Device/ID may vary.)

2. In a terminal in the `app` folder run:

``` console
$ JLinkGDBServer -device NRF52832_XXAA -if SWD -speed 4000
...
Connecting to J-Link...
J-Link is connected.
Firmware: J-Link OB-STM32F072-128KB-CortexM compiled Jan  7 2019 14:08:04
Hardware: V1.00
S/N: 760040473
Checking target voltage...
Target voltage: 3.30 V
Listening on TCP/IP port 2331
Connecting to target...Connected to target
Waiting for GDB connection...
...
```

3. In another terminal (in the same `app` folder) run:

``` console
$ cargo run --example minimal
```

This starts GDB with a connection to JLink, which loads (flashes) the binary to the target (our devkit). The script  sets `breakpoint`s at `main` as well as some exception handlers, enables `semihosting`, loads the binary and finally runs the first instruction (`stepi`). Exactly what is run can be found in the `jlink.gdb` file.

4. You can now continue debugging of the program:

``` console
(gdb) c
Continuing.

Breakpoint 3, main () at examples/minimal.rs:18
18          hprintln!("hello").unwrap();
```

The `cortex-m-rt` run-time initializes the system and your global variables (in this case there are none). After that it calls the `[entry]` function. Here you hit a breakpoint.

5. You can continue debugging:

``` console
(gdb) c
Continuing.
```

At this point, the GDB terminal should read:

``` console
hello
```

Your program is now stuck in an infinite loop (doing nothing).

6. Press `CTRL-c` in the `gdb` terminal:

``` console
Program received signal SIGINT, Interrupt.
0x08000276 in main () at examples/minimal.rs:20
20          loop {}
(gdb)
```

---

## Example using OpenOCD Server

### Hello World! Building and Debugging an Application

1. Connect your devkit using USB. To check that it is found you can run:

``` console
$ lsusb
...
Bus 001 Device 005: ID 1366:0105 SEGGER
...
```

(Bus/Device/ID may vary.)

2. In a terminal in the `app` folder run:

``` console
$ cargo build --example minimal
$ openocd -f openocd_jlink.cfg
...
Info : J-Link OB-STM32F072-128KB-CortexM compiled Jan  7 2019 14:08:04
Info : Hardware version: 1.00
Info : VTarget = 3.300 V
...
```

3. In another terminal (in the same `app` folder) run:

``` console
$ arm-none-eabi-gdb target/thumbv7em-none-eabihf/debug/examples/minimal -x jlink.gdb
```

This starts GDB with a connection to the JLink via OpenOCD, which loads (flashes) the binary to the target (our devkit). The script  sets `breakpoint`s at `main` as well as some exception handlers, enables `semihosting`, loads the binary and finally runs the first instruction (`stepi`). Exactly what is run can be found in the `jlink.gdb` file.

4. You can now continue debugging of the program:

``` console
(gdb) c
Continuing.

Breakpoint 3, main () at examples/minimal.rs:18
18          hprintln!("hello").unwrap();
```

The `cortex-m-rt` run-time initializes the system and your global variables (in this case there are none). After that it calls the `[entry]` function. Here you hit a breakpoint.

5. You can continue debugging:

``` console
(gdb) c
Continuing.
```

At this point, the GDB terminal should read:

``` console
hello
```

Your program is now stuck in an infinite loop (doing nothing).

6. Press `CTRL-c` in the `gdb` terminal:

``` console
Program received signal SIGINT, Interrupt.
0x08000276 in main () at examples/minimal.rs:20
20          loop {}
(gdb)
```

You have now compiled and debugged a minimal Rust example! `gdb` is a very useful tool so lookup some tutorials/docs (e.g., https://sourceware.org/gdb/onlinedocs/gdb/), a Cheat Sheet can be found at https://darkdust.net/files/GDB%20Cheat%20Sheet.pdf.

---

## Trouble Shooting

Working with embedded targets involves a lot of tooling, and many things can go wrong.

---

### `openocd` fails to connect

If you end up with a program that puts the MCU in a bad state.

- Check so the board in connected.
- Ask an instructor.

---

### `gdb` fails to connect

JLink acts as a *gdb server*, while `gdb` is a *gdb client*. By default they connect over port `:2331` (: indicates that the port is on the *localhost*, not a remote connection). In cases you might have another `gdb` connection blocking the port.

``` console
$ ps -all
F S   UID   PID  PPID  C PRI  NI ADDR SZ WCHAN  TTY          TIME CMD
0 S  1000  7549 16215  0  80   0 - 25930 se_sys pts/4    00:00:00 arm-none-eabi-gdb
...
```

In this case you can try killing `gdb` by:

``` console
$ kill -9 7549
```

or even

``` console
$ killall -9 arm-none-eabi-gdb
```

---

## Visual Studio Code

`vscode` is highly configurable, (keyboard shortcuts, keymaps, plugins etc.) There is Rust support through the `rls-vscode` plugin (https://github.com/rust-lang/rls-vscode).

It is possible to run `arm-none-eabi-gdb` from within the `vscode` using the `cortex-debug` plugin (https://marketplace.visualstudio.com/items?itemName=marus25.cortex-debug).

For general informaiton regarding debugging in `vscode`, see https://code.visualstudio.com/docs/editor/debugging.

Some useful (default) shortcuts:

- `CTRL+SHIFT+b` compilation tasks, (e.g., compile all examples `cargo build --examples`). Cargo is smart and just re-compiles what is changed.

- `CTRL+SHIFT+d` debug launch configurations, enter debug mode to choose a binary (e.g., `itm 64MHz (debug)`)

- `F5` to start. It will open the `cortex_m_rt/src/lib.rs` file, which contains the startup code. From there you can continue `F5` again.
- `F6` to break. The program will now be in the infinite loop (for this example). In general it will just break wherever the program counter happens to be.
- You can view the ITM trace in the `OUTPUT` tab, choose the dropdown `SWO: ITM [port 0, type console]`. It should now display:

``` txt
[2019-01-02T21:35:26.457Z]   Hello, world!
```

- `SHIFT-F5` shuts down the debugger.

You may step, view the current context `variables`, add `watches`, inspect the `call stack`, add `breakpoints`, inspect `peripherals` and `registers`. Read more in the documentation for the plugin.

### Caveats

Visual Studio Code is not an "IDE", its a text editor with plugin support, with an API somewhat limiting what can be done from within a plugin (in comparison to Eclipse, IntelliJ...) regarding panel layouts etc. E.g., as far as I know you cannot view the `adapter output` (`openocd`) at the same time as the ITM trace, they are both under the `OUTPUT` tab. Moreover, each time you re-start a debug session, you need to re-select the `SWO: Name [port 0, type console]` to view the ITM output. There are some `hax` around this:

- Never shut down the debug session. Instead use the `DEBUG CONSOLE` (`CTRL+SHIFT+Y`) to get to the `gdb` console. This is not the *full* `gdb` interactive shell with some limitations (no *tab* completion e.g.). Make sure the MCU is stopped (`F6`). The console should show something like:

``` txt
Program
 received signal SIGINT, Interrupt.
0x0800056a in main () at examples/itm.rs:31
31	    loop {}
```

- Now you can edit an re-compile your program, e.g. changing the text:

> iprintln!(stim, "Hello, again!");

- In the `DEBUG CONSOLE`, write `load` press `ENTER` write `monitor reset init` press `ENTER`.

``` txt
load
{"token":97,"outOfBandRecord":[{"isStream":false,"type":"status","asyncClass":"download","output":[]}]}
`/home/pln/rust/app/target/thumbv7em-none-eabihf/debug/examples/itm' has changed; re-reading symbols.
Loading section .vector_table, size 0x400 lma 0x8000000
Loading section .text, size 0x10c8 lma 0x8000400
Loading section .rodata, size 0x2a8 lma 0x80014d0
Start address 0x8001298, load size 6000
Transfer rate: 9 KB/sec, 2000 bytes/write.

mon reset init
{"token":147,"outOfBandRecord":[],"resultRecords":{"resultClass":"done","results":[]}}
adapter speed: 2000 kHz
target halted due to debug-request, current mode: Thread
xPSR: 0x01000000 pc: 0x08001298 msp: 0x20018000
adapter speed: 8000 kHz
```

- The newly compiled binary is now loaded and you can continue (`F5`). Switching to the `OUTPUT` window now preserves the ITM view and displays both traces:

``` txt
[2019-01-02T21:43:27.988Z]   Hello, world!
[2019-01-02T22:07:29.090Z]   Hello, again!
```

- Using the `gdb` terminal (`DEBUG CONSOLE`) from within `vscode` is somewhat instable/experimental. E.g., `CTRL+c` does not `break` the target (use `F6`, or write `interrupt`). The `contiune` command, indeed continues execution (and the *control bar* changes mode, but you cannot `break` using neither `F6` nor `interrupt`). So it seems that the *state* of the `cortex-debug` plugin is not correctly updated. Moreover setting breakpoints from the `gdb` terminal indeed informs `gdb` about the breakpoint, but the state in `vscode` is not updated, so be aware.

---

### Vscode Launch Configurations

Some example launch configurations from the `.vscode/launch.json` file:

``` json
 {
            "type": "cortex-debug",
            "request": "launch",
            "servertype": "openocd",
            "name": "itm 64Mhz (debug)",
            "executable": "./target/thumbv7em-none-eabihf/debug/examples/itm",
            "configFiles": [
                "interface/stlink.cfg",
                "target/stm32f4x.cfg"
            ],
            "postLaunchCommands": [
                "monitor reset init"
            ],
            "swoConfig": {
                "enabled": true,
                "cpuFrequency": 64000000,
                "swoFrequency": 2000000,
                "source": "probe",
                "decoders": [
                    {
                        "type": "console",
                        "label": "ITM",
                        "port": 0
                    }
                ]
            },
            "cwd": "${workspaceRoot}"
        },
        {
            "type": "cortex-debug",
            "request": "launch",
            "servertype": "openocd",
            "name": "hello 16Mhz (debug)",
            "executable": "./target/thumbv7em-none-eabihf/debug/examples/hello",
            "configFiles": [
                "interface/stlink.cfg",
                "target/stm32f4x.cfg"
            ],
            "postLaunchCommands": [
                "monitor arm semihosting enable"
            ],
            "cwd": "${workspaceRoot}"
        },

      {
            "type": "cortex-debug",
            "request": "launch",
            "servertype": "openocd",
            "name": "itm 16Mhz (debug)",
            "executable": "./target/thumbv7em-none-eabihf/debug/examples/itm",
            // uses local config files
            "configFiles": [
                "./stlink.cfg",
                "./stm32f4x.cfg"
            ],
            "postLaunchCommands": [
                "monitor reset init"
            ],
            "swoConfig": {
                "enabled": true,
                "cpuFrequency": 16000000,
                "swoFrequency": 2000000,
                "source": "probe",
                "decoders": [
                    {
                        "type": "console",
                        "label": "ITM",
                        "port": 0
                    }
                ]
            },
            "cwd": "${workspaceRoot}"
        },
```

We see some similarities to the `openocd.gdb` file, we don't need to explicitly connect to the target (that is automatic). Also launching `openocd` is automatic (for good and bad, its re-started each time). `postLaunchCommands` allows arbitrary commands to be executed by `gdb` once the session is up. E.g. in the `hello` case we enable `semihosting`, while in the `itm` case we run `monitor reset init` to get the MCU in 64MHz (first example) or 16MHz (third example), before running the application (continue). Notice the first example uses the "stock" `openocd` configuration files, while the third example uses our local configuration files (that does not change the core frequency).

---

## GDB Advanced Usage

There are numerous ways to automate `gdb`. Scripts can be run by the `gdb` command `source` (`so` for short). Scripting common tasks like setting breakpoints, dumping some memory region etc. can be really helpful.
