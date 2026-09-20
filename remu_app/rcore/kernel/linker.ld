/* rcore kernel linker script (chapter 1).
 *
 * Base address 0x8020_0000 follows the QEMU `virt` convention: OpenSBI (or
 * remu's `remu_firmware`, loaded via `--firmware`) occupies
 * [0x8000_0000, 0x8020_0000) and the S-mode payload starts right after it.
 * Using the same base on remu (the firmware hands over with `mret`) keeps one
 * script for every platform.
 *
 * The workspace `.cargo/config.toml` injects `-Tlink.x` for all riscv-none
 * targets; this crate's build.rs places this file in OUT_DIR as `link.x`, so
 * the kernel replaces the riscv-rt one without touching shared config.
 */

OUTPUT_ARCH(riscv)
ENTRY(_start)

BASE_ADDRESS = 0x80200000;

SECTIONS
{
    . = BASE_ADDRESS;

    skernel = .;

    stext = .;
    .text : {
        *(.text.entry)
        *(.text .text.*)
    }
    . = ALIGN(4K);
    etext = .;

    srodata = .;
    .rodata : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }
    . = ALIGN(4K);
    erodata = .;

    sdata = .;
    .data : {
        *(.data .data.*)
        *(.sdata .sdata.*)
    }
    . = ALIGN(4K);
    edata = .;

    .bss : {
        *(.bss.stack)
        sbss = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
    }
    . = ALIGN(4K);
    ebss = .;

    ekernel = .;

    /DISCARD/ : {
        *(.eh_frame)
    }
}