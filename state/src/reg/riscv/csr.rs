use std::str::FromStr;

use crate::reg::{RegError, RegResult};

#[derive(Clone, Copy, Debug)]
pub enum RvCsrEnum {
    MSTATUS     = 0x300,
    MTVEC       = 0x305,
    MSCRATCH    = 0x340,
    MEPC        = 0x341,
    MCAUSE      = 0x342,
    MVENDORID   = 0xF11,
    MARCHID     = 0xF12,
}

impl RvCsrEnum {
    pub fn validate(index: u32) -> RegResult<Self> {
        Self::try_from(index).map_err(|_| RegError::InvalidCSRIndex)
    }

    pub fn iter() -> impl Iterator<Item = RvCsrEnum> {
        [RvCsrEnum::MSTATUS, RvCsrEnum::MTVEC, RvCsrEnum::MSCRATCH, RvCsrEnum::MEPC, RvCsrEnum::MCAUSE, RvCsrEnum::MVENDORID, RvCsrEnum::MARCHID].iter().copied()
    }
}

impl FromStr for RvCsrEnum {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "mstatus"   => Ok(RvCsrEnum::MSTATUS),
            "mtvec"     => Ok(RvCsrEnum::MTVEC),
            "mscratch"  => Ok(RvCsrEnum::MSCRATCH),
            "mepc"      => Ok(RvCsrEnum::MEPC),
            "mcause"    => Ok(RvCsrEnum::MCAUSE),
            "mvendorid" => Ok(RvCsrEnum::MVENDORID),
            "marchid"   => Ok(RvCsrEnum::MARCHID),
            _ => Err(()),
        }
    }
}

impl TryFrom <u32> for RvCsrEnum {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0x300 => Ok(RvCsrEnum::MSTATUS),
            0x305 => Ok(RvCsrEnum::MTVEC),
            0x340 => Ok(RvCsrEnum::MSCRATCH),
            0x341 => Ok(RvCsrEnum::MEPC),
            0x342 => Ok(RvCsrEnum::MCAUSE),
            0xF11 => Ok(RvCsrEnum::MVENDORID),
            0xF12 => Ok(RvCsrEnum::MARCHID),
            _ => Err(()),
        }
    }
}

impl TryFrom <String> for RvCsrEnum {
    type Error = ();

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_str(&value)
    }
}

impl Into<u32> for RvCsrEnum {
    fn into(self) -> u32 {
        self as u32
    }
}

impl Into<&str> for RvCsrEnum {
    fn into(self) -> &'static str {
        match self {
            RvCsrEnum::MSTATUS   => "mstatus",
            RvCsrEnum::MTVEC     => "mtvec",
            RvCsrEnum::MSCRATCH  => "mscratch",
            RvCsrEnum::MEPC      => "mepc",
            RvCsrEnum::MCAUSE    => "mcause",
            RvCsrEnum::MVENDORID => "mvendorid",
            RvCsrEnum::MARCHID   => "marchid",
        }
    }
}
