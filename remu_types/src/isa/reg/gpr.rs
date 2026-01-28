use strum::{Display, EnumString, FromRepr};

#[derive(Debug, PartialEq, Clone, Copy, EnumString, Display, FromRepr)]
#[strum(ascii_case_insensitive)]
pub enum Gpr {
    #[strum(to_string = "r0", serialize = "zero", serialize = "x0")]
    Zero = 0,
    #[strum(to_string = "ra", serialize = "x1")]
    Ra = 1,
    #[strum(to_string = "sp", serialize = "x2")]
    Sp = 2,
    #[strum(to_string = "gp", serialize = "x3")]
    Gp = 3,
    #[strum(to_string = "tp", serialize = "x4")]
    Tp = 4,
    #[strum(to_string = "t0", serialize = "x5")]
    T0 = 5,
    #[strum(to_string = "t1", serialize = "x6")]
    T1 = 6,
    #[strum(to_string = "t2", serialize = "x7")]
    T2 = 7,
    #[strum(to_string = "s0", serialize = "fp", serialize = "x8")]
    S0 = 8,
    #[strum(to_string = "s1", serialize = "x9")]
    S1 = 9,
    #[strum(to_string = "a0", serialize = "x10")]
    A0 = 10,
    #[strum(to_string = "a1", serialize = "x11")]
    A1 = 11,
    #[strum(to_string = "a2", serialize = "x12")]
    A2 = 12,
    #[strum(to_string = "a3", serialize = "x13")]
    A3 = 13,
    #[strum(to_string = "a4", serialize = "x14")]
    A4 = 14,
    #[strum(to_string = "a5", serialize = "x15")]
    A5 = 15,
    #[strum(to_string = "a6", serialize = "x16")]
    A6 = 16,
    #[strum(to_string = "a7", serialize = "x17")]
    A7 = 17,
    #[strum(to_string = "s2", serialize = "x18")]
    S2 = 18,
    #[strum(to_string = "s3", serialize = "x19")]
    S3 = 19,
    #[strum(to_string = "s4", serialize = "x20")]
    S4 = 20,
    #[strum(to_string = "s5", serialize = "x21")]
    S5 = 21,
    #[strum(to_string = "s6", serialize = "x22")]
    S6 = 22,
    #[strum(to_string = "s7", serialize = "x23")]
    S7 = 23,
    #[strum(to_string = "s8", serialize = "x24")]
    S8 = 24,
    #[strum(to_string = "s9", serialize = "x25")]
    S9 = 25,
    #[strum(to_string = "s10", serialize = "x26")]
    S10 = 26,
    #[strum(to_string = "s11", serialize = "x27")]
    S11 = 27,
    #[strum(to_string = "t3", serialize = "x28")]
    T3 = 28,
    #[strum(to_string = "t4", serialize = "x29")]
    T4 = 29,
    #[strum(to_string = "t5", serialize = "x30")]
    T5 = 30,
    #[strum(to_string = "t6", serialize = "x31")]
    T6 = 31,
}

impl Gpr {
    pub fn idx(&self) -> usize {
        *self as usize
    }
}
