# emotion-rt EE entry + rom0 kernel syscall trampolines.
#
# ABI contract (mirrors src/syscall.rs): rom0 kernel syscalls take the
# function number in $v1 and arguments in $a0-$a3 — integer-only, at most
# 4 register args (the boundary rules proven in Phase 1, now Rust->rom0).
#
# Entry state assumption: reached via BIOS BOOT2/EELOAD (kernel resident,
# usable memory 0x00100000..0x02000000).
#
# _start mirrors ps2sdk's crt0 boot sequence (proven to make KExit return
# to the browser): zero .bss, register the thread with the kernel via
# SetupThread (syscall 60), switch to the kernel-provided stack, enable
# interrupts, enter Rust. Skipping SetupThread leaves the program
# unregistered and KExit (syscall 4) then just kills the thread — a black
# unregistered and KExit (syscall 4) then just kills the thread — a black
# screen (verified the hard way on PCSX2 2.8.2).
#
# The .set push/mips3/pop around the 64-bit helpers is load-bearing:
# under -32 the ps2dev assembler macro-expands ld/sd into lw/sw pairs
# (GS registers only answer full 64-bit accesses), while a file-wide
# .set mips3 would drop r5900-only instructions (sq, ei) elsewhere.
.set noreorder

.section .text._start, "ax"
.globl _start
.extern __rust_start
.extern __rt_boot_args
.extern __rt_exit_thread
_start:
    # zero .bss with 16-byte stores (bounds are 16-aligned by the
    # linkfile). No stack use until the kernel hands us one.
    .set noat
    la  $2, __bss_start
    la  $3, __bss_end
1:  sltu $1, $2, $3
    beq  $1, $0, 2f
    nop
    sq   $0, ($2)
    addiu $2, $2, 16
    j    1b
    nop
2:
    .set at

    # SetupThread (syscall 60): a0=_gp, a1=_stack (-1: kernel picks),
    # a2=_stack_size, a3=&boot args (zeroed above), t0=ExitThread fn.
    # Returns the new stack pointer in v0.
    la   $4, _gp
    li   $5, -1
    lui  $6, 0x2             # 128 KiB stack
    la   $7, __rt_boot_args
    la   $8, __rt_exit_thread
    move $gp, $4
    ori  $3, $0, 60
    syscall
    move $sp, $2

    # enter Rust with interrupts enabled (as ps2sdk's _main does);
    # ei takes effect after the following instruction.
    ei
    j    __rust_start
    nop

.section .bss
.align 4
.globl __rt_boot_args
__rt_boot_args:
    .space 8                 # argc = 0, argv = NULL

# void __rt_exit(void status) -> !  (rom0 KExit, syscall 0x4)
# Valid only because _start ran SetupThread first (see above).
.section .text.rt_exit, "ax"
.globl __rt_exit
__rt_exit:
    ori $v1, $0, 0x4
    syscall
1:  b   1b                   # KExit never returns; trap defensively
    nop

# void __rt_exit_thread()  (ExitThread, syscall 0x23; passed to SetupThread)
.section .text.rt_exit_thread, "ax"
.globl __rt_exit_thread
__rt_exit_thread:
    ori $v1, $0, 0x23
    syscall
    jr  $ra
    nop

# void __rt_set_gs_crt(u32 imode, u32 vmode, u32 ffmode)  (syscall 0x2)
.section .text.rt_set_gs_crt, "ax"
.globl __rt_set_gs_crt
__rt_set_gs_crt:
    ori $v1, $0, 0x2
    syscall
    jr  $ra
    nop

# u32 __rt_read_count()  (cop0 Count, ticks at EEclk/2 = 147.456 MHz)
.section .text.rt_read_count, "ax"
.globl __rt_read_count
__rt_read_count:
    jr  $ra
    mfc0 $v0, $9

# u32 __rt_copy(dest a0, src a1, len a2) — run the copier installed at
# syscall slot 0x5A (kernel-context copy; used for the exec-args block).
.section .text.rt_copy, "ax"
.globl __rt_copy
__rt_copy:
    ori $v1, $0, 0x5A
    syscall
    jr  $ra
    nop

# void __rt_setup(slot a0, fn a1) — install a function into a syscall
# table slot (rom0 "setup"/SetSyscall, syscall 0x74).
.section .text.rt_setup, "ax"
.globl __rt_setup
__rt_setup:
    ori $v1, $0, 0x74
    syscall
    jr  $ra
    nop

# void __rt_flush_cache(op a0) — FlushCache (syscall 0x64).
.section .text.rt_flush_cache, "ax"
.globl __rt_flush_cache
__rt_flush_cache:
    ori $v1, $0, 0x64
    syscall
    jr  $ra
    nop

# int __rt_kcopy_bytes(dest a0, src a1, len a2) — plain byte copier,
# installed at slot 0x5A before use (ps2sdk's kCopyBytes equivalent).
.section .text.rt_kcopy_bytes, "ax"
.globl __rt_kcopy_bytes
__rt_kcopy_bytes:
    beq  $a2, $0, 2f
    nop
1:  lbu  $t0, 0($a1)
    sb   $t0, 0($a0)
    addiu $a0, $a0, 1
    addiu $a1, $a1, 1
    addiu $a2, $a2, -1
    bne  $a2, $0, 1b
    nop
2:  jr   $ra
    move $v0, $0

# void __rt_load_exec_ps2(filename a0, num_args a1, args a2) -> !
# (rom0 _LoadExecPS2, syscall 0x6 — reboots the EE into `filename`)
.section .text.rt_load_exec_ps2, "ax"
.globl __rt_load_exec_ps2
__rt_load_exec_ps2:
    ori $v1, $0, 0x6
    syscall
1:  b   1b                   # never returns; trap defensively
    nop

# u64 __rt_ld64(u32 addr) — single 64-bit device load. The Rust side
# must NOT let LLVM split GS-register accesses: mips2 codegen emits
# lw-pairs for u64 volatiles, and GS registers only respond correctly
# to full 64-bit ld/sd (prussia's _read_u64/_write_u64 exist for this).
# Result: v0 = low word, v1 = high word (o32 64-bit return).
.section .text.rt_ld64, "ax"
.globl __rt_ld64
__rt_ld64:
    .set push
    .set mips3
    ld  $2, 0($4)
    .set pop
    jr  $ra
    nop

# void __rt_sd64(u32 addr, u32 lo, u32 hi) — single 64-bit device store.
.section .text.rt_sd64, "ax"
.globl __rt_sd64
__rt_sd64:
    dsll32 $6, $6, 0        # $6 = hi << 32
    or     $6, $6, $5       # $6 = (hi << 32) | lo
    .set push
    .set mips3
    sd     $6, 0($4)
    .set pop
    jr     $ra
    nop
