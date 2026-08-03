use core::arch::global_asm;
use core::arch::asm;
use crate::println;
use crate::store;

// Include the assembly vector table
global_asm!(include_str!("exceptions.s"));

extern "C" {
    // Symbol defined in exceptions.s
    static vector_table: u8;
}

#[no_mangle]
pub extern "C" fn rust_irq_handler() {
}

#[no_mangle]
pub extern "C" fn rust_sync_handler() {
    let esr: u64;
    let far: u64;
    let elr: u64;

    unsafe {
        core::arch::asm!("mrs {}, esr_el1", out(reg) esr);
        core::arch::asm!("mrs {}, far_el1", out(reg) far);
        core::arch::asm!("mrs {}, elr_el1", out(reg) elr);
    }


    let ec = (esr >> 26) & 0x3F;

    match ec {
        0x25 => {
            let dfsc = esr & 0x3F;
            let is_translation_fault = matches!(dfsc, 0b000100..=0b000111);
            let is_write = (esr >> 6) & 1 == 1;
            println!("Page Fault Detected: is translation fault={}", is_translation_fault);
            if is_translation_fault {
                match store::handle_fault(far, is_write) {
                    Ok(()) => return,          // eret -> instruction re-executes
                    Err(e) => panic!("unhandled fault at {:#x}: {:?}", far, e),
                }
            }
        }
        0x24 => {
            println!("EC=0x24 | Data Abort! FAR: {:#x}, ESR: {:#x}, PC: {:#x}", far, esr, elr);
        }
        0x20 => {
            println!("EC=0x20 | Instruction Abort! FAR: {:#x}, ESR: {:#x}, PC: {:#x}", far, esr, elr);
        }
        _ => {
            println!("EC={:#x} | Unknown Abort! FAR: {:#x}, ESR: {:#x}, PC: {:#x}", ec, far, esr, elr);
        }
    }

    // Handle exception or trigger panic/halt
    loop {}
}

pub unsafe fn init() {
    let table_addr = &vector_table as *const u8 as u64;
    
    // 1. Install vector table base address into VBAR_EL1
    asm!("msr vbar_el1, {}", in(reg) table_addr, options(nomem, nostack));

    // 2. Clear the IRQ mask bit in DAIF to globally enable interrupts
    // Clear 'I' bit (bit 7) in DAIF register
    asm!("msr daifclr, #2", options(nomem, nostack)); // #2 clears IRQ mask flag
}
