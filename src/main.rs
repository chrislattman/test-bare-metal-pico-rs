#![no_std]
#![no_main]

#[cfg(all(feature = "qemu", feature = "rp2350"))]
compile_error!("the qemu feature needs --no-default-features (use `cargo qemu`)");

use core::panic::PanicInfo;

use embedded_hal::digital::OutputPin;
// In QEMU, src/qemu_hal.rs stands in for rp235x-hal under the same name
#[cfg(feature = "qemu")]
mod qemu_hal;
#[cfg(feature = "qemu")]
use qemu_hal as rp235x_hal;

use rp235x_hal as hal;
#[cfg(not(feature = "qemu"))]
use usb_device::{class_prelude::*, prelude::*};
#[cfg(not(feature = "qemu"))]
use usbd_serial::SerialPort;

// The timer (alarm interrupt, delays and timed messages) is only used in release builds.
// Debug builds write to serial and blink the LED on every loop iteration instead, so every
// line can be reached by stepping in GDB (the timer doesn't advance while the CPU is halted).
#[cfg(not(debug_assertions))]
use core::cell::RefCell;
#[cfg(not(debug_assertions))]
use cortex_m::peripheral::NVIC;
#[cfg(not(debug_assertions))]
use critical_section::Mutex;
#[cfg(not(debug_assertions))]
use embedded_hal::delay::DelayNs;
#[cfg(not(debug_assertions))]
use fugit::MicrosDurationU32;
#[cfg(not(debug_assertions))]
use hal::{
    pac::interrupt,
    timer::{Alarm, Alarm0, CopyableTimer0},
};

/// Tell the Boot ROM about our application (copies the boot metadata to .start_block)
#[unsafe(link_section = ".start_block")]
#[used]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

/// External crystal frequency
const XTAL_FREQ_HZ: u32 = 12_000_000u32;

#[cfg(not(debug_assertions))]
type LedPin = hal::gpio::Pin<
    hal::gpio::bank0::Gpio25,
    hal::gpio::FunctionSio<hal::gpio::SioOutput>,
    hal::gpio::PullDown,
>;
#[cfg(not(debug_assertions))]
static G_ALARM: Mutex<RefCell<Option<Alarm0<CopyableTimer0>>>> = Mutex::new(RefCell::new(None));
#[cfg(not(debug_assertions))]
static G_LED: Mutex<RefCell<Option<LedPin>>> = Mutex::new(RefCell::new(None));

// Interrupt service routine (ISR)
// This is supposed to get called but there's something wrong with my code
#[cfg(not(debug_assertions))]
#[interrupt]
fn TIMER0_IRQ_0() {
    critical_section::with(|cs| {
        // Borrow the alarm and LED from global state
        let mut alarm_ref = G_ALARM.borrow(cs).borrow_mut();
        let mut led_pin_ref = G_LED.borrow(cs).borrow_mut();

        // Get mutable references
        let alarm = alarm_ref.as_mut().unwrap();
        let led_pin = led_pin_ref.as_mut().unwrap();

        alarm.clear_interrupt();
        led_pin.set_high();

        // Schedule next interrupt in 6 seconds
        alarm.schedule(MicrosDurationU32::secs(6)).unwrap();
    });
}

#[hal::entry]
fn main() -> ! {
    // Get ownership of peripheral access crate
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
    #[cfg(not(debug_assertions))]
    let mut timer = hal::Timer::new_timer0(pac.TIMER0, &mut pac.RESETS, &clocks);

    // Initialize USB driver
    #[cfg(not(feature = "qemu"))]
    let usb_bus = UsbBusAllocator::new(hal::usb::UsbBus::new(
        pac.USB,
        pac.USB_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));

    // Configure the USB as Communications Device Class
    #[cfg(not(feature = "qemu"))]
    let mut serial = SerialPort::new(&usb_bus);

    // Create a USB device with a fake VID/PID
    #[cfg(not(feature = "qemu"))]
    let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x16c0, 0x27dd))
        .strings(&[StringDescriptors::default()
            .manufacturer("Fake company") // Typically "Raspberry Pi"
            .product("Serial port")
            .serial_number("TEST")])
        .unwrap()
        .device_class(usbd_serial::USB_CLASS_CDC)
        .build();

    // QEMU has no USB host, so UART0 stands in for the USB serial port
    #[cfg(feature = "qemu")]
    let mut serial = hal::uart::Uart::new(pac.UART0, &clocks);

    led_pin.set_high();

    // Configure alarm to trigger in 1.25 seconds
    #[cfg(not(debug_assertions))]
    {
        let mut alarm = timer.alarm_0().unwrap();
        alarm
            .schedule(MicrosDurationU32::micros(1_250_000))
            .unwrap();
        alarm.enable_interrupt();
        critical_section::with(|cs| {
            G_ALARM.borrow(cs).replace(Some(alarm));
            G_LED.borrow(cs).replace(Some(led_pin));
        });
        unsafe {
            NVIC::unmask(hal::pac::Interrupt::TIMER0_IRQ_0);
        }
    }

    // Not compiled in debug builds, so pausing in GDB doesn't trigger a reboot
    #[cfg(not(debug_assertions))]
    watchdog.start(MicrosDurationU32::secs(4));

    // Initialize timer counters
    #[cfg(not(debug_assertions))]
    let mut tick_10s = timer.get_counter();
    #[cfg(not(debug_assertions))]
    let mut tick_hello = tick_10s;
    #[cfg(not(debug_assertions))]
    timer.delay_ms(1_000);
    #[cfg(not(debug_assertions))]
    let mut tick_world = timer.get_counter();

    loop {
        #[cfg(not(feature = "qemu"))]
        usb_dev.poll(&mut [&mut serial]);

        #[cfg(not(debug_assertions))]
        if (timer.get_counter() - tick_10s).to_millis() >= 10_000 {
            serial.write(b"10 second timer went off\r\n").unwrap();
            tick_10s = timer.get_counter();
        } else if (timer.get_counter() - tick_hello).to_millis() >= 2_000 {
            serial.write(b"Hello\r\n").unwrap();
            // Critical sections are required because led_pin is guarded by a mutex
            critical_section::with(|cs| {
                let mut led_pin_ref = G_LED.borrow(cs).borrow_mut();
                let led_pin = led_pin_ref.as_mut().unwrap();
                led_pin.set_low();
            });
            tick_hello = timer.get_counter();
        } else if (timer.get_counter() - tick_world).to_millis() >= 2_000 {
            serial.write(b"World!\r\n").unwrap();
            // Critical sections are required because led_pin is guarded by a mutex
            critical_section::with(|cs| {
                let mut led_pin_ref = G_LED.borrow(cs).borrow_mut();
                let led_pin = led_pin_ref.as_mut().unwrap();
                led_pin.set_high();
            });
            tick_world = timer.get_counter();
        }

        #[cfg(debug_assertions)]
        {
            serial.write(b"Hello\r\n").unwrap();
            led_pin.set_low();
            serial.write(b"World!\r\n").unwrap();
            led_pin.set_high();
        }

        #[cfg(not(debug_assertions))]
        watchdog.feed();
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
