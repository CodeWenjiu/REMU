use remu_hal::{exit_success, FmtWrite, Uart16550};

#[remu_hal::entry]
fn main() -> ! {
    remu_hal::init();
    let mut uart = Uart16550::default_base();
    let _ = writeln!(uart, "riscv_only: only RISC-V targets are supported");

    exit_success()
}
