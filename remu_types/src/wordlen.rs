use std::{
    fmt::{Debug, LowerHex},
    ops::{Add, BitAnd, BitOr, BitXor, Not, Shl, Shr, Sub},
};

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

impl MachineWord for u32 {}
impl MachineWord for i32 {}
impl MachineWord for u64 {}
impl MachineWord for i64 {}

pub trait Xlen: MachineWord {
    type Signed: MachineWord;

    type Unsigned: MachineWord;

    fn to_signed(self) -> Self::Signed;
    fn from_signed(s: Self::Signed) -> Self::Unsigned;

    const BITS: u32;
}

impl Xlen for u32 {
    type Signed = i32;
    type Unsigned = u32;

    #[inline(always)]
    fn to_signed(self) -> i32 {
        self as i32
    }

    #[inline(always)]
    fn from_signed(s: i32) -> u32 {
        s as u32
    }

    const BITS: u32 = 32;
}

impl Xlen for u64 {
    type Signed = i64;
    type Unsigned = u64;

    #[inline(always)]
    fn to_signed(self) -> i64 {
        self as i64
    }

    #[inline(always)]
    fn from_signed(s: i64) -> u64 {
        s as u64
    }

    const BITS: u32 = 64;
}
