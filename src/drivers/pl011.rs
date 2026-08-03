#![allow(dead_code)]

use core::fmt;

#[repr(C)]
pub struct Pl011Regs {
    /// 0x00 — Data register.
    pub dr: u32,
    /// 0x04 — Receive status / error clear.
    pub rsr_ecr: u32,
    _reserved0: [u32; 4],
    /// 0x18 — Flag register.
    pub fr: u32,
    _reserved1: u32,
    /// 0x20 — IrDA low-power counter.
    pub ilpr: u32,
    /// 0x24 — Integer baud rate divisor.
    pub ibrd: u32,
    /// 0x28 — Fractional baud rate divisor.
    pub fbrd: u32,
    /// 0x2C — Line control, high byte.
    pub lcr_h: u32,
    /// 0x30 — Control register.
    pub cr: u32,
    /// 0x34 — Interrupt FIFO level select.
    pub ifls: u32,
    /// 0x38 — Interrupt mask set/clear.
    pub imsc: u32,
    /// 0x3C — Raw interrupt status.
    pub ris: u32,
    /// 0x40 — Masked interrupt status.
    pub mis: u32,
    /// 0x44 — Interrupt clear.
    pub icr: u32,
    /// 0x48 — DMA control.
    pub dmacr: u32,
}

const _: () = {
    use core::mem::offset_of;
    assert!(offset_of!(Pl011Regs, dr) == 0x00);
    assert!(offset_of!(Pl011Regs, rsr_ecr) == 0x04);
    assert!(offset_of!(Pl011Regs, fr) == 0x18);
    assert!(offset_of!(Pl011Regs, ibrd) == 0x24);
    assert!(offset_of!(Pl011Regs, fbrd) == 0x28);
    assert!(offset_of!(Pl011Regs, lcr_h) == 0x2C);
    assert!(offset_of!(Pl011Regs, cr) == 0x30);
    assert!(offset_of!(Pl011Regs, imsc) == 0x38);
    assert!(offset_of!(Pl011Regs, icr) == 0x44);
    assert!(offset_of!(Pl011Regs, dmacr) == 0x48);
};

/// `FR` — flag register bits.
pub mod fr {
    /// Clear to send.
    pub const CTS: u32 = 1 << 0;
    /// Data set ready.
    pub const DSR: u32 = 1 << 1;
    /// Data carrier detect.
    pub const DCD: u32 = 1 << 2;
    /// Transmitter busy (set while a character is still shifting out).
    pub const BUSY: u32 = 1 << 3;
    /// Receive FIFO empty.
    pub const RXFE: u32 = 1 << 4;
    /// Transmit FIFO full.
    pub const TXFF: u32 = 1 << 5;
    /// Receive FIFO full.
    pub const RXFF: u32 = 1 << 6;
    /// Transmit FIFO empty.
    pub const TXFE: u32 = 1 << 7;
    /// Ring indicator.
    pub const RI: u32 = 1 << 8;
}

/// `CR` — control register bits.
pub mod cr {
    /// UART enable.
    pub const UARTEN: u32 = 1 << 0;
    pub const SIREN: u32 = 1 << 1;
    pub const SIRLP: u32 = 1 << 2;
    /// Loopback enable.
    pub const LBE: u32 = 1 << 7;
    /// Transmit enable.
    pub const TXE: u32 = 1 << 8;
    /// Receive enable.
    pub const RXE: u32 = 1 << 9;
    pub const DTR: u32 = 1 << 10;
    pub const RTS: u32 = 1 << 11;
    pub const OUT1: u32 = 1 << 12;
    pub const OUT2: u32 = 1 << 13;
    /// RTS hardware flow control enable.
    pub const RTSEN: u32 = 1 << 14;
    /// CTS hardware flow control enable.
    pub const CTSEN: u32 = 1 << 15;
}

/// `LCR_H` — line control bits.
pub mod lcr_h {
    /// Send break.
    pub const BRK: u32 = 1 << 0;
    /// Parity enable.
    pub const PEN: u32 = 1 << 1;
    /// Even parity select.
    pub const EPS: u32 = 1 << 2;
    /// Two stop bits.
    pub const STP2: u32 = 1 << 3;
    /// Enable FIFOs.
    pub const FEN: u32 = 1 << 4;
    /// Stick parity select.
    pub const SPS: u32 = 1 << 7;

    /// Word length field, bits 6:5.
    pub const fn wlen(bits: u32) -> u32 {
        // 5 bits -> 0b00, 6 -> 0b01, 7 -> 0b10, 8 -> 0b11
        ((bits - 5) & 0b11) << 5
    }
}

/// `IMSC` / `RIS` / `MIS` / `ICR` — interrupt bits (same layout in all four).
pub mod int {
    pub const RIM: u32 = 1 << 0;
    pub const CTSM: u32 = 1 << 1;
    pub const DCDM: u32 = 1 << 2;
    pub const DSRM: u32 = 1 << 3;
    /// Receive interrupt.
    pub const RX: u32 = 1 << 4;
    /// Transmit interrupt.
    pub const TX: u32 = 1 << 5;
    /// Receive timeout.
    pub const RT: u32 = 1 << 6;
    /// Framing error.
    pub const FE: u32 = 1 << 7;
    /// Parity error.
    pub const PE: u32 = 1 << 8;
    /// Break error.
    pub const BE: u32 = 1 << 9;
    /// Overrun error.
    pub const OE: u32 = 1 << 10;

    /// Every interrupt source.
    pub const ALL: u32 = 0x7FF;
    /// All error sources.
    pub const ERRORS: u32 = FE | PE | BE | OE;
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Parity {
    None,
    Even,
    Odd,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StopBits {
    One,
    Two,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FifoLevel {
    Eighth = 0b000,
    Quarter = 0b001,
    Half = 0b010,
    ThreeQuarters = 0b011,
    SevenEighths = 0b100,
}

#[derive(Clone, Copy, Debug)]
pub struct Config {
    pub clock_hz: u32,
    pub baud: u32,
    /// Data bits, 5..=8.
    pub data_bits: u32,
    pub parity: Parity,
    pub stop_bits: StopBits,
    /// Enable the 16-deep hardware FIFOs.
    pub fifo: bool,
    /// RTS/CTS hardware flow control.
    pub flow_control: bool,
    pub rx_trigger: FifoLevel,
    pub tx_trigger: FifoLevel,
}

impl Config {
    /// 115200 8N1, FIFOs on, no flow control.
    pub const fn new(clock_hz: u32) -> Self {
        Self {
            clock_hz,
            baud: 115_200,
            data_bits: 8,
            parity: Parity::None,
            stop_bits: StopBits::One,
            fifo: true,
            flow_control: false,
            rx_trigger: FifoLevel::Half,
            tx_trigger: FifoLevel::Half,
        }
    }

    pub const fn baud(mut self, baud: u32) -> Self {
        self.baud = baud;
        self
    }

    pub const fn data_bits(mut self, bits: u32) -> Self {
        self.data_bits = bits;
        self
    }

    pub const fn parity(mut self, parity: Parity) -> Self {
        self.parity = parity;
        self
    }

    pub const fn stop_bits(mut self, stop: StopBits) -> Self {
        self.stop_bits = stop;
        self
    }

    pub const fn fifo(mut self, on: bool) -> Self {
        self.fifo = on;
        self
    }

    pub const fn flow_control(mut self, on: bool) -> Self {
        self.flow_control = on;
        self
    }

    /// Split `UARTCLK / (16 * baud)` into the integer and 6-bit fractional
    /// divisors. Computed as `(8 * clk) / baud`, which keeps the fraction in
    /// the low 7 bits and lets us round to 6 bits without floating point.
    const fn divisors(&self) -> (u32, u32) {
        let div = (8 * self.clock_hz) / self.baud;
        let ibrd = div >> 7;
        let fbrd = ((div & 0x7F) + 1) / 2;
        (ibrd, fbrd)
    }

    /// The `LCR_H` value implied by this config.
    const fn lcr_h_bits(&self) -> u32 {
        let mut v = lcr_h::wlen(self.data_bits);
        if self.fifo {
            v |= lcr_h::FEN;
        }
        match self.parity {
            Parity::None => {}
            Parity::Even => v |= lcr_h::PEN | lcr_h::EPS,
            Parity::Odd => v |= lcr_h::PEN,
        }
        if matches!(self.stop_bits, StopBits::Two) {
            v |= lcr_h::STP2;
        }
        v
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Error {
    /// No data in the receive FIFO.
    WouldBlock,
    /// FIFO overran; a byte was lost.
    Overrun,
    /// Bad stop bit.
    Framing,
    /// Parity mismatch.
    Parity,
    /// Line held low longer than a frame.
    Break,
}

pub struct Pl011 {
    base: usize,
}

macro_rules! rd {
    ($s:expr, $field:ident) => {
        unsafe { core::ptr::read_volatile(core::ptr::addr_of!((*$s.regs()).$field)) }
    };
}

macro_rules! wr {
    ($s:expr, $field:ident, $val:expr) => {
        unsafe {
            core::ptr::write_volatile(core::ptr::addr_of_mut!((*$s.regs()).$field), $val)
        }
    };
}

impl Pl011 {
    pub const unsafe fn new(base: usize) -> Self {
        Self { base }
    }

    pub const fn base(&self) -> usize {
        self.base
    }

    #[inline(always)]
    const fn regs(&self) -> *mut Pl011Regs {
        self.base as *mut Pl011Regs
    }

    /// Reset and configure the UART.
    ///
    /// Follows the TRM's ordering: disable, drain, reprogram, re-enable.
    /// `LCR_H` must be written *after* `IBRD`/`FBRD` — that write is what
    /// latches the new divisors.
    pub fn init(&mut self, cfg: &Config) {
        // 1. Disable the UART.
        wr!(self, cr, 0);

        // 2. Let any in-flight character finish shifting out.
        while rd!(self, fr) & fr::BUSY != 0 {}

        // 3. Flush the FIFOs by clearing FEN.
        let lcr = rd!(self, lcr_h);
        wr!(self, lcr_h, lcr & !lcr_h::FEN);

        // 4. Mask and clear every interrupt before we touch anything else.
        wr!(self, imsc, 0);
        wr!(self, icr, int::ALL);

        // 5. Baud divisors.
        let (ibrd, fbrd) = cfg.divisors();
        wr!(self, ibrd, ibrd);
        wr!(self, fbrd, fbrd);

        // 6. Line control. This write latches IBRD/FBRD.
        wr!(self, lcr_h, cfg.lcr_h_bits());

        // 7. FIFO trigger levels.
        wr!(
            self,
            ifls,
            (cfg.tx_trigger as u32) | ((cfg.rx_trigger as u32) << 3)
        );

        // 8. Enable.
        let mut c = cr::UARTEN | cr::TXE | cr::RXE;
        if cfg.flow_control {
            c |= cr::RTSEN | cr::CTSEN;
        }
        wr!(self, cr, c);
    }

    /// Disable the UART, draining the transmitter first.
    pub fn disable(&mut self) {
        self.flush();
        wr!(self, cr, 0);
    }

    // -- transmit ----------------------------------------------------------

    /// Write one byte, spinning until there is room in the transmit FIFO.
    pub fn write_byte(&mut self, byte: u8) {
        while rd!(self, fr) & fr::TXFF != 0 {}
        wr!(self, dr, byte as u32);
    }

    /// Write one byte if the FIFO has room, otherwise return the byte back.
    pub fn try_write_byte(&mut self, byte: u8) -> Result<(), u8> {
        if rd!(self, fr) & fr::TXFF != 0 {
            Err(byte)
        } else {
            wr!(self, dr, byte as u32);
            Ok(())
        }
    }

    /// Write a slice, blocking until it has all been handed to the FIFO.
    pub fn write_bytes(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.write_byte(b);
        }
    }

    /// Block until the transmitter has fully drained (FIFO empty *and* the
    /// shift register idle). Call this before resetting or powering down.
    pub fn flush(&mut self) {
        while rd!(self, fr) & fr::BUSY != 0 {}
    }

    // -- receive -----------------------------------------------------------

    /// Read one byte if the receive FIFO is non-empty.
    ///
    /// Reading `DR` pops the FIFO, so the error flags in the upper bits belong
    /// to the byte returned in the low 8.
    pub fn read_byte(&mut self) -> Result<u8, Error> {
        if rd!(self, fr) & fr::RXFE != 0 {
            return Err(Error::WouldBlock);
        }

        let dr = rd!(self, dr);

        // Bits 11:8 of DR are the per-character error flags.
        if dr & (1 << 11) != 0 {
            wr!(self, rsr_ecr, 0);
            return Err(Error::Overrun);
        }
        if dr & (1 << 10) != 0 {
            wr!(self, rsr_ecr, 0);
            return Err(Error::Break);
        }
        if dr & (1 << 9) != 0 {
            wr!(self, rsr_ecr, 0);
            return Err(Error::Parity);
        }
        if dr & (1 << 8) != 0 {
            wr!(self, rsr_ecr, 0);
            return Err(Error::Framing);
        }

        Ok(dr as u8)
    }

    /// Spin until a byte arrives.
    pub fn read_byte_blocking(&mut self) -> Result<u8, Error> {
        loop {
            match self.read_byte() {
                Err(Error::WouldBlock) => core::hint::spin_loop(),
                other => return other,
            }
        }
    }

    // -- interrupts --------------------------------------------------------

    /// Unmask the given interrupt sources (see [`int`]).
    pub fn enable_interrupts(&mut self, mask: u32) {
        let cur = rd!(self, imsc);
        wr!(self, imsc, cur | mask);
    }

    /// Mask the given interrupt sources.
    pub fn disable_interrupts(&mut self, mask: u32) {
        let cur = rd!(self, imsc);
        wr!(self, imsc, cur & !mask);
    }

    /// Which enabled interrupts are currently asserted.
    pub fn interrupt_status(&self) -> u32 {
        rd!(self, mis)
    }

    /// Acknowledge interrupts. Do this *before* draining the FIFO, so a byte
    /// arriving mid-handler isn't dropped.
    pub fn clear_interrupts(&mut self, mask: u32) {
        wr!(self, icr, mask);
    }

    // -- status ------------------------------------------------------------

    /// Raw flag register.
    pub fn flags(&self) -> u32 {
        rd!(self, fr)
    }

    pub fn tx_full(&self) -> bool {
        rd!(self, fr) & fr::TXFF != 0
    }

    pub fn rx_empty(&self) -> bool {
        rd!(self, fr) & fr::RXFE != 0
    }

    pub fn busy(&self) -> bool {
        rd!(self, fr) & fr::BUSY != 0
    }
}

impl fmt::Debug for Pl011 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Pl011")
            .field("base", &format_args!("{:#010x}", self.base))
            .finish()
    }
}

impl fmt::Write for Pl011 {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            if byte == b'\n' {
                self.write_byte(b'\r');
            }
            self.write_byte(byte);
        }
        Ok(())
    }
}
