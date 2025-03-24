use std::{cell::RefCell, rc::Rc, str::FromStr};

use logger::Logger;
use remu_macro::log_err;

use crate::reg::{RegError, RegIdentifier, RegIoResult, RegResult, RegfileIo};

use super::RvCsrEnum;

#[derive(Clone, Copy, Debug)]
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

impl Rv32iGprEnum {
    pub fn validate(index: u32) -> RegResult<Self> {
        Self::try_from(index).map_err(|_| RegError::InvalidGPRIndex)
    }
}

impl FromStr for Rv32iGprEnum {
    type Err = ();

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
            _ => Err(()),
        }
    }
}

impl TryFrom <u32> for Rv32iGprEnum {
    type Error = ();

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
            _ => Err(()),
        }
    }
}

impl Into<usize> for Rv32iGprEnum {
    fn into(self) -> usize {
        self as usize
    }
}

impl Into<&str> for Rv32iGprEnum {
    fn into(self) -> &'static str {
        match self {
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

#[derive(Clone)]
pub struct Rv32iRegFile {
    pub pc: Rc<RefCell<u32>>,
    pub regs: Rc<RefCell<[u32; 32]>>,
    pub csrs: Rc<RefCell<[u32; 4096]>>,
}

impl Rv32iRegFile {
    pub fn new(reset_vector: u32) -> Self {
        Rv32iRegFile {
            pc: Rc::new(RefCell::new(reset_vector)),
            regs: Rc::new(RefCell::new([0; 32])),
            csrs: Rc::new(RefCell::new([0; 4096])),
        }
    }

    fn validate_gpr_index(index: u32) -> Result<u32, ()> {
        let index = log_err!(Rv32iGprEnum::validate(index))?;

        Ok(index as u32)
    }

    fn validate_csr_index(index: u32) -> Result<u32, ()> {
        let index = log_err!(RvCsrEnum::validate(index))?;

        Ok(index as u32)
    }
}

impl RegfileIo for Rv32iRegFile {
    fn read_pc(&self) -> u32 {
        *self.pc.borrow()
    }

    fn write_pc(&mut self, value: u32) {
        *self.pc.borrow_mut() = value;
    }

    fn read_gpr(&self, index: u32) -> RegIoResult<u32> {
        let index = Rv32iRegFile::validate_gpr_index(index)?;
        Ok(self.regs.borrow()[index as usize])
    }

    fn write_gpr(&mut self, index: u32, value: u32) -> RegIoResult<()> {
        let index = Rv32iRegFile::validate_gpr_index(index)?;
        self.regs.borrow_mut()[index as usize] = value;
        Ok(())
    }

    fn read_csr(&self, index: u32) -> RegIoResult<u32> {
        let index = Rv32iRegFile::validate_csr_index(index)?;
        Ok(self.csrs.borrow()[index as usize])
    }

    fn write_csr(&mut self, index: u32, value: u32) -> RegIoResult<()> {
        let index = Rv32iRegFile::validate_csr_index(index)?;
        self.csrs.borrow_mut()[index as usize] = value;
        Ok(())
    }

    fn print_pc(&self) {
        self.print_format("PC", self.read_pc());
    }

    fn print_gpr(&self, index: Option<RegIdentifier>) {
        match index {
            Some(RegIdentifier::Index(index)) => {
                let index = Rv32iRegFile::validate_gpr_index(index).unwrap();
                let name = Rv32iGprEnum::try_from(index).unwrap().into();
                self.print_format(name, self.regs.borrow()[index as usize]);
            },

            Some(RegIdentifier::Name(name)) => {
                let name = Rv32iGprEnum::from_str(&name).unwrap();
                let index: usize = Rv32iGprEnum::try_from(name).unwrap().into();
                self.print_format(name.into(), self.regs.borrow()[index]);
            },

            None => {
                for i in 0..32 {
                    let name = Rv32iGprEnum::try_from(i).unwrap().into();
                    self.print_format(name, self.regs.borrow()[i as usize]);
                }
            }
        }
    }

    fn print_csr(&self, index: Option<RegIdentifier>) {
        match index {
            Some(RegIdentifier::Index(index)) => {
                let index = Rv32iRegFile::validate_csr_index(index).unwrap();
                let name = RvCsrEnum::try_from(index).unwrap().into();
                self.print_format(name, self.csrs.borrow()[index as usize]);
            },

            Some(RegIdentifier::Name(name)) => {
                let name = RvCsrEnum::try_from(name).unwrap();
                let index = RvCsrEnum::try_from(name).unwrap() as u32;
                self.print_format(name.into(), self.csrs.borrow()[index as usize]);
            },

            None => {
                for csr in RvCsrEnum::iter() {
                    let name = csr.into();
                    self.print_format(name, self.csrs.borrow()[csr as usize]);
                }
            }
        }
    }
}