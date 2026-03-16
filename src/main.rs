#![no_std]
#![no_main]

use embedded_hal::delay::DelayNs;
use embedded_hal::digital::OutputPin;
use panic_halt as _;
use rp235x_hal as hal;
use usb_device::{class_prelude::*, prelude::*};
use usbd_serial::SerialPort;

/// Tell the Boot ROM about our application (copies the boot metadata to .start_block)
#[unsafe(link_section = ".start_block")]
#[used]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

/// External crystal frequency
const XTAL_FREQ_HZ: u32 = 12_000_000u32;

#[hal::entry]
fn main() -> ! {
    // Get ownership of peripheral access crates
    let mut pac = hal::pac::Peripherals::take().unwrap();

    // Set up watchdog and clocks
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);
    let clocks = hal::clocks::init_clocks_and_plls(
        XTAL_FREQ_HZ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .unwrap();

    // Single-cycle I/O block (fast GPIO)
    let sio = hal::Sio::new(pac.SIO);

    // Set pins to default state
    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );
    let mut led_pin = pins.gpio25.into_push_pull_output();

    // Initialize timer
    let mut timer = hal::Timer::new_timer0(pac.TIMER0, &mut pac.RESETS, &clocks);

    // Initialize USB driver
    let usb_bus = UsbBusAllocator::new(hal::usb::UsbBus::new(
        pac.USB,
        pac.USB_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));

    // Configure the USB as Communications Device Class
    let mut serial = SerialPort::new(&usb_bus);

    // Create a USB device with a fake VID/PID
    let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x16c0, 0x27dd))
        .strings(&[StringDescriptors::default()
            .manufacturer("Fake company") // Typically "Raspberry Pi"
            .product("Serial port")
            .serial_number("TEST")])
        .unwrap()
        .device_class(usbd_serial::USB_CLASS_CDC)
        .build();

    let mut tick_10s = timer.get_counter();
    let mut tick_hello = tick_10s.clone();
    timer.delay_ms(500);
    let mut tick_world = timer.get_counter();

    loop {
        usb_dev.poll(&mut [&mut serial]);
        if (timer.get_counter() - tick_10s).to_millis() >= 10_000 {
            let _ = serial.write(b"10 second timer went off\r\n");
            tick_10s = timer.get_counter();
        } else if (timer.get_counter() - tick_hello).to_millis() >= 1_000 {
            let _ = serial.write(b"Hello\r\n");
            let _ = led_pin.set_low();
            tick_hello = timer.get_counter();
        } else if (timer.get_counter() - tick_world).to_millis() >= 1_000 {
            let _ = serial.write(b"World!\r\n");
            let _ = led_pin.set_high();
            tick_world = timer.get_counter();
        }
    }
}
