use std::str::FromStr;

use crate::reg::{ALLCSRIdentifier, RegError, RegIdentifier, RegResult};

#[derive(PartialEq, Clone, Copy, Default, Debug)]
pub enum Trap {
    #[default]
    IllegalInstruction = 2,

    Ebreak = 3,

    EcallM = 11,

    Mret, // Not a trap, but used for MRET instruction
}

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
        Self::try_from(index)
    }

    pub fn iter() -> impl Iterator<Item = Self> {
        [Self::MSTATUS, Self::MTVEC, Self::MSCRATCH, Self::MEPC, Self::MCAUSE, Self::MVENDORID, Self::MARCHID].iter().copied()
    }

    pub fn csr_index_converter(index: u32) -> RegResult<Self> {
        RvCsrEnum::try_from(index)
    }

    pub fn csr_identifier_converter(index: RegIdentifier) -> RegResult<Self> {
        let index = match index {
            RegIdentifier::Index(index) => Self::csr_index_converter(index)?,
            RegIdentifier::Name(name) => Self::from_str(&name)?,
        };
        Ok(index)
    }
}

impl FromStr for RvCsrEnum {
    type Err = RegError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "mstatus"   => Ok(RvCsrEnum::MSTATUS),
            "mtvec"     => Ok(RvCsrEnum::MTVEC),
            "mscratch"  => Ok(RvCsrEnum::MSCRATCH),
            "mepc"      => Ok(RvCsrEnum::MEPC),
            "mcause"    => Ok(RvCsrEnum::MCAUSE),
            "mvendorid" => Ok(RvCsrEnum::MVENDORID),
            "marchid"   => Ok(RvCsrEnum::MARCHID),
            _ => Err(RegError::InvalidCSRName { name: s.to_string() }),
        }
    }
}

impl TryFrom <u32> for RvCsrEnum {
    type Error = RegError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0x300 => Ok(RvCsrEnum::MSTATUS),
            0x305 => Ok(RvCsrEnum::MTVEC),
            0x340 => Ok(RvCsrEnum::MSCRATCH),
            0x341 => Ok(RvCsrEnum::MEPC),
            0x342 => Ok(RvCsrEnum::MCAUSE),
            0xF11 => Ok(RvCsrEnum::MVENDORID),
            0xF12 => Ok(RvCsrEnum::MARCHID),
            _ => Err(RegError::InvalidCSRIndex { index: value }),
        }
    }
}

impl TryFrom <String> for RvCsrEnum {
    type Error = RegError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_str(&value)
    }
}

impl Into<u32> for RvCsrEnum {
    fn into(self) -> u32 {
        self as u32
    }
}

impl Into<ALLCSRIdentifier> for RvCsrEnum {
    fn into(self) -> ALLCSRIdentifier {
        ALLCSRIdentifier::RISCV(self)
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
