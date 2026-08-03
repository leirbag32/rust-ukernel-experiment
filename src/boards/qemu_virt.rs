use core::fmt::{self, Write};
 
use core::sync::atomic::{AtomicBool, Ordering};
use crate::drivers::pl011::{Pl011, Config};
use crate::arch::aarch64::spinlock::{SpinMutex};

// ---------------------------------------------------------------------------
// Memory map
// ---------------------------------------------------------------------------

pub const UART0_BASE: usize = 0x0900_0000;
pub const UART1_BASE: usize = 0x0904_0000;
pub const UART_SIZE: usize = 0x1000; // 4KB

/// GIC
pub const GICD_BASE: usize = 0x0800_0000;
/// GICv2
pub const GICC_BASE: usize = 0x0801_0000;
/// GICv3
pub const GICR_BASE: usize = 0x080A_0000;

pub const RTC_BASE: usize = 0x0901_0000;
pub const VIRTIO_MMIO_BASE: usize = 0x0A00_0000;

/// RAM starts at 1GB
pub const RAM_GIB: usize = 64;
pub const RAM_BASE: u64 = 0x4000_0000;
pub const RAM_END: u64 = RAM_BASE + (RAM_GIB as u64) * (1 << 30);

// ---------------------------------------------------------------------------
// Console setup + hooks for Rust println
// ---------------------------------------------------------------------------

pub const UART_CLK_HZ: u32 = 24_000_000;
 
/// Default console settings: 115200 8N1, FIFOs on.
pub const CONSOLE_CONFIG: Config = Config::new(UART_CLK_HZ).baud(115_200);

/// Main Console is on UART0 implemented by PL011 on this platform
pub static CONSOLE: SpinMutex<Option<Pl011>> = SpinMutex::new(None);
static UART0_TAKEN: AtomicBool = AtomicBool::new(false);

pub fn take_uart0() -> Option<Pl011> {
    if UART0_TAKEN.swap(true, Ordering::AcqRel) {
        None
    } else {
        Some(unsafe { Pl011::new(UART0_BASE) })
    }
}

pub fn init() {
    // Console initialization
    if let Some(mut uart) = take_uart0() {
        uart.init(&CONSOLE_CONFIG);
        *CONSOLE.lock() = Some(uart);
    }
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments<'_>) {
    if let Some(uart) = CONSOLE.lock().as_mut() {
        let _ = uart.write_fmt(args);
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::boards::qemu_virt::_print(format_args!($($arg)*))
    };
}
 
#[macro_export]
macro_rules! println {
    () => { $crate::print!("\n") };
    ($($arg:tt)*) => {
        $crate::boards::qemu_virt::_print(format_args!("{}\n", format_args!($($arg)*)))
    };
}
