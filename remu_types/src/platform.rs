#[repr(u8)]
#[derive(Clone, Copy)]
pub enum Platform {
    None = 0,
    Remu = 1,
    Unicorn = 2,
    Spike = 3,
}
