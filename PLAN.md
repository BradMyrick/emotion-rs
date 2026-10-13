# emotion-rs — Plan

Mission: Rust game code on the PS2, a pure-Rust PS2 runtime, and an
upstream `mipsel-sony-ps2` target. Execution is incremental; every phase
ends with something bootable through the homelab pipeline
(`ps2-build.sh` → CHD → PCSX2/Wolf/Moonlight).

## Ground rules

- **Every phase ships a disc.** No long-lived branches of pure theory.
- **Soft-float by default.** The R5900 FPU is nonstandard (paired-single,
  no full IEEE in hardware paths); LLVM's mips backend is untested there.
  `+soft-float` buys correctness now; hard-float is a measured, later win.
- **C is the fallback at every layer.** Hybrid first, pure-Rust later.
- **Pin nightly.** `rust-toolchain.toml` (nightly-2026-08-03 to start);
  bump deliberately, never implicitly.

## Phase 0 — Target spec revival ✅ (validated 2026-10-12)

Modernize prussia's spec (done in draft: `targets/mipsel-sony-ps2.json`)
and prove it on current nightly.

Work items:
1. Fix data-layout drift: modern LLVM wants `e-m:e-p:32:32-...`
   (ELF mangling), prussia's is pre-2019 format. → **Kept `e-m:m`
   (MIPS mangling, per upstream mipsel-sony-psp); fixed instead:
   numeric `target-c-int-width`/`target-pointer-width`, added
   `llvm-abiname: o32` and `crt-objects-fallback: false`.**
2. Build `core` + `compiler-builtins` for the target. → **Works:
   `cargo -Zjson-target-spec -Zbuild-std...` (modern cargo requires
   `-Zjson-target-spec` for .json specs — note for all future builds).**
3. Verify emitted assembly is sane R5900 code. → **Verified:
   `spec_test_add` disassembles to clean o32 R5900 code
   (`addu v0,a0,a1; jr ra`) under mips64r5900el-ps2-elf-objdump.**
4. Confirm `max-atomic-width: 32` (R5900 has LL/SC). → **Spec set;
   runtime LL/SC validation still TODO in Phase 2.**

Acceptance: a `#![no_std]` staticlib compiles for the spec and its object
code passes EE objdump inspection with zero non-R5900 instructions.

## Phase 1 — Hybrid disc (Rust logic + C hardware) ✅ (2026-10-12)

Architecture:

```
examples/hello-rs/src/  no_std staticlib: game code, extern "C" into the shim
examples/hello-rs/shim/ main.c (gsKit/libpad, all hardware state) + Makefile
                         + patch-abi.sh (LLVM ELF-flag rewrite for binutils)
scripts/sync-example.sh rsyncs crate + shim into the server's build tree
rust/ (server)          workspace subset: manifests, targets/ spec, crate
```

Work items:
1. `examples/hello-rs` mirroring hello-ps2 feature-for-feature. → **Done:
    gradient, pulsing gem, bouncing sprite, X palette, START exits.**
2. `homelab/ps2-build` image with pinned rustup nightly. → **Done:
    nightly-2026-08-03 + rust-src + host gcc (build scripts need it);
    zero-dep build except libm, cached under `rust/.cargo-home`.**
3. FFI boundary rules documented. → **Done: integer-only boundary
    (`src/ffi.rs` contract). ee-gcc is hard-float o32, the Rust target
    is +soft-float — floats never cross; ints in, GS u32 colors out.**
4. (Found during link) ps2dev binutils refuses LLVM objects: `.MIPS.abiflags`
   says soft-float, e_flags say mips2. → **`shim/patch-abi.sh` strips the
   section and forces e_flags to ee-gcc's `0x20920025`. Codegen untouched;
    runtime-safe because the boundary is integer-only.**

Acceptance: Rust-built CHD boots via Moonlight, behavior identical to the
C demo. This is the first "memory-safe PS2 game" milestone.

## Phase 2 — Pure-Rust boot ✅ (2026-10-12/13, user-verified on PCSX2 2.8.2)

1. `emotion-rt`: crt0 in Rust/asm — EE context setup, kernel-init call
    (or raw TLB/diag path per prussia_rt), `#[no_mangle] main`. →
    **Done: `crates/emotion-rt` — `start.S` clears .bss (`sq` loop),
    registers the thread via SetupThread (syscall 60 — REQUIRED: without
    it KExit just kills the thread, black screen), switches to the
    kernel-provided stack, `ei`, then Rust `__rust_start` → `main`.**
2. Own linkfile.ld (EE memory map: kernel at 0x8000_0000, user above). →
    **Done: `user-linkfile.ld` — user image at 0x0010_0000 (kernel
    reserves the first MiB), `ENTRY`+`EXTERN(_start)`, `_gp`, BSS
    bounds, `/DISCARD/` for `.MIPS.abiflags`.**
3. Link with the EE binutils directly (no gcc driver). → **Done: cargo
    `-C linker=mips64r5900el-ps2-elf-ld -C linker-flavor=ld`; the ps2dev
    ld defaults to the N32 emulation so `-m elf32lr5900` is mandatory;
    pure-LLVM object sets link with no abiflags fights.**
4. Exit-to-BIOS via rom0 `exit()` syscall (prussia_bios has the ABI). →
    **Done, with corrections to prussia's ABI: syscall 4 is `KExit` and
    only returns to the browser for a SetupThread-registered main
    thread; `VideoMode` values are inverted vs prussia (gsKit:
    NTSC=2, PAL=3); GS registers need single `ld`/`sd` accesses (LLVM
    mips2 splits u64 volatiles into lw-pairs, and ps2dev `as` under -32
    macro-expands ld/sd the same way — `.set push/mips3/pop` fixes the
    assembler side).**

Acceptance: a Rust-only ELF boots to a blank/vsync'd screen in PCSX2
("Run ELF") and cleanly returns to the BIOS on exit. → **Met on BOTH
boot paths (user-verified): BOOT2 disc and direct "Run ELF" hold a
cop0-Count-validated ~5s field-synced screen (300 fields at 60Hz, CSR
bits 12-13 = NFIELD|FIELD; bits 14-15 are FIFO status — do not poll
those), then exit via `exit_to_browser()` — ps2sdk's
`LoadExecPS2("rom0:OSDSYS")` path (0x5A copier + kernel exec block at
0x80075000) — which reboots through the full boot sequence into the
browser. Zero C in the final ELF (5.5KB).**

Also shipped: `emotion-rt::gs` (64-bit GS register window with
`__rt_ld64`/`__rt_sd64` trampolines, CSR/BGCOLOR/PMODE constants),
cop0 Count timer access, `scripts/sync.sh` (generalized example
sync), and a SetGsCrt-beacon remote-trace technique (every call logs
"Mode Changed" in emulog — the only remote execution trace channel;
see AGENTS.md gotchas).

## Phase 3 — SDK crates (safe hardware access)

Priority order (each independently useful):
1. `emotion-dma` — DMAC channels, GIF path (this unlocks graphics).
2. `emotion-gs` — framebuffer + primitives via GIF packets; enough to
   port the hello demo to pure Rust.
3. `emotion-sif` — IOP services: pad input, memory cards.
4. `emotion-spu2` — audio out.
5. `emotion-pad` — safe controller abstraction on sif.

Design notes: newtype-per-register (á la playstation2-pac-rs), DMA
transfers as owned buffers (the "DMA can't read freed memory" guarantee),
`nb`-style async later if warranted.

Acceptance: hello-rs demo runs 100% Rust, no C in the final ELF.

## Phase 4 — Upstream

1. Land the spec as `mipsel-sony-ps2` in rust-lang/rust (tier 3:
   in-tree spec + docs; no CI requirement initially, then opt-in CI
   building core).
2. Companion PRs: rustc target docs, maybe `--print target-spec-json`.
3. Keep emotion-rs as the SDK layer (targets are std-free; SDKs stay
   out-of-tree, like most embedded targets).

Risks: compiler team review cycles; spec churn (fields get renamed —
e.g. the `is-builtin`/`abi` migrations); needs a named maintainer
(that's us).

## Risks & unknowns (tracked, not feared)

| Risk | Mitigation |
|---|---|
| LLVM miscompiles R5900 patterns | soft-float; differential-test vs C; integer-first |
| o32 ABI edge cases (`long double`, varargs on 128-bit regs) | never cross FFI with them; document as UB-free subset |
| Nightly `-Zbuild-std` drift | pinned toolchain, CI matrix on bump |
| prussia license (AFL-2.0) heritage | spec rewritten fresh; runtime written clean-room where practical, credited where copied |
| Nobody's done this | that's the point |

## Test rig

`ps2-build.sh <dir> "<Name>"` on the server compiles, authors the ISO
(SYSTEM.CNF/BOOT2), CHDs it into the library, and any Moonlight screen
launches it in under a minute from edit. This loop is the project's
superpower — guard it.
