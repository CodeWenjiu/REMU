pub const IMG: &[u32] = &[
    0x00000297, // auipc t0,
    0x00028823, // sb  zero,
    0x0102c503, // lbu a0,16
    0x00100073, // ebreak (u

    0x0000101b, // slliw x0, x0, 0 (RV64I)
    0x0000005b, // addid x0, x0, 0 (RV128I unavailable now)
    0x0000100f, // fence.i
    0xc0003073, // csrrc x0, cycle, x0

    0x02000033, // mul x0, x0, x0 (+m)
    0x1000202f, // lr.w x0, (x0) (+a)
    0x00002007, // flw f0, 0(x0) (+f)
    0x00003007, // fld f0, 0(x0) (+d)

    0x00004007, // flq f0, 0(x0) (+q unavailable now)
    0x30200073, // mret

    0xdeadbeef, // some data
    0x5f5f5f5f, // "____"
];

pub const RESET_VECTOR : u32 = 0x80000000;
