.global vector_table
.balign 2048

.macro ventry label
.balign 128
    b \label
.endm

vector_table:
    // --- Current EL with SP_EL0 ---
    ventry el1_sync_sp0       // Synchronous
    ventry el1_irq_sp0        // IRQ / Hardware Interrupt
    ventry el1_fiq_sp0        // Fast Interrupt
    ventry el1_serror_sp0     // System Error

    // --- Current EL with SP_ELx (Kernel Stack) ---
    ventry el1_sync_current   // Synchronous (e.g., page faults, syscalls)
    ventry el1_irq_current    // IRQ / Hardware Interrupt 
    ventry el1_fiq_current    // Fast Interrupt
    ventry el1_serror_current // System Error

    // --- Lower EL using AArch64 ---
    ventry el1_sync_lower     // From User mode (AArch64)
    ventry el1_irq_lower      
    ventry el1_fiq_lower      
    ventry el1_serror_lower   

    // --- Lower EL using AArch32 ---
    ventry el1_sync_lower32   
    ventry el1_irq_lower32    
    ventry el1_fiq_lower32    
    ventry el1_serror_lower32 

// SP_EL0 Handlers
el1_sync_sp0:
el1_irq_sp0:
el1_fiq_sp0:
el1_serror_sp0:
// Lower EL Handlers (AArch64 & AArch32)
el1_sync_lower:
el1_irq_lower:
el1_fiq_lower:
el1_serror_lower:
el1_sync_lower32:
el1_irq_lower32:
el1_fiq_lower32:
el1_serror_lower32:
// Unused / Unimplemented catch-all
    b .

el1_irq_current:
    // Save general purpose registers scratch state if needed
    sub sp, sp, #288
    // ... save registers to stack ...
    bl rust_irq_handler
    // ... restore registers from stack ...
    add sp, sp, #288
    eret

el1_sync_current:
    bl rust_sync_handler
    eret

el1_fiq_current:
el1_serror_current:
    b .
