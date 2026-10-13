//! hello-rs — emotion-rs Phase 1 example: Rust game logic on the PS2.
//!
//! Architecture: this crate is a `#![no_std]` staticlib linked into the
//! ee-gcc C shim (`shim/main.c`). The FFI surface is INTEGER-ONLY:
//! ee-gcc compiles hard-float o32 while this target is `+soft-float`,
//! so floats, 64-bit ints, and varargs may never cross the boundary
//! (see `src/ffi.rs` for the soundness contract).
//!
//! Draw-command routing: all animation math (positions, velocities,
//! bounce, pulse, palette) lives here in Rust; the C side owns the
//! hardware (gsKit/libpad) and converts integer pixel coordinates to
//! floats at the last moment.

#![no_std]

#[cfg(test)]
extern crate std;

#[cfg(not(test))]
mod ffi;
mod game;
mod rgbaq;

#[cfg(not(test))]
use core::panic::PanicInfo;

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    // Phase 1: halt. A frozen frame is the visible symptom.
    loop {}
}

/// C shim entry point: called after gsKit/libpad init with the current
/// video mode dimensions. Runs the game loop until START is pressed.
#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn rust_main(width: u32, height: u32) -> i32 {
    let mut game = game::Game::new(width, height);
    let mut renderer = ffi::FfiRenderer;
    loop {
        let pad = ffi::pad_buttons();
        if !game.step(pad, &mut renderer) {
            break;
        }
    }
    0
}
