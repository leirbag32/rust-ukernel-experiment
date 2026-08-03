//! device-os: a single-address-space microkernel for the device side of
//! AI/compute accelerators. Host runs unchanged Linux; this OS owns the
//! management cores, the persistent object store, and the command queue.
//!
//! Right now: boots on QEMU virt and prints hello. Everything else is
//! stubs for you to fill in (see the roadmap in README.md).

#![no_std]
#![no_main]

use core::arch::global_asm;
use core::panic::PanicInfo;
use crate::drivers::virtio;

mod boards;
mod arch;
mod drivers;
mod mm;
mod sched;
mod store;

global_asm!(include_str!("arch/aarch64/boot.s"));

#[no_mangle]
pub extern "C" fn kernel_main() -> ! {
    // Board specific initialization (UART is init internally for Debug purpose)
    boards::qemu_virt::init();

    // Cannot print before the board is initialized
    println!("======== OS INIT START ========");

    // Unsafe as they access directly to ASM code that cannot be verified by Rust
    unsafe {
        arch::aarch64::init();
        println!("arch: architecture code specific (NEON, FP, etc.) initialized");
        mm::init();
        println!("arch: MMU initialized");
    }

    println!("arch: board initialized");

    // Drivers init
    match drivers::virtio::init() {
        Ok(()) => println!("drivers: virtio initialized"),
        Err(m) => { println!("drivers: virtio FAILED: {}", m); loop { arch::wait_for_event() } }
    }
    match store::init() {
        Ok(()) => println!("persistent-object-storage initialized"),
        Err(m) => println!("persistent-object-storage FAILED: {}", m),
    }

    // Core feature init such as the scheduler
    sched::init();
    println!("core: Scheduler initialized");
    println!("======== OS INIT DONE ========");

    store::run_pos_test();

    panic!("kernel: end of kernel_main: nothing to do!");
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("\n*** PANIC ***\n{}", info);
    loop {
        arch::wait_for_event();
    }
}
