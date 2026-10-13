//! emotion-rt — minimal pure-Rust runtime for the PS2 Emotion Engine.
//!
//! Provides the three pieces a pure-Rust PS2 binary needs to boot:
//! EE crt0 glue ([`__rust_start`], reached from `_start` in `start.S`),
//! rom0 kernel syscalls ([`syscall`]), and GS register-window access
//! ([`gs`]).
//!
//! # Invariants
//!
//! * Integer-only, at most 4 register args at every boundary into asm or
//!   rom0 (same discipline proven in Phase 1; the target is soft-float,
//!   rom0 is not).
//! * Valid only when entered via BIOS BOOT2/EELOAD: the rom0 kernel is
//!   resident, user memory is 0x0010_0000..0x0200_0000, and `_start` in
//!   `start.S` has already zeroed `.bss`, registered the thread with the
//!   kernel (SetupThread) and handed us a stack at its top.
//!
//! # The `main` contract
//!
//! A binary links `emotion-rt` and provides:
//!
//! ```ignore
//! #[no_mangle]
//! fn main() -> ! { /* ... */ }
//! ```

#![no_std]

#[cfg(test)]
extern crate std;

pub mod gs;
pub mod syscall;

/// Entered by `_start` once the machine is ready: `.bss` zeroed, thread
/// registered with the kernel, stack and interrupts live.
///
/// Never returns; `main` is `-> !` and the only exits are the rom0
/// syscalls.
///
/// # Safety
///
/// Must only be entered from `_start` in `start.S`, which establishes
/// the boot invariants (SetupThread-registered kernel thread, valid
/// stack, zeroed `.bss`) this function relies on.
#[cfg(not(test))]
#[no_mangle]
pub unsafe extern "C" fn __rust_start() -> ! {
    extern "Rust" {
        fn main() -> !;
    }
    main()
}
