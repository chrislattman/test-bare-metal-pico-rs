#![no_std]
#![no_main]

use core::{cell::RefCell, panic::PanicInfo};

use cortex_m::peripheral::NVIC;
use critical_section::Mutex;
use embedded_hal::{delay::DelayNs, digital::OutputPin};
use fugit::MicrosDurationU32;
use rp235x_hal::{
    self as hal,
    pac::interrupt,
    timer::{Alarm, Alarm0, CopyableTimer0},
};
use usb_device::{class_prelude::*, prelude::*};
use usbd_serial::SerialPort;

/// Tell the Boot ROM about our application (copies the boot metadata to .start_block)
#[unsafe(link_section = ".start_block")]
#[used]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

/// External crystal frequency
const XTAL_FREQ_HZ: u32 = 12_000_000u32;

type LedPin = hal::gpio::Pin<
    hal::gpio::bank0::Gpio25,
    hal::gpio::FunctionSio<hal::gpio::SioOutput>,
    hal::gpio::PullDown,
>;
static G_ALARM: Mutex<RefCell<Option<Alarm0<CopyableTimer0>>>> = Mutex::new(RefCell::new(None));
static G_LED: Mutex<RefCell<Option<LedPin>>> = Mutex::new(RefCell::new(None));

// Interrupt service routine (ISR)
// This is supposed to get called but there's something wrong with my code
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
        let _ = alarm.schedule(MicrosDurationU32::micros(6_000_000));
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

    led_pin.set_high();

    // Configure alarm to trigger in 1.25 seconds
    let mut alarm = timer.alarm_0().unwrap();
    let _ = alarm.schedule(MicrosDurationU32::micros(1_250_000));
    alarm.enable_interrupt();
    critical_section::with(|cs| {
        G_ALARM.borrow(cs).replace(Some(alarm));
        G_LED.borrow(cs).replace(Some(led_pin));
    });
    unsafe {
        NVIC::unmask(hal::pac::Interrupt::TIMER0_IRQ_0);
    }

    // Initialize timer counters
    let mut tick_10s = timer.get_counter();
    let mut tick_hello = tick_10s;
    timer.delay_ms(1_000);
    let mut tick_world = timer.get_counter();

    loop {
        usb_dev.poll(&mut [&mut serial]);
        if (timer.get_counter() - tick_10s).to_millis() >= 10_000 {
            let _ = serial.write(b"10 second timer went off\r\n");
            tick_10s = timer.get_counter();
        } else if (timer.get_counter() - tick_hello).to_millis() >= 2_000 {
            let _ = serial.write(b"Hello\r\n");
            // Critical sections are required because led_pin is guarded by a mutex
            critical_section::with(|cs| {
                let mut led_pin_ref = G_LED.borrow(cs).borrow_mut();
                let led_pin = led_pin_ref.as_mut().unwrap();
                led_pin.set_low();
            });
            tick_hello = timer.get_counter();
        } else if (timer.get_counter() - tick_world).to_millis() >= 2_000 {
            let _ = serial.write(b"World!\r\n");
            // Critical sections are required because led_pin is guarded by a mutex
            critical_section::with(|cs| {
                let mut led_pin_ref = G_LED.borrow(cs).borrow_mut();
                let led_pin = led_pin_ref.as_mut().unwrap();
                led_pin.set_high();
            });
            tick_world = timer.get_counter();
        }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
