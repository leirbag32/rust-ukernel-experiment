.section ".text.boot"
.global _start

_start:
    // Park all cores except core 0.
    mrs     x0, mpidr_el1
    and     x0, x0, #0xFF
    cbz     x0, .L_core0
.L_park:
    wfe
    b       .L_park

.L_core0:
    // Stack pointer <- __stack_top (grows down into the reserved region).
    adrp    x0, __stack_top
    add     x0, x0, :lo12:__stack_top
    mov     sp, x0

    // Zero .bss so Rust statics start from a known state.
    adrp    x0, __bss_start
    add     x0, x0, :lo12:__bss_start
    adrp    x1, __bss_end
    add     x1, x1, :lo12:__bss_end
.L_bss_loop:
    cmp     x0, x1
    b.ge    .L_bss_done
    str     xzr, [x0], #8
    b       .L_bss_loop
.L_bss_done:

    bl      kernel_main

.L_halt:
    wfe
    b       .L_halt
