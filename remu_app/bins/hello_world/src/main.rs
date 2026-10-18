// `hello_world` — one `main`, every platform.
//
// Output goes through the unified `remu_hal::println!`, which each platform
// arm implements internally (MMIO UART / std stdout / syscall fd 1) — no
// device handles or platform cfgs in app code. `init()` is the platform
// bring-up (bare-metal heap etc.; no-op elsewhere).

#[remu_hal::entry]
fn main() -> ! {
    remu_hal::init();
    remu_hal::println!("Hello World");
    remu_hal::println!("Answer: {}", 42);
    remu_hal::exit_success()
}
