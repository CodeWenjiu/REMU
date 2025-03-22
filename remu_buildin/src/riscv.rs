pub const IMG: [u32; 6] = [
    0x00000297,  // auipc t0,
    0x00028823,  // sb  zero,
    0x0102c503,  // lbu a0,16
    0x00100073,  // ebreak (u
    0xdeadbeef,  // some data
    0x5f5f5f5f,  // "____"
];

pub const RESET_VECTOR : u32 = 0x80000000;
