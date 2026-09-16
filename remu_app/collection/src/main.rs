use remu_hal::{Box, FmtWrite, String, Uart16550, Vec, exit_success};

#[remu_hal::entry]
fn main() -> ! {
    remu_hal::init();
    let mut uart = Uart16550::default_base();
    let _ = writeln!(uart, "collection test");

    let mut v: Vec<u32> = Vec::new();
    v.push(1);
    v.push(2);
    v.push(3);
    let _ = writeln!(uart, "Vec sum: {}", v.iter().sum::<u32>());
    v.extend([4, 5]);
    let _ = writeln!(uart, "Vec len: {}", v.len());

    let s: String = String::from("hello");
    let _ = writeln!(uart, "String: {}", s);

    let b: Box<u32> = Box::new(42);
    let _ = writeln!(uart, "Box: {}", *b);

    exit_success()
}
