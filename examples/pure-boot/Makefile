# pure-boot — pure-Rust PS2 ELF, linked with EE binutils directly
# (no gcc driver: cargo -> mips64r5900el-ps2-elf-ld, flavor `ld`).
#
# Runs inside homelab/ps2-build via ps2-build.sh; must produce game.iso.
# rust/target and rust/.cargo-home are server-side caches: `clean` spares them.

TOOL   = mips64r5900el-ps2-elf
ELF    = rust/target/mipsel-sony-ps2/release/pure-boot
BOOT   = SLUS_900.02

RUST_ARGS = --release -Zbuild-std=core,compiler_builtins \
            -Zbuild-std-features=compiler-builtins-mem \
            -Zjson-target-spec --target targets/mipsel-sony-ps2.json
# Link straight with ld. The ps2dev ld defaults to the N32 emulation
# (elf32lr5900n32); our objects are o32 -> force elf32lr5900.
# emotion-rt's build script injects -T<OUT_DIR>/linkfile.ld; this absolute
# -T is the guaranteed path (duplicate -T flags are harmless).
RUSTFLAGS_LINK = -C linker=$(TOOL)-ld -C linker-flavor=ld \
                 -C link-arg=-m -C link-arg=elf32lr5900 \
                 -C link-arg=-T$${PWD}/crates/emotion-rt/user-linkfile.ld

all: iso

.PHONY: rust
rust:
	cd rust && CARGO_HOME=$$(pwd)/.cargo-home RUSTFLAGS="$(RUSTFLAGS_LINK)" \
	  cargo build $(RUST_ARGS)

# Force the final ELF's e_flags to ee-gcc's 0x20920025 (mips3/r5900,
# hard-single) — same byte patch as Phase 1's patch-abi.sh, applied
# post-link; codegen untouched. Then gate on ELF shape + entry code.
.PHONY: patch
patch: rust
	printf '\045\000\222\040' | dd of=$(ELF) bs=1 seek=36 conv=notrunc 2>/dev/null
	$(TOOL)-readelf -h $(ELF) | grep -E 'Entry point|Flags'
	$(TOOL)-readelf -l $(ELF) | grep -E 'LOAD'
	$(TOOL)-objdump -d --start-address=0x100000 --stop-address=0x100040 $(ELF)

iso: patch
	rm -rf cdroot && mkdir -p cdroot
	cp $(ELF) cdroot/$(BOOT)
	printf "BOOT2 = cdrom0:\\\\$(BOOT);1\nVER = 1.00\nVMODE = NTSC\n" > cdroot/SYSTEM.CNF
	xorriso -as mkisofs -l -r -V PURE_BOOT -o game.iso cdroot/

clean:
	rm -f game.iso
	rm -rf cdroot
