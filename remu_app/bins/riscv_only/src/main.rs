use remu_hal::exit_success;

#[remu_hal::entry]
fn main() -> ! {
    remu_hal::init();
    remu_hal::println!("riscv_only: only RISC-V targets are supported");

    exit_success()
}
