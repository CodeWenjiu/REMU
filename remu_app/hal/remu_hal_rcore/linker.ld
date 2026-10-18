/* rcore user program linker script (ch2).
 *
 * Base address 0x8040_0000 follows the rCore convention: user programs are
 * loaded (here: pre-loaded by the simulator via `--app`) at this fixed
 * address, after the kernel at 0x8020_0000. `_start` is placed in `.text.entry`
 * so entry == base address.
 */

OUTPUT_ARCH(riscv)
ENTRY(_start)

BASE_ADDRESS = 0x80400000;

SECTIONS
{
    . = BASE_ADDRESS;

    .text : {
        *(.text.entry)
        *(.text .text.*)
    }
    . = ALIGN(4K);
    etext = .;

    .rodata : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }
    . = ALIGN(4K);
    erodata = .;

    .data : {
        *(.data .data.*)
        *(.sdata .sdata.*)
    }
    . = ALIGN(4K);
    edata = .;

    .bss : {
        sbss = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
    }
    . = ALIGN(4K);
    ebss = .;

    /DISCARD/ : {
        *(.eh_frame)
    }
}