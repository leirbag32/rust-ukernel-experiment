//! ============================================================================
//!   VA                  region                     size      mapping
//!   ─────────────────────────────────────────────────────────────────────
//!   0x7F_FFFF_FFFF ──┐
//!                    │  unmapped (guard)           255 GiB   faults
//!   0x40_4000_0000 ──┼─────────────────────────────────────────────────────
//!                    │  PERSISTENT OBJECTS                   L2/L3, 4 KiB
//!                    │  slot 256+, POS_VA_BASE               demand-paged
//!   0x40_0000_0000 ──┼─────────────────────────────────────────────────────
//!                    │  gap (RAM growth)           191 GiB   unmapped
//!   0x10_4000_0000 ──┼─────────────────────────────────────────────────────
//!                    │  MAIN RAM (identity)         64 GiB   block descs
//!                    │  slots 1..64, VA == PA                Normal WB, IS
//!       __kernel_end │    ├─ frame allocator arena
//!        0x4008_0000 │    └─ kernel image, stacks, virtio rings
//!   0x4000_0000 ─────┼─────────────────────────────────────────────────────
//!                    │  DEVICE MMIO (identity)       1 GiB   1 block desc
//!                    │  slot 0: GIC, UART, virtio            Device-nGnRnE
//!   0x0000_0000 ─────┘
//! ============================================================================

use crate::println;
use core::arch::asm;
use core::ptr::{addr_of, addr_of_mut};

pub const PAGE: u64 = 4096;
pub const RAM_BASE: u64 = 0x4000_0000;
pub const RAM_GIB: u64 = 64;
pub const RAM_END: u64 = RAM_BASE + RAM_GIB * (1 << 30);

pub const MAP_RO: u64 = 0;
pub const MAP_RW: u64 = 1;

const ADDR_MASK: u64 = 0x0000_FFFF_FFFF_F000;

const DESC_BLOCK: u64 = 0b01;
const DESC_TABLE: u64 = 0b11;
const DESC_PAGE: u64 = 0b11;

const AF: u64 = 1 << 10;
const SH_INNER: u64 = 0b11 << 8;
const ATTR_NORMAL: u64 = 0 << 2;
const ATTR_DEVICE: u64 = 1 << 2;
const AP_RW_EL1: u64 = 0b00 << 6;
const AP_RO_EL1: u64 = 0b10 << 6;

/// T0SZ=25 (39-bit VA), WB-WA walks, inner shareable, 4 KiB granule,
/// EPD1=1 (TTBR1 unused), IPS=0b010 (40-bit PA).
const TCR: u64 = 25 | (0b01 << 8) | (0b01 << 10) | (0b11 << 12) | (1 << 23) | (0b010 << 32);
const MAIR: u64 = 0x0000_0000_0000_00FF;

#[repr(C, align(4096))]
pub struct Table {
    entries: [u64; 512],
}

static mut L1_TABLE: Table = Table { entries: [0; 512] };

static mut NEXT_FRAME: u64 = 0;
static mut FRAME_START: u64 = 0;

/// Must run before init(): the L1 setup itself allocates no frames, but
/// map_page() does, and callers expect the arena to exist.
pub fn frames_init() {
    extern "C" {
        static __kernel_end: u8;
    }
    unsafe {
        let start = (addr_of!(__kernel_end) as u64 + PAGE - 1) & !(PAGE - 1);
        FRAME_START = start;
        NEXT_FRAME = start;
    }
}

/// Zeroed 4 KiB frame from identity-mapped RAM, so the returned physical
/// address is directly usable as a pointer.
pub fn alloc_frame() -> Option<u64> {
    unsafe {
        let f = NEXT_FRAME;
        if f == 0 || f + PAGE > RAM_END {
            return None;
        }
        NEXT_FRAME = f + PAGE;
        core::ptr::write_bytes(f as *mut u8, 0, PAGE as usize);
        Some(f)
    }
}

pub fn frames_used() -> u64 {
    unsafe { (NEXT_FRAME - FRAME_START) / PAGE }
}

pub unsafe fn init() {
    extern "C" {
        static __kernel_start: u8;
        static __kernel_end: u8;
    }

    L1_TABLE.entries[0] = AF | ATTR_DEVICE | DESC_BLOCK;

    let normal = AF | SH_INNER | ATTR_NORMAL | DESC_BLOCK;
    for i in 0..RAM_GIB {
        let pa = RAM_BASE + i * (1 << 30);
        L1_TABLE.entries[(pa >> 30) as usize] = pa | normal;
    }

    let ttbr = addr_of!(L1_TABLE) as u64;
    let t0sz = TCR & 0x3F;

    println!("mm: L1 table      {:#018x} ({} entries)", ttbr, 512);
    println!("mm: granule       4 KiB, {}-bit VA (T0SZ={})", 64 - t0sz, t0sz);
    println!("mm: TCR={:#x} MAIR={:#x}", TCR, MAIR);
    println!("mm: device        {:#018x}..{:#018x}  1 GiB  block, slot 0",
        0u64, 1u64 << 30);
    println!("mm: ram           {:#018x}..{:#018x}  {} GiB block, slots 1..{}",
        RAM_BASE, RAM_END, RAM_GIB, RAM_GIB);
    println!("mm:   kernel      {:#018x}..{:#018x}  {} KiB",
        addr_of!(__kernel_start) as u64,
        addr_of!(__kernel_end) as u64,
        (addr_of!(__kernel_end) as u64 - addr_of!(__kernel_start) as u64) / 1024);
    println!("mm:   frames      {:#018x}..{:#018x}  {} MiB",
        FRAME_START, RAM_END, (RAM_END - FRAME_START) / (1024 * 1024));
    println!("mm: objects       {:#018x}..           demand-paged, L2/L3",
        crate::store::POS_VA_BASE);

    asm!("msr mair_el1, {}", in(reg) MAIR);
    asm!("msr tcr_el1, {}", in(reg) TCR);
    asm!("msr ttbr0_el1, {}", in(reg) ttbr);
    asm!("isb; tlbi vmalle1; dsb sy; isb", options(nostack, nomem));

    let mut sctlr: u64;
    asm!("mrs {}, sctlr_el1", out(reg) sctlr);
    sctlr |= 0x1 | 0x4 | 0x1000;
    asm!("msr sctlr_el1, {}", in(reg) sctlr);
    asm!("isb", options(nostack, nomem));

    println!("mm: mmu enabled   SCTLR={:#x} (M|C|I)", sctlr);
}

unsafe fn next_level(entry: &mut u64) -> &'static mut Table {
    if *entry & 0b11 == 0 {
        let pa = alloc_frame().expect("mm: out of frames for page table");
        *entry = (pa & ADDR_MASK) | DESC_TABLE;
        asm!("dsb ishst", options(nostack));
    }
    debug_assert!(*entry & 0b11 == DESC_TABLE, "mm: block descriptor where table expected");
    &mut *((*entry & ADDR_MASK) as *mut Table)
}

pub fn map_page(va: u64, pa: u64, flags: u64) {
    unsafe {
        let l1 = &mut *addr_of_mut!(L1_TABLE);
        let l2 = next_level(&mut l1.entries[((va >> 30) & 0x1FF) as usize]);
        let l3 = next_level(&mut l2.entries[((va >> 21) & 0x1FF) as usize]);

        let ap = if flags & MAP_RW != 0 { AP_RW_EL1 } else { AP_RO_EL1 };
        l3.entries[((va >> 12) & 0x1FF) as usize] =
            (pa & ADDR_MASK) | AF | SH_INNER | ap | ATTR_NORMAL | DESC_PAGE;

        asm!("dsb ishst; isb", options(nostack));
    }
}

unsafe fn walk(va: u64) -> Option<&'static mut Table> {
    let l1 = &*addr_of!(L1_TABLE);
    let e1 = l1.entries[((va >> 30) & 0x1FF) as usize];
    if e1 & 0b11 != DESC_TABLE {
        return None;
    }
    let l2 = &*((e1 & ADDR_MASK) as *const Table);
    let e2 = l2.entries[((va >> 21) & 0x1FF) as usize];
    if e2 & 0b11 != DESC_TABLE {
        return None;
    }
    Some(&mut *((e2 & ADDR_MASK) as *mut Table))
}

pub fn unmap_page(va: u64) {
    unsafe {
        let Some(l3) = walk(va) else { return };
        l3.entries[((va >> 12) & 0x1FF) as usize] = 0;
        asm!("dsb ishst", options(nostack));
        asm!("tlbi vaae1is, {}", in(reg) va >> 12, options(nostack));
        asm!("dsb ish; isb", options(nostack));
    }
}

/// True for anything covered by an identity block, or by a valid L3 page.
pub fn is_mapped(va: u64) -> bool {
    unsafe {
        let l1 = &*addr_of!(L1_TABLE);
        if l1.entries[((va >> 30) & 0x1FF) as usize] & 0b11 == DESC_BLOCK {
            return true;
        }
        match walk(va) {
            Some(l3) => l3.entries[((va >> 12) & 0x1FF) as usize] & 0b11 == DESC_PAGE,
            None => false,
        }
    }
}
