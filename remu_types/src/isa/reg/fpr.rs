//! RISC-V 浮点寄存器及 ABI 名称（与 GPR 对应：ft0/fa0/fs0 等）  
//! 映射见 RISC-V ELF psABI：ft0–ft7, fs0–fs1, fa0–fa7, fs2–fs11, ft8–ft11

use strum::{Display, EnumString, FromRepr};

#[derive(Debug, PartialEq, Clone, Copy, EnumString, Display, FromRepr)]
#[strum(ascii_case_insensitive)]
pub enum Fpr {
    #[strum(to_string = "ft0", serialize = "f0")]
    Ft0 = 0,
    #[strum(to_string = "ft1", serialize = "f1")]
    Ft1 = 1,
    #[strum(to_string = "ft2", serialize = "f2")]
    Ft2 = 2,
    #[strum(to_string = "ft3", serialize = "f3")]
    Ft3 = 3,
    #[strum(to_string = "ft4", serialize = "f4")]
    Ft4 = 4,
    #[strum(to_string = "ft5", serialize = "f5")]
    Ft5 = 5,
    #[strum(to_string = "ft6", serialize = "f6")]
    Ft6 = 6,
    #[strum(to_string = "ft7", serialize = "f7")]
    Ft7 = 7,
    #[strum(to_string = "fs0", serialize = "f8")]
    Fs0 = 8,
    #[strum(to_string = "fs1", serialize = "f9")]
    Fs1 = 9,
    #[strum(to_string = "fa0", serialize = "f10")]
    Fa0 = 10,
    #[strum(to_string = "fa1", serialize = "f11")]
    Fa1 = 11,
    #[strum(to_string = "fa2", serialize = "f12")]
    Fa2 = 12,
    #[strum(to_string = "fa3", serialize = "f13")]
    Fa3 = 13,
    #[strum(to_string = "fa4", serialize = "f14")]
    Fa4 = 14,
    #[strum(to_string = "fa5", serialize = "f15")]
    Fa5 = 15,
    #[strum(to_string = "fa6", serialize = "f16")]
    Fa6 = 16,
    #[strum(to_string = "fa7", serialize = "f17")]
    Fa7 = 17,
    #[strum(to_string = "fs2", serialize = "f18")]
    Fs2 = 18,
    #[strum(to_string = "fs3", serialize = "f19")]
    Fs3 = 19,
    #[strum(to_string = "fs4", serialize = "f20")]
    Fs4 = 20,
    #[strum(to_string = "fs5", serialize = "f21")]
    Fs5 = 21,
    #[strum(to_string = "fs6", serialize = "f22")]
    Fs6 = 22,
    #[strum(to_string = "fs7", serialize = "f23")]
    Fs7 = 23,
    #[strum(to_string = "fs8", serialize = "f24")]
    Fs8 = 24,
    #[strum(to_string = "fs9", serialize = "f25")]
    Fs9 = 25,
    #[strum(to_string = "fs10", serialize = "f26")]
    Fs10 = 26,
    #[strum(to_string = "fs11", serialize = "f27")]
    Fs11 = 27,
    #[strum(to_string = "ft8", serialize = "f28")]
    Ft8 = 28,
    #[strum(to_string = "ft9", serialize = "f29")]
    Ft9 = 29,
    #[strum(to_string = "ft10", serialize = "f30")]
    Ft10 = 30,
    #[strum(to_string = "ft11", serialize = "f31")]
    Ft11 = 31,
}

impl Fpr {
    pub fn idx(&self) -> usize {
        *self as usize
    }
}
