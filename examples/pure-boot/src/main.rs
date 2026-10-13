//! pure-boot — emotion-rs Phase 2 acceptance demo.
//!
//! A 100% Rust ELF: crt0 (`emotion-rt`) sets up the machine, we hold a
//! blank screen synced on GS field flips for ~5 seconds, then return to
//! the browser via rom0 `exit`.
//!
//! The hold is also a three-way self test, readable from the couch:
//! browser back at ~5s means field sync + exit both work; ~10s means
//! the field poll is stuck and the cop0 deadline fired; never means
//! exit itself is broken.

#![no_std]
#![cfg_attr(not(test), no_main)]

#[cfg(test)]
extern crate std;

#[cfg(not(test))]
use core::{
    panic::PanicInfo,
    sync::atomic::{AtomicU32, Ordering},
};

#[cfg(not(test))]
use emotion_rt::{
    gs,
    syscall::{self, FieldFrame, Interlace, VideoMode},
};

/// Hold time in accepted fields (~60/s NTSC): ~5s.
#[cfg(not(test))]
const FIELDS: u32 = 300;

/// One accepted field: at least one raw CSR field-state change PLUS
/// either a full field period on the cop0 timer (147.456 MHz), or a
/// raw-flip cap as a bounded fallback if the timer turns out not to
/// tick (raw CSR state flips far faster than true vsync on PCSX2 —
/// empirically ~kHz).
const FIELD_TICKS: u32 = 147_456_000 / 60;
const RAW_FLIP_CAP: u32 = 1000;

const fn field_complete(ticks: u32, raw_flips: u32) -> bool {
    ticks >= FIELD_TICKS || raw_flips >= RAW_FLIP_CAP
}

/// cop0 Count ticks at 147.456 MHz; 10s in ticks is the hang guard for
/// the whole hold (wraps u32 at ~29s, so all arithmetic is wrapping).
const DEADLINE_TICKS: u32 = 1_474_560_000;

/// Field counter: also a live LL/SC smoke test (Phase 0 open item) —
/// if atomics were broken on this target, this loop would derail.
#[cfg(not(test))]
static FIELDS_SEEN: AtomicU32 = AtomicU32::new(0);

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

/// Wrapping deadline check (cop0 Count wraps every ~29s).
const fn deadline_hit(start: u32, now: u32) -> bool {
    now.wrapping_sub(start) >= DEADLINE_TICKS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_completes_on_timer_or_raw_cap() {
        // Neither gate hit yet.
        assert!(!field_complete(0, 0));
        assert!(!field_complete(FIELD_TICKS - 1, RAW_FLIP_CAP - 1));
        // Timer gate.
        assert!(field_complete(FIELD_TICKS, 0));
        // Raw-flip fallback gate.
        assert!(field_complete(0, RAW_FLIP_CAP));
    }

    #[test]
    fn deadline_arithmetic_wraps_cleanly() {
        assert!(!deadline_hit(0, 0));
        assert!(!deadline_hit(0, DEADLINE_TICKS - 1));
        assert!(deadline_hit(0, DEADLINE_TICKS));
        // Counter wrapped mid-hold: start just below the wrap, `now`
        // is (start + DEADLINE_TICKS) mod 2^32.
        let start = u32::MAX - 5;
        assert!(deadline_hit(start, start.wrapping_add(DEADLINE_TICKS)));
        assert!(!deadline_hit(start, start.wrapping_add(DEADLINE_TICKS - 1)));
    }
}

#[cfg(not(test))]
#[no_mangle]
fn main() -> ! {
    syscall::set_gs_crt(Interlace::Interlaced, VideoMode::Ntsc, FieldFrame::Frame);

    // NOTE: no PMODE touch. Disabling the display read circuits freezes
    // PCSX2's field counter (rasterizer idle -> CSR.FLD never toggles),
    // which stalled the vsync poll; leaving the BIOS display state alone
    // keeps the field clock running.

    let start = syscall::read_count();
    let mut last = gs::field_state(unsafe { gs::CSR.read() });
    let mut i: u32 = 0;
    let mut beaconed = false;
    loop {
        // Accumulate raw field-state changes until one full field period
        // elapses on the cop0 timer (or the raw cap hits — bounded).
        let field_start = syscall::read_count();
        let mut raw: u32 = 0;
        loop {
            last = gs::wait_field_flip(last);
            raw += 1;
            if field_complete(syscall::read_count().wrapping_sub(field_start), raw) {
                break;
            }
        }
        i += 1;
        FIELDS_SEEN.fetch_add(1, Ordering::Relaxed);
        // Trace beacon: first field + first cop0 read survived iteration 1
        // (SetGsCrt calls each leave a "Mode Changed" line in PCSX2's log).
        if !beaconed {
            beaconed = true;
            syscall::set_gs_crt(Interlace::Interlaced, VideoMode::Pal, FieldFrame::Frame);
        }
        if i >= FIELDS || deadline_hit(start, syscall::read_count()) {
            // Trace beacon: hold finished, LoadExecPS2 is next.
            syscall::set_gs_crt(Interlace::Interlaced, VideoMode::Ntsc, FieldFrame::Frame);
            // Universal exit: works from BOOT2 disc boots AND direct
            // ELF launches (KExit would only cover the disc case).
            syscall::exit_to_browser();
        }
    }
}
