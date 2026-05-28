//! ISA type definitions — single-file table + generation.
//!
//! Each `pub struct` is a zero-sized marker implementing [`RvIsa`](crate::isa::RvIsa).
//! To add an ISA variant, add a row to `for_each_isa!` below.

#![allow(non_camel_case_types)]

// ── Table ──
// Row: (Name, XLEN, has_M, has_F, VConfig, has_WJ, MISA, ISA_str, base, ext, platforms)
// platforms: RN (both), R (remu only), N (nzea only)

#[macro_export]
macro_rules! for_each_isa {
    ($cb:ident) => {
        $cb!(RV32I,               u32, -, -, $crate::isa::extension_v::NoV,          -, 0x4000_0100, "rv32i",              i,  none, RN);
        $cb!(RV32IM,              u32, +, -, $crate::isa::extension_v::NoV,          -, 0x4000_1100, "rv32im",             im, none, RN);
        $cb!(RV32I_wjCus0,        u32, -, -, $crate::isa::extension_v::NoV,          +, 0x4000_0100, "riscv32i_wjCus0",    i,  wj,   RN);
        $cb!(RV32IM_wjCus0,       u32, +, -, $crate::isa::extension_v::NoV,          +, 0x4000_1100, "riscv32im_wjCus0",   im, wj,   RN);
        $cb!(RV32I_zve32x_zvl128b,u32, -, -, $crate::isa::extension_v::Zve32xZvl128b,-, 0x4000_0100, "rv32i_zve32x_zvl128b", i,  zve,  R);
        $cb!(RV32IM_zve32x_zvl128b,u32,+, -, $crate::isa::extension_v::Zve32xZvl128b,-, 0x4000_1100, "rv32im_zve32x_zvl128b",im, zve,  R);
    };
}

// ── Generator ──
// 8 arms for (has_M, has_F, has_WJ) ∈ {+,-}³. $p is platforms (ignored).

macro_rules! gen_isa_type {
    ($N:ident, $X:ty, -, -, $V:ty, -, $M:expr, $S:literal, $b:tt, $e:tt, $p:tt) => {
        #[derive(Clone, Copy)]
        pub struct $N;
        impl $crate::isa::RvIsa for $N {
            type XLEN = $X;
            type PcState = $crate::isa::reg::PcState;
            type GprState = $crate::isa::reg::GprState;
            type FprState = ();
            type VConfig = $V;
            const ISA_STR: &str = $S;
            const MISA: u32 = $M;
        }
    };
    ($N:ident, $X:ty, -, -, $V:ty, +, $M:expr, $S:literal, $b:tt, $e:tt, $p:tt) => {
        #[derive(Clone, Copy)]
        pub struct $N;
        impl $crate::isa::RvIsa for $N {
            type XLEN = $X;
            type PcState = $crate::isa::reg::PcState;
            type GprState = $crate::isa::reg::GprState;
            type FprState = ();
            type VConfig = $V;
            const ISA_STR: &str = $S;
            const MISA: u32 = $M;
            const HAS_WJ_CUS0: bool = true;
        }
    };
    ($N:ident, $X:ty, +, -, $V:ty, -, $M:expr, $S:literal, $b:tt, $e:tt, $p:tt) => {
        #[derive(Clone, Copy)]
        pub struct $N;
        impl $crate::isa::RvIsa for $N {
            type XLEN = $X;
            type PcState = $crate::isa::reg::PcState;
            type GprState = $crate::isa::reg::GprState;
            type FprState = ();
            type VConfig = $V;
            const ISA_STR: &str = $S;
            const MISA: u32 = $M;
            const HAS_M: bool = true;
        }
    };
    ($N:ident, $X:ty, +, -, $V:ty, +, $M:expr, $S:literal, $b:tt, $e:tt, $p:tt) => {
        #[derive(Clone, Copy)]
        pub struct $N;
        impl $crate::isa::RvIsa for $N {
            type XLEN = $X;
            type PcState = $crate::isa::reg::PcState;
            type GprState = $crate::isa::reg::GprState;
            type FprState = ();
            type VConfig = $V;
            const ISA_STR: &str = $S;
            const MISA: u32 = $M;
            const HAS_M: bool = true;
            const HAS_WJ_CUS0: bool = true;
        }
    };
    ($N:ident, $X:ty, -, +, $V:ty, -, $M:expr, $S:literal, $b:tt, $e:tt, $p:tt) => {
        #[derive(Clone, Copy)]
        pub struct $N;
        impl $crate::isa::RvIsa for $N {
            type XLEN = $X;
            type PcState = $crate::isa::reg::PcState;
            type GprState = $crate::isa::reg::GprState;
            type FprState = $crate::isa::reg::FprRegs;
            type VConfig = $V;
            const ISA_STR: &str = $S;
            const MISA: u32 = $M;
            const HAS_F: bool = true;
        }
    };
    ($N:ident, $X:ty, -, +, $V:ty, +, $M:expr, $S:literal, $b:tt, $e:tt, $p:tt) => {
        #[derive(Clone, Copy)]
        pub struct $N;
        impl $crate::isa::RvIsa for $N {
            type XLEN = $X;
            type PcState = $crate::isa::reg::PcState;
            type GprState = $crate::isa::reg::GprState;
            type FprState = $crate::isa::reg::FprRegs;
            type VConfig = $V;
            const ISA_STR: &str = $S;
            const MISA: u32 = $M;
            const HAS_F: bool = true;
            const HAS_WJ_CUS0: bool = true;
        }
    };
    ($N:ident, $X:ty, +, +, $V:ty, -, $M:expr, $S:literal, $b:tt, $e:tt, $p:tt) => {
        #[derive(Clone, Copy)]
        pub struct $N;
        impl $crate::isa::RvIsa for $N {
            type XLEN = $X;
            type PcState = $crate::isa::reg::PcState;
            type GprState = $crate::isa::reg::GprState;
            type FprState = $crate::isa::reg::FprRegs;
            type VConfig = $V;
            const ISA_STR: &str = $S;
            const MISA: u32 = $M;
            const HAS_M: bool = true;
            const HAS_F: bool = true;
        }
    };
    ($N:ident, $X:ty, +, +, $V:ty, +, $M:expr, $S:literal, $b:tt, $e:tt, $p:tt) => {
        #[derive(Clone, Copy)]
        pub struct $N;
        impl $crate::isa::RvIsa for $N {
            type XLEN = $X;
            type PcState = $crate::isa::reg::PcState;
            type GprState = $crate::isa::reg::GprState;
            type FprState = $crate::isa::reg::FprRegs;
            type VConfig = $V;
            const ISA_STR: &str = $S;
            const MISA: u32 = $M;
            const HAS_M: bool = true;
            const HAS_F: bool = true;
            const HAS_WJ_CUS0: bool = true;
        }
    };
}

for_each_isa!(gen_isa_type);
