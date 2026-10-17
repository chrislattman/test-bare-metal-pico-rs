# Raspberry Pi Pico 2 running no_std Rust

Instructions:

- Follow the instructions at https://github.com/chrislattman/test-bare-metal-pico to install `picotool` if you haven't already
- Run `rustup target add thumbv8m.main-none-eabihf`

To build application and run on board:

- Unplug USB cable from board
- Hold down BOOTSEL button while plugging in USB cable
- Run `cargo run --release`

To run in QEMU:

- Run `rustup target add thumbv8m.main-none-eabi`
- Run `cargo qemu`, and in another shell `telnet localhost 5678`
- `cargo qemu-debug` does the same with a debug build, but halts at startup and waits for GDB on localhost:1234. See .cargo/config.toml for details.

Note: this example does NOT work for the Raspberry Pi Pico 2 W, as that on-board LED is accessed through the CYW43439 Wi-Fi/Bluetooth chip.
