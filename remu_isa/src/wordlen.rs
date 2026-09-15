use std::{
    fmt::{Debug, LowerHex},
    ops::{Add, BitAnd, BitOr, BitXor, Not, Shl, Shr, Sub},
};

use crate::AllUsize;

pub trait MachineWord:
    Copy
    + Clone
    + Debug
    + LowerHex
    + Default
    + PartialEq
    + PartialOrd
    + Add<Output = Self>
    + Sub<Output = Self>
    + BitAnd<Output = Self>
    + BitOr<Output = Self>
    + BitXor<Output = Self>
    + Shl<u32, Output = Self>
    + Shr<u32, Output = Self>
    + Not<Output = Self>
    + From<u8>
{
}

/// Fold a machine word into an [`AllUsize`] for display / difftest.
pub trait IntoAllUsize: Copy {
    fn into_all(self) -> AllUsize;
}
impl IntoAllUsize for u32 {
    #[inline(always)]
    fn into_all(self) -> AllUsize {
        AllUsize::U32(self)
    }
}
impl IntoAllUsize for u64 {
    #[inline(always)]
    fn into_all(self) -> AllUsize {
        AllUsize::U64(self)
    }
}
impl IntoAllUsize for u128 {
    #[inline(always)]
    fn into_all(self) -> AllUsize {
        AllUsize::U128(self)
    }
}
impl IntoAllUsize for i32 {
    #[inline(always)]
    fn into_all(self) -> AllUsize {
        AllUsize::U32(self as u32)
    }
}
impl IntoAllUsize for i64 {
    #[inline(always)]
    fn into_all(self) -> AllUsize {
        AllUsize::U64(self as u64)
    }
}
impl IntoAllUsize for i128 {
    #[inline(always)]
    fn into_all(self) -> AllUsize {
        AllUsize::U128(self as u128)
    }
}

/// Wrap-around arithmetic + width conversions needed by the executors.
/// Implemented for the primitive unsigned types via a single macro so every
/// call is `#[inline(always)]` (zero-cost on the hot path).
pub trait WordOps: MachineWord {
    fn wrapping_add(self, rhs: Self) -> Self;
    fn wrapping_sub(self, rhs: Self) -> Self;
    fn wrapping_mul(self, rhs: Self) -> Self;
    fn wrapping_div(self, rhs: Self) -> Self;
    fn wrapping_rem(self, rhs: Self) -> Self;
    fn wrapping_shl(self, shamt: u32) -> Self;
    fn wrapping_shr(self, shamt: u32) -> Self;

    fn to_u8(self) -> u8;
    fn to_u16(self) -> u16;
    fn to_u32(self) -> u32;
    fn to_u64(self) -> u64;
    fn to_usize(self) -> usize;
    fn to_u128(self) -> u128;
    fn to_i128(self) -> i128;
}

macro_rules! impl_word_ops {
    ($T:ty) => {
        impl WordOps for $T {
            #[inline(always)]
            fn wrapping_add(self, rhs: Self) -> Self {
                self.wrapping_add(rhs)
            }
            #[inline(always)]
            fn wrapping_sub(self, rhs: Self) -> Self {
                self.wrapping_sub(rhs)
            }
            #[inline(always)]
            fn wrapping_mul(self, rhs: Self) -> Self {
                self.wrapping_mul(rhs)
            }
            #[inline(always)]
            fn wrapping_div(self, rhs: Self) -> Self {
                self.wrapping_div(rhs)
            }
            #[inline(always)]
            fn wrapping_rem(self, rhs: Self) -> Self {
                self.wrapping_rem(rhs)
            }
            #[inline(always)]
            fn wrapping_shl(self, shamt: u32) -> Self {
                self.wrapping_shl(shamt)
            }
            #[inline(always)]
            fn wrapping_shr(self, shamt: u32) -> Self {
                self.wrapping_shr(shamt)
            }
            #[inline(always)]
            fn to_u8(self) -> u8 {
                self as u8
            }
            #[inline(always)]
            fn to_u16(self) -> u16 {
                self as u16
            }
            #[inline(always)]
            fn to_u32(self) -> u32 {
                self as u32
            }
            #[inline(always)]
            fn to_u64(self) -> u64 {
                self as u64
            }
            #[inline(always)]
            fn to_usize(self) -> usize {
                self as usize
            }
            #[inline(always)]
            fn to_u128(self) -> u128 {
                self as u128
            }
            #[inline(always)]
            fn to_i128(self) -> i128 {
                self as i128
            }
        }
    };
}

impl_word_ops!(u32);
impl_word_ops!(u64);
impl_word_ops!(u128);
impl_word_ops!(i32);
impl_word_ops!(i64);
impl_word_ops!(i128);

pub trait Xlen: MachineWord + IntoAllUsize + WordOps {
    type Signed: MachineWord + WordOps;

    fn to_signed(self) -> Self::Signed;
    fn from_signed(s: Self::Signed) -> Self;

    /// Construct from a u64, keeping the low bits (RV32 truncates, RV64 passes through).
    fn from_u64(v: u64) -> Self;

    /// Construct from an i64, keeping the low bits (sign pattern preserved).
    fn from_i64(v: i64) -> Self;

    /// Widen a already sign-extended (into u32's high bits) immediate to `Self`,
    /// preserving the sign: RV32 truncates back to 32 bits, RV64 sign-extends.
    #[inline(always)]
    fn from_imm(imm: u32) -> Self {
        Self::from_u64((imm as i32 as i64) as u64)
    }

    const BITS: u32;

    /// Shift-amount mask for `sll/srl/sra`-style instructions: `BITS - 1`.
    /// Takes `self` only to pin `Self` in generic code.
    #[inline(always)]
    fn shamt_mask(self) -> u32 {
        Self::BITS - 1
    }
}

impl MachineWord for u32 {}
impl MachineWord for i32 {}
impl MachineWord for u64 {}
impl MachineWord for i64 {}
impl MachineWord for u128 {}
impl MachineWord for i128 {}

impl Xlen for u32 {
    type Signed = i32;
    #[inline(always)]
    fn to_signed(self) -> i32 {
        self as i32
    }
    #[inline(always)]
    fn from_signed(s: i32) -> u32 {
        s as u32
    }
    #[inline(always)]
    fn from_u64(v: u64) -> u32 {
        v as u32
    }
    #[inline(always)]
    fn from_i64(v: i64) -> u32 {
        v as u32
    }
    const BITS: u32 = 32;
}

impl Xlen for u64 {
    type Signed = i64;
    #[inline(always)]
    fn to_signed(self) -> i64 {
        self as i64
    }
    #[inline(always)]
    fn from_signed(s: i64) -> u64 {
        s as u64
    }
    #[inline(always)]
    fn from_u64(v: u64) -> u64 {
        v
    }
    #[inline(always)]
    fn from_i64(v: i64) -> u64 {
        v as u64
    }
    const BITS: u32 = 64;
}

impl Xlen for u128 {
    type Signed = i128;
    #[inline(always)]
    fn to_signed(self) -> i128 {
        self as i128
    }
    #[inline(always)]
    fn from_signed(s: i128) -> u128 {
        s as u128
    }
    #[inline(always)]
    fn from_u64(v: u64) -> u128 {
        v as u128
    }
    #[inline(always)]
    fn from_i64(v: i64) -> u128 {
        v as u128
    }
    const BITS: u32 = 128;
}
