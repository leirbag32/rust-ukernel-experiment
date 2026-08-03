pub mod exceptions;
pub mod spinlock;
pub mod context;

use core::arch::asm;

pub fn current_el() -> u64 {
    let el: u64;
    unsafe { asm!("mrs {}, CurrentEL", out(reg) el) };
    (el >> 2) & 0b11
}

pub fn wait_for_event() {
    unsafe { asm!("wfe") };
}

pub unsafe fn init() {
    // Exception / Vector Table initialization
    crate::arch::aarch64::exceptions::init();

    // Enable FP/SIMD at EL1: CPACR_EL1.FPEN = 0b11 (bits [21:20])
    unsafe {
        let mut cpacr: u64;
        asm!("mrs {}, cpacr_el1", out(reg) cpacr);
        cpacr |= 0b11 << 20;
        asm!("msr cpacr_el1, {}", in(reg) cpacr, options(nostack));
        asm!("isb", options(nostack));
    }
}
