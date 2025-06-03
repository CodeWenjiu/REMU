use std::{cell::RefCell, rc::Rc, str::FromStr};

use logger::Logger;
use remu_macro::{log_err, log_error};
use remu_utils::{ProcessError, ProcessResult};

use crate::{reg::{AnyRegfile, RegError, RegIdentifier, RegResult, RegfileIo}, CheckFlags4reg};

use super::RvCsrEnum;

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

impl Rv32eGprEnum {
    fn gpr_index_converter(index: u32) -> RegResult<Self> {
        Self::try_from(index)
    }

    fn gpr_identifier_converter(index: RegIdentifier) -> RegResult<Self> {
        let index = match index {
            RegIdentifier::Index(index) => Self::gpr_index_converter(index)?,
            RegIdentifier::Name(name) => Self::from_str(&name)?,
        };
        Ok(index)
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

impl TryFrom <u32> for Rv32eGprEnum {
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

impl Into<u32> for Rv32eGprEnum {
    fn into(self) -> u32 {
        self as u32
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

#[derive(Clone)]
pub struct Rv32eRegFile {
    pub pc: Rc<RefCell<u32>>,
    pub regs: Rc<RefCell<[u32; 16]>>,
    pub csrs: Rc<RefCell<[u32; 4096]>>,
}

impl Rv32eRegFile {
    fn init(&mut self, reset_vector: u32) {
        self.write_pc(reset_vector);
        
        self.write_csr(RvCsrEnum::MSTATUS.into(), 0x1800).unwrap();
        self.write_csr(RvCsrEnum::MVENDORID.into(), 0x79737978).unwrap(); // "ysyx"
        self.write_csr(RvCsrEnum::MARCHID.into(), 23060198).unwrap(); // my id
    }

    pub fn new(reset_vector: u32) -> Self {
        let mut result = Rv32eRegFile {
            pc: Rc::new(RefCell::new(0)),
            regs: Rc::new(RefCell::new([0; 16])),
            csrs: Rc::new(RefCell::new([0; 4096])),
        };

        result.init(reset_vector);

        result
    }
}

impl RegfileIo for Rv32eRegFile {
    fn read_pc(&self) -> u32 {
        *self.pc.borrow()
    }

    fn write_pc(&mut self, value: u32) {
        *self.pc.borrow_mut() = value;
    }

    fn read_gpr(&self, index: u32) -> ProcessResult<u32> {
        let index = log_err!(Rv32eGprEnum::gpr_index_converter(index), ProcessError::Recoverable)?;
        Ok(self.regs.borrow()[index as usize])
    }

    fn write_gpr(&mut self, index: u32, value : u32) -> ProcessResult<()> {
        let index = log_err!(Rv32eGprEnum::gpr_index_converter(index), ProcessError::Recoverable)?;
        if index == Rv32eGprEnum::X0 {
            return Ok(());
        }
        self.regs.borrow_mut()[index as usize] = value;
        Ok(())
    }

    fn get_gprs(&self) -> Vec<u32> {
        self.regs.borrow().to_vec()
    }

    fn read_csr(&self, index: u32) -> ProcessResult<u32> {
        let index = RvCsrEnum::csr_index_converter(index)?;
        Ok(self.csrs.borrow()[index as usize])
    }

    fn write_csr(&mut self, index: u32, value : u32) -> ProcessResult<()> {
        let index = RvCsrEnum::csr_index_converter(index)?;
        self.csrs.borrow_mut()[index as usize] = value;
        Ok(())
    }

    fn read_reg(&self, name: &str) -> ProcessResult<u32> {
        if name == "pc" {
            return Ok(self.read_pc());
        }

        if let Ok(index) = Rv32eGprEnum::from_str(name) {
            return Ok(self.regs.borrow()[index as usize]);
        }

        if let Ok(index) = RvCsrEnum::from_str(name) {
            return Ok(self.csrs.borrow()[index as usize]);
        }
        
        log_error!(format!("Invalid register name: {}", name));

        Err(ProcessError::Recoverable)
    }

    fn print_pc(&self) {
        self.print_format("PC", self.read_pc());
    }

    fn set_pc(&mut self, value:u32) -> ProcessResult<()> {
        self.write_pc( value);
        Ok(())
    }

    fn print_gpr(&self, index: Option<RegIdentifier>) -> ProcessResult<()> {
        match index {
            Some(identifier) => {
                let index = log_err!(Rv32eGprEnum::gpr_identifier_converter(identifier), ProcessError::Recoverable)?;
                let name = Rv32eGprEnum::from(index).into();
                self.print_format(name, self.regs.borrow()[index as usize]);
            }

            None => {
                for i in 0..16 {
                    let name = Rv32eGprEnum::try_from(i).unwrap().into();
                    self.print_format(name, self.regs.borrow()[i as usize]);
                }
            }
        }

        Ok(())
    }

    fn set_gpr(&mut self, index: RegIdentifier, value: u32) -> ProcessResult<()> {
        let index = log_err!(Rv32eGprEnum::gpr_identifier_converter(index), ProcessError::Recoverable)?;
        
        self.regs.borrow_mut()[index as usize] = value;

        Ok(())
    }

    fn print_csr(&self, index: Option<RegIdentifier>) -> ProcessResult<()> {
        match index {
            Some(identifier) => {
                let index = RvCsrEnum::csr_identifier_converter(identifier)?;
                let name = RvCsrEnum::from(index).into();
                self.print_format(name, self.csrs.borrow()[index as usize]);
            }

            None => {
                for csr in RvCsrEnum::iter() {
                    let name = csr.into();
                    self.print_format(name, self.csrs.borrow()[csr as u32 as usize]);
                }
            }
        }

        Ok(())
    }

    fn set_csr(&mut self, index: RegIdentifier, value: u32) -> ProcessResult<()> {
        let index = RvCsrEnum::csr_identifier_converter(index)?;
        self.csrs.borrow_mut()[index as usize] = value;
        Ok(())
    }

    fn sync_reg(&mut self,_target: &crate::reg::AnyRegfile) {
        if let AnyRegfile::Rv32e(target) = _target {
            self.pc.replace(target.pc.borrow().clone());
            self.regs.replace(target.regs.borrow().clone());
            // csr is not need for now
        } else {
            panic!("Invalid register file type");
        }
    }

    fn check(&self, regfile: &AnyRegfile, flags: CheckFlags4reg) -> ProcessResult<()> {
        if let AnyRegfile::Rv32e(regfile) = regfile {
            if flags.contains(CheckFlags4reg::pc) {
                if *self.pc.borrow() != *regfile.pc.borrow() {
                    log_error!(format!(
                        "Dut PC: {:#010x}, Ref PC: {:#010x}",
                        self.read_pc(),
                        regfile.read_pc()
                    ));
                    return Err(ProcessError::Recoverable);
                }
            }

            if flags.contains(CheckFlags4reg::gpr) {
                let gprs = self.get_gprs();
                let ref_gprs = regfile.get_gprs();

                for (i, (a, b)) in gprs.iter().zip(ref_gprs.iter()).enumerate() {
                    if a != b {
                        log_error!(format!(
                            "Dut GPR[{}]: {:#010x}, Ref GPR[{}]: {:#010x}",
                            i, b, i, a
                        ));
                        return Err(ProcessError::Recoverable);
                    }
                }
            }

            if flags.contains(CheckFlags4reg::csr) {
                for csr in RvCsrEnum::iter() {
                    let index = csr as u32;
                    if self.csrs.borrow()[index as usize] != regfile.csrs.borrow()[index as usize] {
                        log_error!(format!(
                            "Dut CSR[{}]: {:#010x}, Ref CSR[{}]: {:#010x}",
                            index, self.csrs.borrow()[index as usize], index, regfile.csrs.borrow()[index as usize]
                        ));
                        return Err(ProcessError::Recoverable);
                    }
                }
            }
        } else {
            panic!("Invalid register file type");
        }

        Ok(())
    }
}
