//! Stand-in for the subset of `rp235x-hal` used by `main.rs`, for running in QEMU's `mps2-an521`
//! machine (Arm SSE-200, Cortex-M33 in secure state). QEMU has no RP2350 machine.
//!
//! The types mirror the `rp235x-hal` API but are implemented on the peripherals QEMU models:
//!
//! - `Timer::get_counter()` and `DelayNs` -> SysTick at 1 ms (runs at real-time speed in QEMU)
//! - `Alarm0` -> CMSDK APB TIMER0 (interrupt 3, handled by `TIMER0_IRQ_0`)
//! - `Watchdog` -> the SSE-200 secure CMSDK watchdog (the board reboots if it isn't fed in time)
//! - `uart::Uart` -> CMSDK APB UART0, which stands in for the USB serial port
//! - clocks, SIO and GPIO -> no-ops (there's no LED to see in QEMU)

// Like the real HAL, not everything here is used by every build (e.g. debug builds don't use the
// timer or the watchdog)
#![allow(dead_code, unused_imports)]

use core::ptr::{read_volatile, write_volatile};

pub use cortex_m_rt::entry;

/// QEMU clocks the AN521 CPU, SysTick, timers and watchdogs at 20 MHz
const SYSCLK_HZ: u32 = 20_000_000;

fn write_reg(addr: u32, value: u32) {
    unsafe { write_volatile(addr as *mut u32, value) }
}

fn read_reg(addr: u32) -> u32 {
    unsafe { read_volatile(addr as *const u32) }
}

#[allow(non_camel_case_types)]
pub mod pac {
    use core::sync::atomic::{AtomicBool, Ordering};

    pub use self::Interrupt as interrupt;
    pub use cortex_m_rt::interrupt;

    pub struct WATCHDOG;
    pub struct XOSC;
    pub struct CLOCKS;
    pub struct PLL_SYS;
    pub struct PLL_USB;
    pub struct RESETS;
    pub struct SIO;
    pub struct IO_BANK0;
    pub struct PADS_BANK0;
    pub struct TIMER0;
    pub struct UART0;

    #[allow(non_snake_case)]
    pub struct Peripherals {
        pub WATCHDOG: WATCHDOG,
        pub XOSC: XOSC,
        pub CLOCKS: CLOCKS,
        pub PLL_SYS: PLL_SYS,
        pub PLL_USB: PLL_USB,
        pub RESETS: RESETS,
        pub SIO: SIO,
        pub IO_BANK0: IO_BANK0,
        pub PADS_BANK0: PADS_BANK0,
        pub TIMER0: TIMER0,
        pub UART0: UART0,
    }

    static TAKEN: AtomicBool = AtomicBool::new(false);

    impl Peripherals {
        pub fn take() -> Option<Self> {
            if TAKEN.swap(true, Ordering::AcqRel) {
                return None;
            }
            Some(Peripherals {
                WATCHDOG,
                XOSC,
                CLOCKS,
                PLL_SYS,
                PLL_USB,
                RESETS,
                SIO,
                IO_BANK0,
                PADS_BANK0,
                TIMER0,
                UART0,
            })
        }
    }

    /// SSE-200 interrupt numbers, named after the RP2350 interrupts they stand in for
    #[derive(Clone, Copy)]
    #[repr(u16)]
    pub enum Interrupt {
        /// CMSDK APB TIMER0
        TIMER0_IRQ_0 = 3,
    }

    unsafe impl cortex_m::interrupt::InterruptNumber for Interrupt {
        fn number(self) -> u16 {
            self as u16
        }
    }

    // Device interrupt vector table, appended to the core exceptions by cortex-m-rt's link.x.
    // Handler names default to DefaultHandler in qemu/device.x.
    unsafe extern "C" {
        fn DefaultHandler();
        fn TIMER0_IRQ_0();
    }

    #[repr(C)]
    pub union Vector {
        handler: unsafe extern "C" fn(),
    }

    #[unsafe(link_section = ".vector_table.interrupts")]
    #[unsafe(no_mangle)]
    pub static __INTERRUPTS: [Vector; 4] = [
        Vector {
            handler: DefaultHandler,
        }, // 0: non-secure watchdog reset request
        Vector {
            handler: DefaultHandler,
        }, // 1: non-secure watchdog
        Vector {
            handler: DefaultHandler,
        }, // 2: S32K timer
        Vector {
            handler: TIMER0_IRQ_0,
        }, // 3: TIMER0
    ];
}

pub mod block {
    /// Boot ROM metadata has no meaning in QEMU; it's only kept so `main.rs` builds unchanged
    pub struct ImageDef(#[allow(dead_code)] u32);

    impl ImageDef {
        pub const fn secure_exe() -> Self {
            ImageDef(0)
        }
    }
}

pub mod clocks {
    use super::pac;
    use core::convert::Infallible;

    pub struct ClocksManager;

    /// QEMU's clocks are fixed, so there's nothing to configure
    pub fn init_clocks_and_plls(
        _xosc_crystal_freq: u32,
        _xosc_dev: pac::XOSC,
        _clocks_dev: pac::CLOCKS,
        _pll_sys_dev: pac::PLL_SYS,
        _pll_usb_dev: pac::PLL_USB,
        _resets: &mut pac::RESETS,
        _watchdog: &mut super::Watchdog,
    ) -> Result<ClocksManager, Infallible> {
        Ok(ClocksManager)
    }
}

pub struct Sio {
    pub gpio_bank0: gpio::SioGpioBank0,
}

impl Sio {
    pub fn new(_sio: pac::SIO) -> Self {
        Sio {
            gpio_bank0: gpio::SioGpioBank0,
        }
    }
}

pub mod gpio {
    use super::pac;
    use core::{convert::Infallible, marker::PhantomData};

    pub mod bank0 {
        pub struct Gpio25;
    }

    pub struct FunctionNull;
    pub struct FunctionSio<C>(PhantomData<C>);
    pub struct SioOutput;
    pub struct PullDown;
    pub struct SioGpioBank0;

    pub struct Pin<I, F, P>(PhantomData<(I, F, P)>);

    impl<I, F, P> Pin<I, F, P> {
        pub fn into_push_pull_output(self) -> Pin<I, FunctionSio<SioOutput>, P> {
            Pin(PhantomData)
        }
    }

    /// There's nothing to see in QEMU, so setting the pin does nothing
    impl<I, P> embedded_hal::digital::ErrorType for Pin<I, FunctionSio<SioOutput>, P> {
        type Error = Infallible;
    }

    impl<I, P> embedded_hal::digital::OutputPin for Pin<I, FunctionSio<SioOutput>, P> {
        fn set_low(&mut self) -> Result<(), Infallible> {
            Ok(())
        }

        fn set_high(&mut self) -> Result<(), Infallible> {
            Ok(())
        }
    }

    pub struct Pins {
        pub gpio25: Pin<bank0::Gpio25, FunctionNull, PullDown>,
    }

    impl Pins {
        pub fn new(
            _io: pac::IO_BANK0,
            _pads: pac::PADS_BANK0,
            _sio: SioGpioBank0,
            _reset: &mut pac::RESETS,
        ) -> Self {
            Pins {
                gpio25: Pin(PhantomData),
            }
        }
    }
}

pub mod timer {
    use super::{SYSCLK_HZ, pac, read_reg, write_reg};
    use core::{
        convert::Infallible,
        marker::PhantomData,
        sync::atomic::{AtomicU32, Ordering},
    };
    use cortex_m_rt::exception;
    use fugit::{MicrosDurationU32, TimerInstantU64};

    pub type Instant = TimerInstantU64<1_000_000>;

    // SysTick (secure)
    const SYST_CSR: u32 = 0xE000_E010;
    const SYST_RVR: u32 = 0xE000_E014;
    const SYST_CVR: u32 = 0xE000_E018;
    const SYST_RELOAD: u32 = SYSCLK_HZ / 1_000 - 1; // 1 ms tick

    // CMSDK APB TIMER0 (secure alias)
    const TIMER0_CTRL: u32 = 0x5000_0000;
    const TIMER0_VALUE: u32 = 0x5000_0004;
    const TIMER0_RELOAD: u32 = 0x5000_0008;
    const TIMER0_INTCLEAR: u32 = 0x5000_000C;
    const TIMER_CTRL_EN: u32 = 1 << 0;
    const TIMER_CTRL_INTEN: u32 = 1 << 3;

    static MS_TICKS: AtomicU32 = AtomicU32::new(0);

    #[exception]
    fn SysTick() {
        MS_TICKS.fetch_add(1, Ordering::Relaxed);
    }

    #[derive(Clone, Copy)]
    pub struct CopyableTimer0;

    pub struct Timer<D> {
        alarm_taken: bool,
        _device: PhantomData<D>,
    }

    impl Timer<CopyableTimer0> {
        pub fn new_timer0(
            _timer: pac::TIMER0,
            _resets: &mut pac::RESETS,
            _clocks: &super::clocks::ClocksManager,
        ) -> Self {
            write_reg(SYST_RVR, SYST_RELOAD);
            write_reg(SYST_CVR, 0);
            write_reg(SYST_CSR, 0x7); // processor clock, interrupt enabled, counter enabled
            Timer {
                alarm_taken: false,
                _device: PhantomData,
            }
        }
    }

    impl<D> Timer<D> {
        /// Microseconds since the timer was created
        pub fn get_counter(&self) -> Instant {
            loop {
                let ms = MS_TICKS.load(Ordering::Relaxed);
                let cvr = read_reg(SYST_CVR);
                if ms == MS_TICKS.load(Ordering::Relaxed) {
                    let us =
                        ms as u64 * 1_000 + ((SYST_RELOAD - cvr) / (SYSCLK_HZ / 1_000_000)) as u64;
                    return Instant::from_ticks(us);
                }
            }
        }

        pub fn alarm_0(&mut self) -> Option<Alarm0<D>> {
            if self.alarm_taken {
                return None;
            }
            self.alarm_taken = true;
            Some(Alarm0 {
                interrupt_enabled: false,
                _device: PhantomData,
            })
        }

        fn wait_until(&self, target: u64) {
            loop {
                let now = self.get_counter().ticks();
                if now >= target {
                    break;
                }
                if target - now > 1_000 {
                    cortex_m::asm::wfi(); // woken by the next SysTick, so QEMU doesn't spin the host CPU
                }
            }
        }
    }

    impl<D> embedded_hal::delay::DelayNs for Timer<D> {
        fn delay_ns(&mut self, ns: u32) {
            self.wait_until(self.get_counter().ticks() + ns.div_ceil(1_000) as u64);
        }

        fn delay_us(&mut self, us: u32) {
            self.wait_until(self.get_counter().ticks() + us as u64);
        }

        fn delay_ms(&mut self, ms: u32) {
            self.wait_until(self.get_counter().ticks() + ms as u64 * 1_000);
        }
    }

    pub trait Alarm {
        fn clear_interrupt(&mut self);
        fn enable_interrupt(&mut self);
        fn schedule(&mut self, countdown: MicrosDurationU32) -> Result<(), Infallible>;
    }

    /// One-shot alarm on CMSDK TIMER0, which counts down at the system clock
    pub struct Alarm0<D> {
        interrupt_enabled: bool,
        _device: PhantomData<D>,
    }

    impl<D> Alarm0<D> {
        fn ctrl(&self) -> u32 {
            if self.interrupt_enabled {
                TIMER_CTRL_INTEN
            } else {
                0
            }
        }
    }

    impl<D> Alarm for Alarm0<D> {
        /// Clears the interrupt and stops the timer, since the alarm has fired
        fn clear_interrupt(&mut self) {
            write_reg(TIMER0_CTRL, self.ctrl());
            write_reg(TIMER0_INTCLEAR, 1);
        }

        fn enable_interrupt(&mut self) {
            self.interrupt_enabled = true;
            write_reg(TIMER0_CTRL, read_reg(TIMER0_CTRL) | TIMER_CTRL_INTEN);
        }

        fn schedule(&mut self, countdown: MicrosDurationU32) -> Result<(), Infallible> {
            let ticks = countdown.ticks() * (SYSCLK_HZ / 1_000_000);
            write_reg(TIMER0_CTRL, 0);
            write_reg(TIMER0_RELOAD, ticks);
            write_reg(TIMER0_VALUE, ticks);
            write_reg(TIMER0_CTRL, self.ctrl() | TIMER_CTRL_EN);
            Ok(())
        }
    }
}

pub use timer::Timer;

pub mod watchdog {
    use super::{SYSCLK_HZ, pac, uart, write_reg};

    // CMSDK APB watchdog (secure); its interrupt is routed to NMI
    const WDOG_LOAD: u32 = 0x5008_1000;
    const WDOG_CTRL: u32 = 0x5008_1008;
    const WDOG_LOCK: u32 = 0x5008_1C00;
    const WDOG_UNLOCK_KEY: u32 = 0x1ACC_E551;
    const WDOG_CTRL_INTEN: u32 = 1 << 0;

    pub struct Watchdog {
        load: u32,
    }

    impl Watchdog {
        pub fn new(_watchdog: pac::WATCHDOG) -> Self {
            Watchdog { load: 0 }
        }

        pub fn start<T: Into<fugit::MicrosDurationU32>>(&mut self, period: T) {
            self.load = period.into().ticks() * (SYSCLK_HZ / 1_000_000);
            write_reg(WDOG_LOCK, WDOG_UNLOCK_KEY);
            write_reg(WDOG_LOAD, self.load);
            write_reg(WDOG_CTRL, WDOG_CTRL_INTEN);
            write_reg(WDOG_LOCK, 0);
        }

        pub fn feed(&self) {
            write_reg(WDOG_LOCK, WDOG_UNLOCK_KEY);
            write_reg(WDOG_LOAD, self.load); // writing LOAD reloads the counter
            write_reg(WDOG_LOCK, 0);
        }
    }

    // The watchdog's first timeout raises its interrupt, which the SSE-200 routes to NMI.
    // Reset there so the timeout matches the RP2350 (one period, not two).
    #[cortex_m_rt::exception]
    unsafe fn NonMaskableInt() {
        uart::write_bytes(b"\r\n*** watchdog timeout, rebooting ***\r\n");
        cortex_m::peripheral::SCB::sys_reset();
    }
}

pub use watchdog::Watchdog;

pub mod uart {
    use super::{SYSCLK_HZ, pac, read_reg, write_reg};
    use core::convert::Infallible;

    // CMSDK APB UART0 (secure alias), connected to QEMU's first -serial
    const UART0_DATA: u32 = 0x5020_0000;
    const UART0_STATE: u32 = 0x5020_0004;
    const UART0_CTRL: u32 = 0x5020_0008;
    const UART0_BAUDDIV: u32 = 0x5020_0010;
    const UART_STATE_TXFULL: u32 = 1 << 0;
    const UART_CTRL_TXEN: u32 = 1 << 0;

    pub(super) fn write_bytes(data: &[u8]) {
        for &b in data {
            while read_reg(UART0_STATE) & UART_STATE_TXFULL != 0 {}
            write_reg(UART0_DATA, b as u32);
        }
    }

    /// Stands in for `usbd_serial::SerialPort`, since QEMU has no USB host to enumerate a device
    pub struct Uart;

    impl Uart {
        pub fn new(_uart: pac::UART0, _clocks: &super::clocks::ClocksManager) -> Self {
            write_reg(UART0_BAUDDIV, SYSCLK_HZ / 115_200); // QEMU ignores the rate, but drops output if BAUDDIV < 16
            write_reg(UART0_CTRL, UART_CTRL_TXEN);
            Uart
        }

        pub fn write(&mut self, data: &[u8]) -> Result<usize, Infallible> {
            write_bytes(data);
            Ok(data.len())
        }
    }
}
