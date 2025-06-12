use std::str::FromStr;

use crate::reg::{AnyRegfile, RegError, RegIdentifier, RegResult};

use super::common::{GprEnum, RiscvRegFile, SyncRegOps};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Rv32iGprEnum {
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
    A6 = 16,
    A7 = 17,
    S2 = 18,
    S3 = 19,
    S4 = 20,
    S5 = 21,
    S6 = 22,
    S7 = 23,
    S8 = 24,
    S9 = 25,
    S10 = 26,
    S11 = 27,
    T3 = 28,
    T4 = 29,
    T5 = 30,
    T6 = 31,
}

impl GprEnum for Rv32iGprEnum {
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
        self == Rv32iGprEnum::X0
    }

    fn reg_count() -> usize {
        32
    }
}

impl FromStr for Rv32iGprEnum {
    type Err = RegError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "x0" => Ok(Rv32iGprEnum::X0),
            "ra" => Ok(Rv32iGprEnum::RA),
            "sp" => Ok(Rv32iGprEnum::SP),
            "gp" => Ok(Rv32iGprEnum::GP),
            "tp" => Ok(Rv32iGprEnum::TP),
            "t0" => Ok(Rv32iGprEnum::T0),
            "t1" => Ok(Rv32iGprEnum::T1),
            "t2" => Ok(Rv32iGprEnum::T2),
            "s0" => Ok(Rv32iGprEnum::S0),
            "s1" => Ok(Rv32iGprEnum::S1),
            "a0" => Ok(Rv32iGprEnum::A0),
            "a1" => Ok(Rv32iGprEnum::A1),
            "a2" => Ok(Rv32iGprEnum::A2),
            "a3" => Ok(Rv32iGprEnum::A3),
            "a4" => Ok(Rv32iGprEnum::A4),
            "a5" => Ok(Rv32iGprEnum::A5),
            "a6" => Ok(Rv32iGprEnum::A6),
            "a7" => Ok(Rv32iGprEnum::A7),
            "s2" => Ok(Rv32iGprEnum::S2),
            "s3" => Ok(Rv32iGprEnum::S3),
            "s4" => Ok(Rv32iGprEnum::S4),
            "s5" => Ok(Rv32iGprEnum::S5),
            "s6" => Ok(Rv32iGprEnum::S6),
            "s7" => Ok(Rv32iGprEnum::S7),
            "s8" => Ok(Rv32iGprEnum::S8),
            "s9" => Ok(Rv32iGprEnum::S9),
            "s10" => Ok(Rv32iGprEnum::S10),
            "s11" => Ok(Rv32iGprEnum::S11),
            "t3" => Ok(Rv32iGprEnum::T3),
            "t4" => Ok(Rv32iGprEnum::T4),
            "t5" => Ok(Rv32iGprEnum::T5),
            "t6" => Ok(Rv32iGprEnum::T6),
            _ => Err(RegError::InvalidGPRName { name: s.to_string() }),
        }
    }
}

impl TryFrom<u32> for Rv32iGprEnum {
    type Error = RegError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Rv32iGprEnum::X0),
            1 => Ok(Rv32iGprEnum::RA),
            2 => Ok(Rv32iGprEnum::SP),
            3 => Ok(Rv32iGprEnum::GP),
            4 => Ok(Rv32iGprEnum::TP),
            5 => Ok(Rv32iGprEnum::T0),
            6 => Ok(Rv32iGprEnum::T1),
            7 => Ok(Rv32iGprEnum::T2),
            8 => Ok(Rv32iGprEnum::S0),
            9 => Ok(Rv32iGprEnum::S1),
            10 => Ok(Rv32iGprEnum::A0),
            11 => Ok(Rv32iGprEnum::A1),
            12 => Ok(Rv32iGprEnum::A2),
            13 => Ok(Rv32iGprEnum::A3),
            14 => Ok(Rv32iGprEnum::A4),
            15 => Ok(Rv32iGprEnum::A5),
            16 => Ok(Rv32iGprEnum::A6),
            17 => Ok(Rv32iGprEnum::A7),
            18 => Ok(Rv32iGprEnum::S2),
            19 => Ok(Rv32iGprEnum::S3),
            20 => Ok(Rv32iGprEnum::S4),
            21 => Ok(Rv32iGprEnum::S5),
            22 => Ok(Rv32iGprEnum::S6),
            23 => Ok(Rv32iGprEnum::S7),
            24 => Ok(Rv32iGprEnum::S8),
            25 => Ok(Rv32iGprEnum::S9),
            26 => Ok(Rv32iGprEnum::S10),
            27 => Ok(Rv32iGprEnum::S11),
            28 => Ok(Rv32iGprEnum::T3),
            29 => Ok(Rv32iGprEnum::T4),
            30 => Ok(Rv32iGprEnum::T5),
            31 => Ok(Rv32iGprEnum::T6),
            _ => Err(RegError::InvalidGPRIndex { index: value }),
        }
    }
}

impl Into<usize> for Rv32iGprEnum {
    fn into(self) -> usize {
        self as usize
    }
}

impl From<Rv32iGprEnum> for &str {
    fn from(reg: Rv32iGprEnum) -> Self {
        match reg {
            Rv32iGprEnum::X0 => "x0",
            Rv32iGprEnum::RA => "ra",
            Rv32iGprEnum::SP => "sp",
            Rv32iGprEnum::GP => "gp",
            Rv32iGprEnum::TP => "tp",
            Rv32iGprEnum::T0 => "t0",
            Rv32iGprEnum::T1 => "t1",
            Rv32iGprEnum::T2 => "t2",
            Rv32iGprEnum::S0 => "s0",
            Rv32iGprEnum::S1 => "s1",
            Rv32iGprEnum::A0 => "a0",
            Rv32iGprEnum::A1 => "a1",
            Rv32iGprEnum::A2 => "a2",
            Rv32iGprEnum::A3 => "a3",
            Rv32iGprEnum::A4 => "a4",
            Rv32iGprEnum::A5 => "a5",
            Rv32iGprEnum::A6 => "a6",
            Rv32iGprEnum::A7 => "a7",
            Rv32iGprEnum::S2 => "s2",
            Rv32iGprEnum::S3 => "s3",
            Rv32iGprEnum::S4 => "s4",
            Rv32iGprEnum::S5 => "s5",
            Rv32iGprEnum::S6 => "s6",
            Rv32iGprEnum::S7 => "s7",
            Rv32iGprEnum::S8 => "s8",
            Rv32iGprEnum::S9 => "s9",
            Rv32iGprEnum::S10 => "s10",
            Rv32iGprEnum::S11 => "s11",
            Rv32iGprEnum::T3 => "t3",
            Rv32iGprEnum::T4 => "t4",
            Rv32iGprEnum::T5 => "t5",
            Rv32iGprEnum::T6 => "t6",
        }
    }
}

// 使用通用寄存器文件实现
pub type Rv32iRegFile = RiscvRegFile<Rv32iGprEnum>;

// 为Rv32iRegFile实现SyncRegOps特型
impl SyncRegOps for Rv32iRegFile {
    fn do_sync(&mut self, target: &AnyRegfile) {
        if let AnyRegfile::Rv32i(target_rf) = target {
            self.pc.replace(target_rf.pc.borrow().clone());
            self.regs.replace(target_rf.regs.borrow().clone());
            // CSR暂不需要同步
        } else {
            panic!("Invalid register file type for Rv32iRegFile");
        }
    }
}

// 创建Rv32iRegFile的函数
pub fn new_rv32i_regfile(reset_vector: u32) -> Rv32iRegFile {
    Rv32iRegFile::new(reset_vector)
}