//! rom0 kernel syscall wrappers.
//!
//! The EE rom0 kernel takes the function number in `$v1` and arguments
//! in `$a0..$a3` (`start.S` owns the trampolines). Integer-only, at most
//! 4 register args, and only valid while the rom0 kernel is resident —
//! i.e. after a BIOS BOOT2/EELOAD entry, which is the only supported
//! entry path.

/// Interlacing mode argument to [`set_gs_crt`].
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interlace {
    Noninterlaced = 0,
    Interlaced = 1,
}

/// GS video output mode argument to [`set_gs_crt`].
///
/// Values follow gsKit (`GS_MODE_NTSC == 0x02`); prussia's enum inverts
/// them — empirically disproven on PCSX2 2.8.2 (vmode=3 boots as PAL).
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoMode {
    Ntsc = 2,
    Pal = 3,
}

/// Field/frame readout mode argument to [`set_gs_crt`].
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldFrame {
    Frame = 0,
    Field = 1,
}

/// Raw `extern "C"` bindings to the `start.S` trampolines.
///
/// Not part of the stable surface: use the safe wrappers below.
#[cfg(not(test))]
pub(crate) mod syscall_asm {
    extern "C" {
        pub(crate) fn __rt_set_gs_crt(imode: u32, vmode: u32, ffmode: u32);
        pub(crate) fn __rt_exit(status: u32) -> !;
        pub(crate) fn __rt_read_count() -> u32;
        pub(crate) fn __rt_setup(slot: u32, function: u32);
        pub(crate) fn __rt_copy(dest: u32, src: u32, len: u32) -> u32;
        pub(crate) fn __rt_kcopy_bytes() -> u32;
        pub(crate) fn __rt_flush_cache(op: u32);
        pub(crate) fn __rt_load_exec_ps2(filename: u32, num_args: u32, args: u32) -> !;
    }
}

/// Read the cop0 `Count` timer (ticks at EE clock / 2 = 147.456 MHz,
/// wraps every ~29s). Use wrapping arithmetic on differences.
pub fn read_count() -> u32 {
    #[cfg(not(test))]
    {
        // SAFETY: register read with no arguments and no side effects.
        unsafe { syscall_asm::__rt_read_count() }
    }
    #[cfg(test)]
    {
        0
    }
}

/// Configure the GS display controller (rom0 `SetGsCrt`, syscall 0x2).
pub fn set_gs_crt(interlace: Interlace, mode: VideoMode, field_frame: FieldFrame) {
    #[cfg(not(test))]
    {
        // SAFETY: integer-only args, 3 register args, trampoline in
        // start.S follows the o32 register convention exactly.
        unsafe {
            syscall_asm::__rt_set_gs_crt(interlace as u32, mode as u32, field_frame as u32)
        }
    }
    #[cfg(test)]
    {
        let _ = (interlace, mode, field_frame);
    }
}

/// Exit the program and return to the PS2 browser (rom0 `KExit`,
/// syscall 0x4). Never returns.
///
/// **BOOT2-booted programs only**: KExit returns to the browser only
/// when the kernel knows our parent (disc boot via EELOAD). From a
/// direct ELF launch (PCSX2 "Run ELF" / ExecPS2) there is no parent and
/// KExit just kills the thread. Use [`exit_to_browser`] for the
/// universal path.
pub fn exit(status: u32) -> ! {
    #[cfg(not(test))]
    {
        // SAFETY: one integer register arg; the trampoline never
        // returns and neither do we.
        unsafe { syscall_asm::__rt_exit(status) }
    }
    #[cfg(test)]
    {
        let _ = status;
        unimplemented!("rom0 syscalls are unavailable on the host")
    }
}

/// Kernel RAM scratch used for the exec-args block (the same free area
/// ps2sdk's tlbsrc payload occupies; contents survive EE reboots).
#[cfg(not(test))]
const KEXEC_BLOCK: u32 = 0x8007_5000;
/// Strings live at `_kExecArg + 16` pointers, per ps2sdk's `SetArg`.
#[cfg(not(test))]
const KEXEC_STR: u32 = KEXEC_BLOCK + 64;
#[cfg(not(test))]
const OSDSYS: &[u8] = b"rom0:OSDSYS\0";

/// Reboot the EE into `rom0:OSDSYS` (the PS2 browser). Never returns.
///
/// Universal exit: works from both BOOT2 disc boots and direct ELF
/// launches. Replicates ps2sdk's `LoadExecPS2("rom0:OSDSYS", 0, NULL)`
/// path — install a byte-copier at syscall slot 0x5A, stage the
/// filename and argv pointer in kernel-resident memory, write back the
/// data cache, then invoke rom0 `_LoadExecPS2` (syscall 0x6).
pub fn exit_to_browser() -> ! {
    #[cfg(not(test))]
    {
        // SAFETY: the sequence mirrors ps2sdk's proven SetArg +
        // LoadExecPS2 flow: 0x80075000 is free kernel scratch on every
        // retail BIOS, all arguments are plain integers/pointers into
        // static or kernel memory, and the final syscall never returns.
        unsafe {
            let kcopy: unsafe extern "C" fn() -> u32 = syscall_asm::__rt_kcopy_bytes;
            syscall_asm::__rt_setup(0x5A, kcopy as usize as u32);
            syscall_asm::__rt_copy(KEXEC_STR, OSDSYS.as_ptr() as u32, OSDSYS.len() as u32);
            let str_ptr: u32 = KEXEC_STR;
            syscall_asm::__rt_copy(KEXEC_BLOCK, &str_ptr as *const u32 as u32, 4);
            syscall_asm::__rt_flush_cache(0);
            syscall_asm::__rt_load_exec_ps2(KEXEC_STR, 0, KEXEC_BLOCK + 4)
        }
    }
    #[cfg(test)]
    {
        unimplemented!("rom0 syscalls are unavailable on the host")
    }
}
