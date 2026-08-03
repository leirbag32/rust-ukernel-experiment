use core::ptr::{addr_of, addr_of_mut, read_volatile, write_volatile};
use core::sync::atomic::{fence, Ordering};

pub const SECTOR: usize = 512;
const QSIZE: usize = 8;

const MMIO_BASE: usize = 0x0a00_0000;
const MMIO_STRIDE: usize = 0x200;
const MMIO_SLOTS: usize = 32;

// -- virtio-mmio register offsets (spec §4.2.2) --
const MAGIC: usize = 0x000;         // 0x74726976 "virt"
const VERSION: usize = 0x004;       // must be 2
const DEVICE_ID: usize = 0x008;     // 2 = block
const DEV_FEAT: usize = 0x010;
const DEV_FEAT_SEL: usize = 0x014;
const DRV_FEAT: usize = 0x020;
const DRV_FEAT_SEL: usize = 0x024;
const QUEUE_SEL: usize = 0x030;
const QUEUE_NUM_MAX: usize = 0x034;
const QUEUE_NUM: usize = 0x038;
const QUEUE_READY: usize = 0x044;
const QUEUE_NOTIFY: usize = 0x050;
const INT_STATUS: usize = 0x060;
const INT_ACK: usize = 0x064;
const STATUS: usize = 0x070;
const QUEUE_DESC_LO: usize = 0x080;
const QUEUE_DESC_HI: usize = 0x084;
const QUEUE_DRV_LO: usize = 0x090;  // avail ring
const QUEUE_DRV_HI: usize = 0x094;
const QUEUE_DEV_LO: usize = 0x0a0;  // used ring
const QUEUE_DEV_HI: usize = 0x0a4;
const CONFIG: usize = 0x100;        // blk: capacity (sectors) u64 le

// status bits (§2.1)
const ST_ACK: u32 = 1;
const ST_DRIVER: u32 = 2;
const ST_DRIVER_OK: u32 = 4;
const ST_FEATURES_OK: u32 = 8;
const ST_FAILED: u32 = 128;

// descriptor flags (§2.6.5)
const DF_NEXT: u16 = 1;
const DF_WRITE: u16 = 2; // DEVICE writes this buffer

// blk request types (§5.2.6)
const BLK_IN: u32 = 0;  // read
const BLK_OUT: u32 = 1; // write

#[repr(C, align(16))]
#[derive(Clone, Copy)]
struct Desc { addr: u64, len: u32, flags: u16, next: u16 }

#[repr(C, align(2))]
struct Avail { flags: u16, idx: u16, ring: [u16; QSIZE] }

#[repr(C)]
#[derive(Clone, Copy)]
struct UsedElem { id: u32, len: u32 }

#[repr(C, align(4))]
struct Used { flags: u16, idx: u16, ring: [UsedElem; QSIZE] }

#[repr(C, align(4096))]
struct VirtQueue { desc: [Desc; QSIZE], avail: Avail, used: Used }

#[repr(C)]
struct BlkReqHdr { ty: u32, _reserved: u32, sector: u64 }

static mut VQ: VirtQueue = unsafe { core::mem::zeroed() };
static mut REQ_HDR: BlkReqHdr = BlkReqHdr { ty: 0, _reserved: 0, sector: 0 };
static mut REQ_STATUS: u8 = 0xff;

struct Blk { base: usize, last_used: u16, capacity: u64, ready: bool }
static mut BLK: Blk = Blk { base: 0, last_used: 0, capacity: 0, ready: false };

fn blk() -> &'static mut Blk { unsafe { &mut *addr_of_mut!(BLK) } }

fn r32(base: usize, off: usize) -> u32 {
    unsafe { read_volatile((base + off) as *const u32) }
}
fn w32(base: usize, off: usize, v: u32) {
    unsafe { write_volatile((base + off) as *mut u32, v) }
}

/// Probe the 32 virtio-mmio slots for a modern block device and bring
/// it up. Call once, before store::init().
pub fn init() -> Result<(), &'static str> {
    let mut base = 0usize;
    for slot in 0..MMIO_SLOTS {
        let b = MMIO_BASE + slot * MMIO_STRIDE;
        if r32(b, MAGIC) == 0x7472_6976 && r32(b, DEVICE_ID) == 2 {
            if r32(b, VERSION) != 2 {
                return Err("blk device is legacy; add -global virtio-mmio.force-legacy=false");
            }
            base = b;
            break;
        }
    }
    if base == 0 {
        return Err("no virtio-blk device found (missing -drive/-device flags?)");
    }

    // §3.1.1 driver init sequence
    w32(base, STATUS, 0); // reset
    w32(base, STATUS, ST_ACK);
    w32(base, STATUS, ST_ACK | ST_DRIVER);

    // Features: accept only VIRTIO_F_VERSION_1 (bit 32) — mandatory for
    // modern devices; we want nothing else.
    w32(base, DEV_FEAT_SEL, 1);
    if r32(base, DEV_FEAT) & 1 == 0 {
        w32(base, STATUS, ST_FAILED);
        return Err("device lacks VERSION_1 feature");
    }
    w32(base, DRV_FEAT_SEL, 0);
    w32(base, DRV_FEAT, 0);
    w32(base, DRV_FEAT_SEL, 1);
    w32(base, DRV_FEAT, 1); // bit 32
    w32(base, STATUS, ST_ACK | ST_DRIVER | ST_FEATURES_OK);
    if r32(base, STATUS) & ST_FEATURES_OK == 0 {
        w32(base, STATUS, ST_FAILED);
        return Err("device rejected features");
    }

    // Queue 0 setup: rings live in our identity-mapped .bss, VA == PA.
    w32(base, QUEUE_SEL, 0);
    if r32(base, QUEUE_NUM_MAX) < QSIZE as u32 {
        return Err("queue too small");
    }
    w32(base, QUEUE_NUM, QSIZE as u32);
    let desc = unsafe { addr_of!(VQ.desc) as u64 };
    let avail = unsafe { addr_of!(VQ.avail) as u64 };
    let used = unsafe { addr_of!(VQ.used) as u64 };
    w32(base, QUEUE_DESC_LO, desc as u32);
    w32(base, QUEUE_DESC_HI, (desc >> 32) as u32);
    w32(base, QUEUE_DRV_LO, avail as u32);
    w32(base, QUEUE_DRV_HI, (avail >> 32) as u32);
    w32(base, QUEUE_DEV_LO, used as u32);
    w32(base, QUEUE_DEV_HI, (used >> 32) as u32);
    w32(base, QUEUE_READY, 1);

    w32(base, STATUS, ST_ACK | ST_DRIVER | ST_FEATURES_OK | ST_DRIVER_OK);

    let cap_lo = r32(base, CONFIG) as u64;
    let cap_hi = r32(base, CONFIG + 4) as u64;

    let b = blk();
    b.base = base;
    b.capacity = (cap_hi << 32) | cap_lo;
    b.last_used = 0;
    b.ready = true;
    Ok(())
}

pub fn capacity_sectors() -> u64 { blk().capacity }

/// One request: 3-descriptor chain (header -> data -> status), notify,
/// poll the used ring. Cooperative-friendly: yields while waiting.
fn do_io(ty: u32, lba: u64, buf: *mut u8, len: usize, device_writes: bool) -> Result<(), ()> {
    let b = blk();
    if !b.ready || len % SECTOR != 0 { return Err(()); }

    unsafe {
        write_volatile(addr_of_mut!(REQ_HDR.ty), ty);
        write_volatile(addr_of_mut!(REQ_HDR.sector), lba);
        write_volatile(addr_of_mut!(REQ_STATUS), 0xff);

        let d = addr_of_mut!(VQ.desc) as *mut Desc;
        write_volatile(d.add(0), Desc {
            addr: addr_of!(REQ_HDR) as u64, len: 16, flags: DF_NEXT, next: 1,
        });
        write_volatile(d.add(1), Desc {
            addr: buf as u64, len: len as u32,
            flags: DF_NEXT | if device_writes { DF_WRITE } else { 0 }, next: 2,
        });
        write_volatile(d.add(2), Desc {
            addr: addr_of!(REQ_STATUS) as u64, len: 1, flags: DF_WRITE, next: 0,
        });

        let av_idx = read_volatile(addr_of!(VQ.avail.idx));
        let slot = (av_idx as usize) % QSIZE;
        write_volatile((addr_of_mut!(VQ.avail.ring) as *mut u16).add(slot), 0);
        fence(Ordering::SeqCst); // ring entry visible before idx bump (§2.6.13)
        write_volatile(addr_of_mut!(VQ.avail.idx), av_idx.wrapping_add(1));
        fence(Ordering::SeqCst);

        w32(b.base, QUEUE_NOTIFY, 0);

        // Poll for completion. Single request in flight -> next used.idx.
        let target = b.last_used.wrapping_add(1);
        while read_volatile(addr_of!(VQ.used.idx)) != target {
            core::hint::spin_loop();
            // If the scheduler is up, be a good citizen:
            crate::sched::yield_now();
        }
        fence(Ordering::SeqCst);
        b.last_used = target;

        // ack interrupt line even in polling mode (keeps status clean)
        let is = r32(b.base, INT_STATUS);
        if is != 0 { w32(b.base, INT_ACK, is); }

        if read_volatile(addr_of!(REQ_STATUS)) != 0 { return Err(()); }
    }
    Ok(())
}

/// Read `buf.len()` bytes (multiple of 512) starting at sector `lba`.
pub fn read(lba: u64, buf: &mut [u8]) -> Result<(), ()> {
    do_io(BLK_IN, lba, buf.as_mut_ptr(), buf.len(), true)
}

/// Write `buf.len()` bytes (multiple of 512) starting at sector `lba`.
pub fn write(lba: u64, buf: &[u8]) -> Result<(), ()> {
    do_io(BLK_OUT, lba, buf.as_ptr() as *mut u8, buf.len(), false)
}
