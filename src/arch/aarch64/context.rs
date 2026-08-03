use core::arch::global_asm;

/// List of register on AArch64 to save when switching context
/// offsets:  x19..x28 = 0..72, fp = 80, lr = 88, sp = 96
#[repr(C)]
#[derive(Default)]
pub struct Context {
    pub x19: u64,
    pub x20: u64,
    pub x21: u64,
    pub x22: u64,
    pub x23: u64,
    pub x24: u64,
    pub x25: u64,
    pub x26: u64,
    pub x27: u64,
    pub x28: u64,
    pub fp: u64,
    pub lr: u64,
    pub sp: u64,
}

impl Context {
    pub fn new(entry: extern "C" fn(), stack_top: u64) -> Self {
        extern "C" {
            fn task_trampoline();
        }
        Self {
            x19: entry as u64,
            lr: task_trampoline as u64,
            sp: stack_top,
            ..Default::default()
        }
    }
}

extern "C" {
    /// Save current callee-saved state into *prev, load from *next, ret.
    pub fn cpu_switch_to(prev: *mut Context, next: *const Context);
}

global_asm!(
    r#"
.section .text
.global cpu_switch_to
cpu_switch_to:
    stp x19, x20, [x0, #0]
    stp x21, x22, [x0, #16]
    stp x23, x24, [x0, #32]
    stp x25, x26, [x0, #48]
    stp x27, x28, [x0, #64]
    stp x29, x30, [x0, #80]
    mov x9, sp
    str x9,       [x0, #96]

    ldp x19, x20, [x1, #0]
    ldp x21, x22, [x1, #16]
    ldp x23, x24, [x1, #32]
    ldp x25, x26, [x1, #48]
    ldp x27, x28, [x1, #64]
    ldp x29, x30, [x1, #80]
    ldr x9,       [x1, #96]
    mov sp, x9
    ret

.global task_trampoline
task_trampoline:
    // New task's first ever instructions. Entry fn was stashed in x19.
    blr x19
    // Entry returned: tell the scheduler this task is done. Never returns.
    bl  task_exit
"#
);
