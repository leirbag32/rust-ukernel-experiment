/* QEMU -M virt (AArch64): RAM starts at 0x4000_0000, the -kernel option
 * loads / enters at 0x4008_0000, so we link there. */

ENTRY(_start)

SECTIONS
{
    . = 0x40080000;
    __kernel_start = .;

    .text : {
	/* Entry point, ALWAYS FIRST */
        KEEP(*(.text.boot))

	/* Vector Table must be aligned to 2048 bytes */
	. = ALIGN(2048);
	KEEP(*(.text.vector_table*))

	/* Anything else */
        *(.text .text.*)
    }

    .rodata : ALIGN(4096) {
        *(.rodata .rodata.*)
    }

    .data : ALIGN(4096) {
        *(.data .data.*)
    }

    .bss (NOLOAD) : ALIGN(4096) {
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        . = ALIGN(16);
        __bss_end = .;
    }

    /* 64 KiB boot stack, reserved BEFORE __kernel_end. The stack grows
     * DOWN from __stack_top into this region. */
    .stack (NOLOAD) : ALIGN(16) {
        __stack_bottom = .;
        . += 0x10000;
        __stack_top = .;
    }

    . = ALIGN(4096);
    __kernel_end = .;

    /* Unwind tables are orphan sections: without this DISCARD the linker
     * may place .eh_frame_hdr at the image start, ahead of .text.boot,
     * and QEMU executes it as garbage. Keep this list. */
    /DISCARD/ : {
        *(.comment)
        *(.note .note.*)
        *(.eh_frame .eh_frame_hdr)
    }
}
