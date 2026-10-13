//! Integer-only FFI boundary to the C shim (`shim/main.c`).
//!
//! # Soundness contract
//!
//! Every function below is implemented on the C side as a plain C
//! function taking and returning only `u32`/`i32` by value. Nothing
//! else may ever cross this boundary:
//!
//! - **no floats**: ee-gcc compiles hard-float o32 (float args in
//!   `$f12`/`$f14`), while this target is `+soft-float` (float args in
//!   GP registers). A float argument would be read from the wrong
//!   register — silent garbage, not a link error.
//! - **no `u64`/`i64`**, no varargs: o32 register-pair rules are a
//!   needless risk when nothing needs them.
//! - **no pointers or buffers**: there is nothing to alias, dereference,
//!   or overrun, so the calls cannot violate memory safety.
//! - **at most 4 arguments** (register-only): a 5th argument means the
//!   o32 stack-arg slot, and LLVM schedules that store into the `jal`
//!   delay slot — PCSX2 2.8.2's recompiler lost it in transit (verified
//!   on-screen: rodata readbacks green, C-side receipt red). State that
//!   would be a 5th arg travels via a dedicated register-only setter.
//!
//! GS colors cross as the packed `u32` RGBAQ low word (Q implied 0);
//! the shim widens to `u64` with the `GS_SETREG_RGBAQ` layout.

use crate::game::Renderer;

extern "C" {
    fn hr_pad_buttons() -> u32;
    fn hr_clear(rgba: u32);
    fn hr_gradient();
    fn hr_gem(cx: i32, cy: i32, r: i32, inner_half: i32);
    fn hr_set_color(rgba: u32);
    fn hr_sprite(x: i32, y: i32, w: i32, h: i32);
    fn hr_frame_end();
}

/// Current pad bitfield, active-high (`0xffff ^ padButtonStatus.btns`).
pub fn pad_buttons() -> u32 {
    // SAFETY: integer return value only; the shim side has no memory effects.
    unsafe { hr_pad_buttons() }
}

/// `Renderer` backed by the C shim. All `unsafe` in this crate is
/// confined to this module.
pub(crate) struct FfiRenderer;

impl Renderer for FfiRenderer {
    fn clear(&mut self, rgba: u32) {
        // SAFETY: integer by value; contract above.
        unsafe { hr_clear(rgba) }
    }

    fn gradient(&mut self) {
        // SAFETY: no arguments, no return; contract above.
        unsafe { hr_gradient() }
    }

    fn gem(&mut self, cx: i32, cy: i32, r: i32, inner_half: i32) {
        // SAFETY: integers by value; contract above.
        unsafe { hr_gem(cx, cy, r, inner_half) }
    }

    fn sprite(&mut self, x: i32, y: i32, w: i32, h: i32, rgba: u32) {
        // SAFETY: integers by value; contract above. Color travels via a
        // separate register-only call (hr_set_color) — see the note in
        // shim/main.c about 5th stack args.
        unsafe {
            hr_set_color(rgba);
            hr_sprite(x, y, w, h);
        }
    }

    fn frame_end(&mut self) {
        // SAFETY: no arguments, no return; contract above.
        unsafe { hr_frame_end() }
    }
}
