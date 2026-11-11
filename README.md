# Raspberry Pi Pico 2 running no_std Rust

Instructions:

- Follow the instructions at https://github.com/chrislattman/test-bare-metal-pico to install `picotool` if you haven't already

To build application and run on board:

- Unplug USB cable from board
- Hold down BOOTSEL button while plugging in USB cable
- Run `cargo run --release`

To run on the RP2350's RISC-V (Hazard3) cores instead of the Cortex-M33 cores, do the same but run `cargo run-riscv` (a release build for `riscv32imac-unknown-none-elf`). The bootrom switches the cores to RISC-V based on the image. The QEMU targets below are Arm-only.

To run in QEMU:

- Run `cargo qemu`, and in another shell `telnet localhost 5678`
- `cargo qemu-debug` does the same with a debug build, but halts at startup and waits for GDB on localhost:1234. See .cargo/config.toml for details.

Note: this example does NOT work for the Raspberry Pi Pico 2 W, as that on-board LED is accessed through the CYW43439 Wi-Fi/Bluetooth chip.
