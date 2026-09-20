/* remu firmware linker script.
 *
 * Base address 0x8000_0000 follows the QEMU `virt` convention: the firmware
 * (OpenSBI on QEMU) occupies [0x8000_0000, 0x8020_0000) and the S-mode payload
 * sits right after it. remu loads this ELF via `--firmware`, whose entry
 * becomes the reset PC, so no `--init-pc` needs to be configured.
 *
 * The workspace `.cargo/config.toml` injects `-Tlink.x` for all riscv-none
 * targets; this crate's build.rs places this file in OUT_DIR as `link.x`, so
 * the firmware replaces the riscv-rt one without touching shared config.
 */

OUTPUT_ARCH(riscv)
ENTRY(_start)

BASE_ADDRESS = 0x80000000;

SECTIONS
{
    . = BASE_ADDRESS;

    sfi = .;

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

    efi = .;

    /DISCARD/ : {
        *(.eh_frame)
    }
}