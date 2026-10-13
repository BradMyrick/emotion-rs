//! GS RGBAQ color packing (Rust port of the `GS_SETREG_RGBAQ` layout).
//!
//! GS register layout: R[0:8) G[8:16) B[16:24) A[24:32) Q[32:42).
//! Colors cross the C boundary as the low 32 bits (Q implied 0); the
//! shim widens `u32` to `u64`, which is exactly `GS_SETREG_RGBAQ(r,g,b,a,0)`.

/// Pack R, G, B, A into the GS RGBAQ low word.
pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> u32 {
    (r as u32) | ((g as u32) << 8) | ((b as u32) << 16) | ((a as u32) << 24)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packs_fields_in_gs_order() {
        assert_eq!(rgba(0x10, 0x14, 0x22, 0x00), 0x0022_1410);
        assert_eq!(rgba(0xff, 0xff, 0xff, 0x50), 0x50ff_ffff);
        assert_eq!(rgba(0x00, 0xf0, 0xff, 0x00), 0x00ff_f000);
    }

    #[test]
    fn zero_is_transparent_black() {
        assert_eq!(rgba(0, 0, 0, 0), 0);
    }
}
