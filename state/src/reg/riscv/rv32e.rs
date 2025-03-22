use std::str::FromStr;

use logger::Logger;

use crate::reg::{RegError, RegIdentifier, RegIoResult, RegResult, RegfileIo};

use super::RvCsrEnum;

#[derive(Clone, Copy, Debug)]
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

impl Rv32eGprEnum {
    fn validate(index: u32) -> RegResult<Self> {
        Self::try_from(index).map_err(|_| RegError::InvalidGPRIndex)
    }
}

impl FromStr for Rv32eGprEnum {
    type Err = ();

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
            _ => Err(()),
        }
    }
}

impl TryFrom <u32> for Rv32eGprEnum {
    type Error = ();

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
            _ => Err(()),
        }
    }
}

impl Into<u32> for Rv32eGprEnum {
    fn into(self) -> u32 {
        self as u32
    }
}

impl Into<&str> for Rv32eGprEnum {
    fn into(self) -> &'static str {
        match self {
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

pub struct Rv32eRegFile {
    pc: u32,
    regs: [u32; 16],
    csrs: [u32; 4096],
}

impl Rv32eRegFile {
    pub fn new() -> Self {
        Rv32eRegFile {
            pc: 0,
            regs: [0; 16],
            csrs: [0; 4096],
        }
    }

    fn validate_gpr_index(index: u32) -> Result<u32, ()> {
        let index = Rv32eGprEnum::validate(index).map_err(|e| 
            Logger::show(&e.to_string(), Logger::ERROR)
        )?;

        Ok(index as u32)
    }

    fn validate_csr_index(index: u32) -> Result<u32, ()>  {
        let index = RvCsrEnum::validate(index).map_err(|e| 
            Logger::show(&e.to_string(), Logger::ERROR)
        )?;

        Ok(index as u32)
    }
}

impl RegfileIo for Rv32eRegFile {
    fn read_pc(&self) -> u32 {
        self.pc
    }

    fn write_pc(&mut self, value: u32) {
        self.pc = value;
    }

    fn read_gpr(&self,index : u32) -> RegIoResult<u32> {
        let index = Rv32eRegFile::validate_gpr_index(index)?;
        Ok(self.regs[index as usize])
    }

    fn write_gpr(&mut self,index : u32, value : u32) -> RegIoResult<()> {
        let index = Rv32eRegFile::validate_gpr_index(index)?;
        self.regs[index as usize] = value;
        Ok(())
    }

    fn read_csr(&self,index : u32) -> RegIoResult<u32> {
        let index = Rv32eRegFile::validate_csr_index(index)?;
        Ok(self.csrs[index as usize])
    }

    fn write_csr(&mut self,index : u32, value : u32) -> RegIoResult<()> {
        let index = Rv32eRegFile::validate_csr_index(index)?;
        self.csrs[index as usize] = value;
        Ok(())
    }

    fn print_gpr(&self, index: Option<RegIdentifier>) {
        match index {
            Some(RegIdentifier::Index(index)) => {
                let index = Rv32eRegFile::validate_gpr_index(index).unwrap();
                let name = Rv32eGprEnum::try_from(index).unwrap().into();
                self.print_format(name, self.regs[index as usize]);
            },

            Some(RegIdentifier::Name(name)) => {
                let name = Rv32eGprEnum::from_str(&name).unwrap();
                let index: u32 = name.into();
                self.print_format(name.into(), self.regs[index as usize]);
            },
            
            None => {
                for i in 0..16 {
                    let name = Rv32eGprEnum::try_from(i).unwrap().into();
                    self.print_format(name, self.regs[i as usize]);
                }
            }
        }
    }

    fn print_csr(&self, index: Option<RegIdentifier>) {
        match index {
            Some(RegIdentifier::Index(index)) => {
                let index = Rv32eRegFile::validate_csr_index(index).unwrap();
                let name = RvCsrEnum::try_from(index).unwrap().into();
                self.print_format(name, self.csrs[index as usize]);
            },

            Some(RegIdentifier::Name(name)) => {
                let name = RvCsrEnum::from_str(&name).unwrap();
                let index: u32 = name.into();
                self.print_format(name.into(), self.csrs[index as usize]);
            },
            
            None => {
                for csr in RvCsrEnum::iter() {
                    let name = csr.into();
                    self.print_format(name, self.csrs[csr as u32 as usize]);
                }
            }
        }
    }
}
