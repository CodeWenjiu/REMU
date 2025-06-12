use std::str::FromStr;

use crate::reg::{AnyRegfile, RegError, RegIdentifier, RegResult};

use super::common::{GprEnum, RiscvRegFile, SyncRegOps};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Rv32eGprEnum {
    X0 = 0,
    RA = 1,
    SP = 2,
    GP = 3,
    TP = 4,
    T0 = 5,
    T1 = 6,
    T2 = 7,
    S0 = 8,
    S1 = 9,
    A0 = 10,
    A1 = 11,
    A2 = 12,
    A3 = 13,
    A4 = 14,
    A5 = 15,
}

impl GprEnum for Rv32eGprEnum {
    fn from_identifier(identifier: RegIdentifier) -> RegResult<Self> {
        match identifier {
            RegIdentifier::Index(index) => Self::from_index(index),
            RegIdentifier::Name(name) => Self::from_str(&name),
        }
    }

    fn from_index(index: u32) -> RegResult<Self> {
        Self::try_from(index)
    }

    fn to_str(self) -> &'static str {
        self.into()
    }

    fn is_zero(self) -> bool {
        self == Rv32eGprEnum::X0
    }

    fn reg_count() -> usize {
        16
    }
}

impl FromStr for Rv32eGprEnum {
    type Err = RegError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "x0" => Ok(Rv32eGprEnum::X0),
            "ra" => Ok(Rv32eGprEnum::RA),
            "sp" => Ok(Rv32eGprEnum::SP),
            "gp" => Ok(Rv32eGprEnum::GP),
            "tp" => Ok(Rv32eGprEnum::TP),
            "t0" => Ok(Rv32eGprEnum::T0),
            "t1" => Ok(Rv32eGprEnum::T1),
            "t2" => Ok(Rv32eGprEnum::T2),
            "s0" => Ok(Rv32eGprEnum::S0),
            "s1" => Ok(Rv32eGprEnum::S1),
            "a0" => Ok(Rv32eGprEnum::A0),
            "a1" => Ok(Rv32eGprEnum::A1),
            "a2" => Ok(Rv32eGprEnum::A2),
            "a3" => Ok(Rv32eGprEnum::A3),
            "a4" => Ok(Rv32eGprEnum::A4),
            "a5" => Ok(Rv32eGprEnum::A5),
            _ => Err(RegError::InvalidGPRName { name: s.to_string() }),
        }
    }
}

impl TryFrom<u32> for Rv32eGprEnum {
    type Error = RegError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Rv32eGprEnum::X0),
            1 => Ok(Rv32eGprEnum::RA),
            2 => Ok(Rv32eGprEnum::SP),
            3 => Ok(Rv32eGprEnum::GP),
            4 => Ok(Rv32eGprEnum::TP),
            5 => Ok(Rv32eGprEnum::T0),
            6 => Ok(Rv32eGprEnum::T1),
            7 => Ok(Rv32eGprEnum::T2),
            8 => Ok(Rv32eGprEnum::S0),
            9 => Ok(Rv32eGprEnum::S1),
            10 => Ok(Rv32eGprEnum::A0),
            11 => Ok(Rv32eGprEnum::A1),
            12 => Ok(Rv32eGprEnum::A2),
            13 => Ok(Rv32eGprEnum::A3),
            14 => Ok(Rv32eGprEnum::A4),
            15 => Ok(Rv32eGprEnum::A5),
            _ => Err(RegError::InvalidGPRIndex { index: value }),
        }
    }
}

impl Into<usize> for Rv32eGprEnum {
    fn into(self) -> usize {
        self as usize
    }
}

impl From<Rv32eGprEnum> for &str {
    fn from(reg: Rv32eGprEnum) -> Self {
        match reg {
            Rv32eGprEnum::X0 => "x0",
            Rv32eGprEnum::RA => "ra",
            Rv32eGprEnum::SP => "sp",
            Rv32eGprEnum::GP => "gp",
            Rv32eGprEnum::TP => "tp",
            Rv32eGprEnum::T0 => "t0",
            Rv32eGprEnum::T1 => "t1",
            Rv32eGprEnum::T2 => "t2",
            Rv32eGprEnum::S0 => "s0",
            Rv32eGprEnum::S1 => "s1",
            Rv32eGprEnum::A0 => "a0",
            Rv32eGprEnum::A1 => "a1",
            Rv32eGprEnum::A2 => "a2",
            Rv32eGprEnum::A3 => "a3",
            Rv32eGprEnum::A4 => "a4",
            Rv32eGprEnum::A5 => "a5",
        }
    }
}

// 使用通用寄存器文件实现
pub type Rv32eRegFile = RiscvRegFile<Rv32eGprEnum>;

// 为Rv32eRegFile实现SyncRegOps特型
impl SyncRegOps for Rv32eRegFile {
    fn do_sync(&mut self, target: &AnyRegfile) {
        if let AnyRegfile::Rv32e(target_rf) = target {
            self.pc.replace(target_rf.pc.borrow().clone());
            self.regs.replace(target_rf.regs.borrow().clone());
            // CSR暂不需要同步
        } else {
            panic!("Invalid register file type for Rv32eRegFile");
        }
    }
}

// 创建Rv32eRegFile的函数
pub fn new_rv32e_regfile(reset_vector: u32) -> Rv32eRegFile {
    Rv32eRegFile::new(reset_vector)
}
