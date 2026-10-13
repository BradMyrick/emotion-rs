# emotion-rs

**Rust for the PlayStation 2's Emotion Engine.**

Three goals, in order:

1. **Hybrid Rust + C homebrew today** — Rust `no_std` game logic linked
   against the proven C stack (gsKit, libpad, ps2sdk) with `ee-gcc`.
   Memory-safe game code, working graphics/input from day one.
2. **A standalone pure-Rust PS2 runtime** — crt0, DMA, GIF/GS drawing,
   SIF/pad, SPU2 — as safe Rust crates (`prussia` showed the way;
   we go further).
3. **An upstream target** — `mipsel-sony-ps2` as a tier-3 target in
   rust-lang/rust, so nobody has to hand around a target JSON again.

## Why

- AI-assisted game generation writes a *lot* of pointer-and-buffer code.
  Memory safety is the difference between "runs" and "runs correctly."
  The PS2's DMA engine happily reads unallocated memory if you tell it to;
  Rust's type system can make that unrepresentable.
- Rust upstream already ships `mipsel-sony-psp` and `mipsel-sony-psx`.
  The PS2 is the conspicuous gap. The EE (R5900) is broadly LLVM-mips
   compatible; the blocker was never codegen, just nobody doing the work.
- We have a full test rig: source → `ps2-build.sh` → bootable CHD →
  PCSX2 via Wolf/Moonlight on three screens. Build-to-couch in ~20s.

## Status

- Phase 0 validated (2026-10-12): the target spec builds `core` +
  `compiler-builtins` on nightly-2026-08-03 and emits verified R5900/o32
  machine code.
- Phase 1 done (2026-10-12): `examples/hello-rs` — a full hybrid disc.
  Rust `no_std` staticlib (game logic, soft-float, integer-only FFI)
  linked against gsKit/libpad by ee-gcc, shipped as a bootable CHD.
  See [PLAN.md](PLAN.md) for the roadmap,
  [targets/mipsel-sony-ps2.json](targets/mipsel-sony-ps2.json) for the
  spec (modernized from prussia's circa-2020 original), and
  `examples/hello-rs/shim/` for the C side (including `patch-abi.sh`,
  which rewrites LLVM ELF flags for ps2dev binutils).

## Prior art / credit

- [prussia](https://github.com/Ravenslofty/prussia) — the original Rust
  PS2 SDK attempt; its target spec, runtime ideas, and
  playstation2-pac-rs are the foundation we build on.
- [ps2dev](https://github.com/ps2dev) — the C toolchain (EE gcc, ps2sdk,
  gsKit) our hybrid phase links against.

## License

MIT (see LICENSE). Spec and runtime work adapted from prussia credited
in-file where applicable.
