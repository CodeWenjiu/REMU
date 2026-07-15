/* remu default RAM: 0x8000_0000 .. 0x8800_0000 (128MB)
 * All sections in RAM (no flash).
 */
MEMORY
{
    RAM (rwx) : ORIGIN = 0x80000000, LENGTH = 128M
}

REGION_ALIAS("REGION_TEXT", RAM);
REGION_ALIAS("REGION_RODATA", RAM);
REGION_ALIAS("REGION_DATA", RAM);
REGION_ALIAS("REGION_BSS", RAM);
REGION_ALIAS("REGION_HEAP", RAM);
REGION_ALIAS("REGION_STACK", RAM);

/* Stack size per hart (single-hart). Must be set before _heap_size. */
PROVIDE(_hart_stack_size = 8K);

/* Heap uses all RAM between end of .uninit and (stack_start - stack_size). */
PROVIDE(_heap_size = (ORIGIN(REGION_STACK) + LENGTH(REGION_STACK) - _hart_stack_size) - __euninit);
