/* Minimal HTIF mailbox symbol for Spike.
   Linked by all RISC-V targets — just a harmless RAM address.
   The full .htif section layout (with fromhost, alignment, etc.)
   is in link/htif.x, linked only by the spike platform. */
PROVIDE(tohost = 0x87FFFEE0);
