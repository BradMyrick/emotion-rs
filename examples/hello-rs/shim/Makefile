EE_BIN  = hello-rs.elf
EE_OBJS = main.o
RUST_A   = rust/target/mipsel-sony-ps2/release/libhello_rs.a
RUST_ABI = rust/libhello_rs-ps2abi.a
# The patched Rust staticlib must precede the C libs so its bundled core /
# compiler_builtins resolve the soft-float libcalls (__mulsf3 etc.).
EE_LIBS = $(RUST_ABI) -lgskit -ldmakit -lpad

EE_INCS += -I/usr/local/ps2dev/gsKit/include
EE_LDFLAGS += -L/usr/local/ps2dev/gsKit/lib

RUST_ARGS = --release -Zbuild-std=core,compiler_builtins -Zjson-target-spec \
            --target targets/mipsel-sony-ps2.json

all: iso

.PHONY: rust
rust:
	cd rust && CARGO_HOME=$$(pwd)/.cargo-home cargo build $(RUST_ARGS)

# rewrite LLVM ELF flags so ps2dev binutils links the archive (see script)
$(RUST_ABI): $(RUST_A) patch-abi.sh
	sh patch-abi.sh $(RUST_A) $(abspath $(RUST_ABI))

# extra prerequisites for the link rule from Makefile.eeglobal
$(EE_BIN): rust $(RUST_ABI)

iso: $(EE_BIN)
	rm -rf cdroot && mkdir -p cdroot
	cp $(EE_BIN) cdroot/SLUS_900.01
	printf "BOOT2 = cdrom0:\\\\SLUS_900.01;1\nVER = 1.00\nVMODE = NTSC\n" > cdroot/SYSTEM.CNF
	xorriso -as mkisofs -l -r -V HELLO_RS -o game.iso cdroot/

include $(PS2SDK)/samples/Makefile.pref
include $(PS2SDK)/samples/Makefile.eeglobal

# NOTE: deliberately does not touch rust/target or rust/.cargo-home —
# the build-std cache keeps the edit-build-boot loop fast.
clean:
	rm -f $(EE_BIN) $(EE_OBJS) game.iso
	rm -rf cdroot
