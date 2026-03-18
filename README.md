# Raspberry Pi Pico 2 running no_std Rust

Instructions:

- Follow the instructions at https://github.com/chrislattman/test-bare-metal-pico to install picotool if you haven't already
- Run `rustup target add thumbv8m.main-none-eabihf`

To build application and run on board:

- Unplug USB cable from board
- Hold down BOOTSEL button while plugging in USB cable
- Run `cargo run --release`

Note: this example does NOT work for the Raspberry Pi Pico 2 W, as that on-board LED is accessed through the CYW43439 Wi-Fi/Bluetooth chip.
