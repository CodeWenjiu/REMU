//! Tests heap allocator and collection types (Vec, String, Box, etc.)

#![no_std]
#![no_main]

use remu_hal::{entry, init, FmtWrite, Uart16550, Vec, String, Box, exit_success};

#[entry]
fn main() -> ! {
    unsafe { init() };

    let mut uart = Uart16550::default_base();
    let _ = writeln!(uart, "collection test");

    // Vec
    let mut v: Vec<u32> = Vec::new();
    v.push(1);
    v.push(2);
    v.push(3);
    let _ = writeln!(uart, "Vec sum: {}", v.iter().sum::<u32>());
    v.extend([4, 5]);
    let _ = writeln!(uart, "Vec len: {}", v.len());

    // String
    let s: String = String::from("hello");
    let _ = writeln!(uart, "String: {}", s);

    // Box
    let b: Box<u32> = Box::new(42);
    let _ = writeln!(uart, "Box: {}", *b);

    let _ = remu_hal::Write::flush(&mut uart);
    exit_success()
}
