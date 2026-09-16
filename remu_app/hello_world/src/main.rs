use remu_hal::{FmtWrite, Uart16550, exit_success};

#[remu_hal::entry]
fn main() -> ! {
    remu_hal::init();
    let mut uart = Uart16550::default_base();
    let _ = writeln!(uart, "Hello World");
    let _ = writeln!(uart, "Answer: {}", 42);

    exit_success()
}
