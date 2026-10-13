#!/bin/sh
# patch-abi.sh <archive-in> <archive-out> — make LLVM-emitted MIPS objects
# linkable by the ps2dev-patched binutils:
#   1. drop .MIPS.abiflags (LLVM marks soft-float; binutils aborts the
#      merge against ee-gcc's hard-single-float GNU tag)
#   2. force e_flags (ELF32, offset 36) to 0x20920025, the value ee-gcc
#      emits for -march=r5900 -mabi=o32, so ISA/ABI class checks pass
# Codegen is untouched: our code is a valid mips2 subset of mips3, and the
# Rust/C boundary is integer-only, so the FP ABI flag is runtime-irrelevant
# (see examples/hello-rs/src/ffi.rs for the boundary contract).
set -eu

IN=$(readlink -f "$1")
OUT=$(readlink -f "$2")
TOOL=mips64r5900el-ps2-elf
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
cd "$TMP"

for o in $("$TOOL-ar" t "$IN"); do
    "$TOOL-ar" p "$IN" "$o" > "$o"
    "$TOOL-objcopy" --remove-section=.MIPS.abiflags "$o"
    printf '\045\000\222\040' | dd of="$o" bs=1 seek=36 conv=notrunc 2>/dev/null
done
"$TOOL-ar" rcs "$OUT" $("$TOOL-ar" t "$IN")
