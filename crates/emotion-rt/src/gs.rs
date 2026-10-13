//! GS (Graphics Synthesizer) privileged register-window access.
//!
//! GS registers are 64-bit and memory-mapped at
//! `0x1200_0000 + (register_index << 4)`. All access is volatile:
//! these are device registers with side effects, not memory.

/// A handle to one GS register in the privileged window.
#[derive(Clone, Copy)]
pub struct GsReg {
    addr: usize,
}

/// Single 64-bit device access trampolines (`ld`/`sd` in `start.S`).
///
/// LLVM's mips2 codegen splits u64 volatile accesses into 32-bit halves,
/// which GS registers do not answer correctly — all register I/O must go
/// through these (prussia's `_read_u64`/`_write_u64` exist for the same
/// reason).
#[cfg(not(test))]
mod asm {
    extern "C" {
        pub(super) fn __rt_ld64(addr: u32) -> u64;
        pub(super) fn __rt_sd64(addr: u32, lo: u32, hi: u32);
    }
}

impl GsReg {
    /// Map a register index (per the GS manual / gsKit's `gsInit.h`).
    pub const fn new(index: u16) -> Self {
        Self { addr: 0x1200_0000 + ((index as usize) << 4) }
    }

    /// Single-`ld` volatile read of the register.
    ///
    /// # Safety
    ///
    /// The address is a fixed device register: always mapped, never
    /// freed, and volatile access is the only sound way to touch it.
    #[cfg(not(test))]
    pub unsafe fn read(&self) -> u64 {
        // SAFETY: trampoline performs one 64-bit load at our fixed address.
        unsafe { asm::__rt_ld64(self.addr as u32) }
    }

    /// Single-`sd` volatile write of the register.
    ///
    /// # Safety
    ///
    /// See [`read`](Self::read); writing may change machine state.
    #[cfg(not(test))]
    pub unsafe fn write(&self, value: u64) {
        // SAFETY: trampoline performs one 64-bit store at our fixed address.
        unsafe {
            asm::__rt_sd64(self.addr as u32, value as u32, (value >> 32) as u32)
        }
    }
}

/// GS System Status register (`CSR`, index 0x100 → 0x1200_1000).
pub const CSR: GsReg = GsReg::new(0x100);
/// GS Background Color register (`BGCOLOR`, index 0x0E → 0x1200_00E0).
pub const BGCOLOR: GsReg = GsReg::new(0x0E);
/// GS Output Mode register (`PMODE`, index 0x00 → 0x1200_0000).
pub const PMODE: GsReg = GsReg::new(0x00);

/// CSR bits 12-13: `NFIELD` | `FIELD` — the field state (per PCSX2's
/// `GSReg::CSR`: rNFIELD = bit 12, rFIELD = bit 13; bits 14-15 are the
/// FIFO status, which stays constant while idle — do not poll those).
pub const CSR_FIELD_MASK: u64 = 0x3 << 12;
/// CSR bit 9: GS soft reset (gsKit `GS_RESET`).
pub const CSR_RESET: u64 = 1 << 9;

/// Extract the 2-bit field state (`NFIELD`|`FIELD`, CSR bits 12-13)
/// from a CSR read. Toggles each vsync in interlaced output.
pub const fn field_state(csr: u64) -> u8 {
    ((csr & CSR_FIELD_MASK) >> 12) as u8
}

/// Pack an RGB value into `BGCOLOR` layout: `R | G<<8 | B<<16`
/// (matches gsKit `GS_SET_BGCOLOR`).
pub const fn bgcolor(r: u8, g: u8, b: u8) -> u64 {
    (r as u64) | ((g as u64) << 8) | ((b as u64) << 16)
}

/// Spin until the field state (CSR bits 12-13) differs from `last`,
/// returning the new state.
///
/// This is a field-rate (~60Hz NTSC) vsync: no interrupts, no GS
/// interrupt-mask setup — the cheapest possible frame sync.
#[cfg(not(test))]
pub fn wait_field_flip(last: u8) -> u8 {
    loop {
        // SAFETY: CSR is a fixed device register (see GsReg::read).
        let csr = unsafe { CSR.read() };
        let f = field_state(csr);
        if f != last {
            return f;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gsreg_addressing_matches_gskit() {
        assert_eq!(GsReg::new(0x100).addr, 0x1200_1000); // CSR
        assert_eq!(GsReg::new(0x0E).addr, 0x1200_00E0); // BGCOLOR
    }

    #[test]
    fn bgcolor_layout_matches_gskit_macro() {
        // GS_SET_BGCOLOR(R,G,B): R<<0 | G<<8 | B<<16
        assert_eq!(bgcolor(1, 2, 3), 0x030201);
        assert_eq!(bgcolor(0, 0, 96), 96 << 16);
    }

    #[test]
    fn field_state_extracts_csr_bits_12_13() {
        assert_eq!(field_state(0), 0);
        assert_eq!(field_state(1 << 12), 1); // NFIELD
        assert_eq!(field_state(1 << 13), 2); // FIELD
        assert_eq!(field_state(CSR_FIELD_MASK), 3);
        // FIFO status (bits 14-15) must not leak into the field state.
        assert_eq!(field_state(3 << 14), 0);
        assert_eq!(field_state(CSR_RESET), 0);
    }
}
