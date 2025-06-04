use std::{cell::RefCell, rc::Rc, str::FromStr};

use logger::Logger;
use remu_macro::{log_err, log_error};
use remu_utils::{ProcessError, ProcessResult};

use crate::{reg::{AnyRegfile, RegError, RegIdentifier, RegResult, RegfileIo}, CheckFlags4reg};

use super::RvCsrEnum;

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

impl Rv32iGprEnum {
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

impl TryFrom <u32> for Rv32iGprEnum {
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

#[derive(Clone)]
pub struct Rv32iRegFile {
    pub pc: Rc<RefCell<u32>>,
    pub regs: Rc<RefCell<[u32; 32]>>,
    pub csrs: Rc<RefCell<[u32; 4096]>>,
}

impl Rv32iRegFile {
    fn init(&mut self, reset_vector: u32) {
        self.write_pc(reset_vector);
        
        self.write_csr(RvCsrEnum::MSTATUS.into(), 0x1800).unwrap();
        self.write_csr(RvCsrEnum::MVENDORID.into(), 0x79737978).unwrap(); // "ysyx"
        self.write_csr(RvCsrEnum::MARCHID.into(), 23060198).unwrap(); // my id
    }
    
    pub fn new(reset_vector: u32) -> Self {
        let mut result = Rv32iRegFile {
            pc: Rc::new(RefCell::new(0)),
            regs: Rc::new(RefCell::new([0; 32])),
            csrs: Rc::new(RefCell::new([0; 4096])),
        };

        result.init(reset_vector);

        result
    }
}

impl RegfileIo for Rv32iRegFile {
    fn read_pc(&self) -> u32 {
        *self.pc.borrow()
    }

    fn write_pc(&mut self, value: u32) {
        *self.pc.borrow_mut() = value;
    }

    fn read_gpr(&self, index: u32) -> ProcessResult<u32> {
        let index = log_err!(Rv32iGprEnum::gpr_index_converter(index), ProcessError::Recoverable)?;
        Ok(self.regs.borrow()[index as usize])
    }

    fn write_gpr(&mut self, index: u32, value: u32) -> ProcessResult<()> {
        let index = log_err!(Rv32iGprEnum::gpr_index_converter(index), ProcessError::Recoverable)?;
        if index == Rv32iGprEnum::X0 {
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

    fn write_csr(&mut self, index: u32, value: u32) -> ProcessResult<()> {
        let index = RvCsrEnum::csr_index_converter(index)?;
        self.csrs.borrow_mut()[index as usize] = value;
        Ok(())
    }

    fn read_reg(&self, name: &str) -> ProcessResult<u32> {
        if name == "pc" {
            return Ok(self.read_pc());
        }

        if let Ok(index) = Rv32iGprEnum::from_str(name) {
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
                let index = log_err!(Rv32iGprEnum::gpr_identifier_converter(identifier), ProcessError::Recoverable)?;
                let name = Rv32iGprEnum::from(index).into();
                self.print_format(name, self.regs.borrow()[index as usize]);
            }

            None => {
                for i in 0..32 {
                    let name = Rv32iGprEnum::try_from(i).unwrap().into();
                    self.print_format(name, self.regs.borrow()[i as usize]);
                }
            }
        }

        Ok(())
    }

    fn set_gpr(&mut self, index: RegIdentifier, value: u32) -> ProcessResult<()> {
        let index = log_err!(Rv32iGprEnum::gpr_identifier_converter(index), ProcessError::Recoverable)?;
        
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
        if let AnyRegfile::Rv32i(target) = _target {
            self.pc.replace(target.pc.borrow().clone());
            self.regs.replace(target.regs.borrow().clone());
            // csr is not need for now
        } else {
            panic!("Invalid register file type");
        }
    }

    fn check(&self, regfile: &AnyRegfile, flags: CheckFlags4reg) -> ProcessResult<()> {
        if let AnyRegfile::Rv32i(regfile) = regfile {
            let mut errors = Vec::new();

            // 检查 PC
            if flags.contains(CheckFlags4reg::pc) {
                if *self.pc.borrow() != *regfile.pc.borrow() {
                    errors.push(format!(
                        "Dut PC: {:#010x}, Ref PC: {:#010x}",
                        regfile.read_pc(),
                        self.read_pc()
                    ));
                }
            }

            // 检查 GPR
            if flags.contains(CheckFlags4reg::gpr) {
                let gpr_errors: Vec<_> = self.get_gprs()
                    .iter()
                    .zip(regfile.get_gprs().iter())
                    .enumerate()
                    .filter_map(|(i, (a, b))| {
                        if a != b {
                            Some(format!(
                                "Dut GPR[{}]: {:#010x}, Ref GPR[{}]: {:#010x}",
                                i, b, i, a
                            ))
                        } else {
                            None
                        }
                    })
                    .collect();
                errors.extend(gpr_errors);
            }

            // 检查 CSR
            if flags.contains(CheckFlags4reg::csr) {
                let csr_errors: Vec<_> = RvCsrEnum::iter()
                    .filter_map(|csr| {
                        let index = csr as u32;
                        if self.csrs.borrow()[index as usize] != regfile.csrs.borrow()[index as usize] {
                            Some(format!(
                                "Dut CSR[{}]: {:#010x}, Ref CSR[{}]: {:#010x}",
                                index, self.csrs.borrow()[index as usize], index, regfile.csrs.borrow()[index as usize]
                            ))
                        } else {
                            None
                        }
                    })
                    .collect();
                errors.extend(csr_errors);
            }

            // 统一处理所有错误
            if !errors.is_empty() {
                for error in errors {
                    log_error!(error);
                }
                return Err(ProcessError::Recoverable);
            }
        } else {
            panic!("Invalid register file type");
        }

        Ok(())
    }
}