/* HTIF mailboxes for Spike. Must be linked after link.x / memory.x.
   Spike polls writes to `tohost` for program exit.
   The `NOLOAD` flag tells the linker this section occupies no space in the ELF file. */
SECTIONS
{
  .htif (NOLOAD) :
  {
    . = ALIGN(8);
    tohost = .;
    QUAD(0);
    fromhost = .;
    QUAD(0);
  } > RAM
}
INSERT AFTER .bss;
