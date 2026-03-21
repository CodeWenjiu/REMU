#![no_std]
#![no_main]

use remu_hal::{entry, Uart16550, Write, exit_success};

#[entry]
fn main() -> ! {
    let mut uart = Uart16550::default_base();
    let _ = uart.write(b"Hello World\n");
    let _ = uart.flush();
    exit_success()
}
