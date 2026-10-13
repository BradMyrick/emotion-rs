/* emotion-rt linkfile — EE user image, linked for the ps2dev-patched
 * binutils `ld` (also works with stock mips64el-none-elf-ld).
 *
 * Layout follows prussia's user-linkfile.ld (base 0x00100000: the EE
 * kernel reserves the first MiB) plus ps2sdk's _gp convention as
 * insurance against LLVM gp-relative addressing. */

ENTRY(_start);
EXTERN(_start);

SECTIONS
{
    .text 0x00100000 : {
        *(.text .text.*)
    }

    .rodata ALIGN(128) : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }

    _gp = ALIGN(128) + 0x7ff0;

    .data ALIGN(128) : {
        *(.sdata .sdata.*)
        *(.data .data.*)
    }

    .bss ALIGN(128) : {
        __bss_start = .;
        *(.sbss .sbss.*)
        *(.bss .bss.*)
        *(COMMON)
        . = ALIGN(16);
        __bss_end = .;
    }

    _end = .;

    /DISCARD/ : {
        *(.MIPS.abiflags)
        *(.pdr)
        *(.reginfo)
        *(.comment)
    }
}

ASSERT(_end < 0x00200000, "EE user image exceeds the 32MiB user area")
