use remu_hal::{Box, String, Vec, exit_success};

#[remu_hal::entry]
fn main() -> ! {
    remu_hal::init();
    remu_hal::println!("collection test");

    let mut v: Vec<u32> = Vec::new();
    v.push(1);
    v.push(2);
    v.push(3);
    remu_hal::println!("Vec sum: {}", v.iter().sum::<u32>());
    v.extend([4, 5]);
    remu_hal::println!("Vec len: {}", v.len());

    let s: String = String::from("hello");
    remu_hal::println!("String: {}", s);

    let b: Box<u32> = Box::new(42);
    remu_hal::println!("Box: {}", *b);

    exit_success()
}
