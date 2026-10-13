use std::{env, fs, path::PathBuf, process::Command};

// Builds in two modes:
//  * TARGET=mipsel-sony-ps2 with the ps2dev assembler on PATH (build
//    container): assemble start.S and expose it + the linker script to
//    dependent binaries.
//  * anything else (host cargo test / clippy, NUC cross `cargo check`
//    without the toolchain): only stage the linker script; skip the asm
//    so link-less builds stay green.
fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let target = env::var("TARGET").unwrap();

    fs::copy(manifest.join("user-linkfile.ld"), out_dir.join("linkfile.ld"))
        .expect("copy user-linkfile.ld");
    println!("cargo:rerun-if-changed={}/start.S", manifest.display());
    println!("cargo:rerun-if-changed={}/user-linkfile.ld", manifest.display());

    if target != "mipsel-sony-ps2" {
        return;
    }

    // Only for the PS2 target: a host link-arg would apply the EE linker
    // script to host test binaries and crash them.
    println!("cargo:rustc-link-arg=-T{}/linkfile.ld", out_dir.display());

    let as_ok = Command::new("mips64r5900el-ps2-elf-as")
        .arg("-march=r5900")
        .arg("-32")
        .arg("-o")
        .arg(out_dir.join("start.o"))
        .arg(manifest.join("start.S"))
        .status();
    match as_ok {
        Ok(st) if st.success() => {}
        Ok(st) => panic!("start.S assembly failed: {st}"),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            eprintln!(
                "warning: mips64r5900el-ps2-elf-as not found — start.S skipped \
                 (fine for check/clippy; a real link needs the build container)"
            );
            return;
        }
        Err(e) => panic!("failed to run assembler: {e}"),
    }

    let ar_ok = Command::new("mips64r5900el-ps2-elf-ar")
        .current_dir(&out_dir)
        .args(["crs", "libemotion-rt-start.a", "start.o"])
        .status();
    match ar_ok {
        Ok(st) if st.success() => {}
        st => panic!("archiving start.o failed: {st:?}"),
    }
    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=emotion-rt-start");
}
