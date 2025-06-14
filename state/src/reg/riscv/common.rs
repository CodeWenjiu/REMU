use std::{cell::RefCell, marker::PhantomData, rc::Rc, str::FromStr};

use logger::Logger;
use remu_macro::{log_err, log_error};
use remu_utils::{ProcessError, ProcessResult};

use crate::{reg::{AnyRegfile, RegError, RegIdentifier, RegResult, RegfileIo}, CheckFlags4reg};

use super::RvCsrEnum;

/// 通用寄存器枚举特性，为rv32e和rv32i统一接口
pub trait GprEnum: Sized + Copy + Clone + TryFrom<u32, Error = RegError> + Into<usize> + From<Self> + FromStr<Err = RegError> {
    fn from_identifier(identifier: RegIdentifier) -> RegResult<Self>;
    fn from_index(index: u32) -> RegResult<Self>;
    fn to_str(self) -> &'static str;
    fn is_zero(self) -> bool;
    fn reg_count() -> usize;
}

/// 实现ISA特定同步的特型
pub trait SyncRegOps {
    fn do_sync(&mut self, target: &AnyRegfile);
}

/// 通用RISC-V寄存器文件实现
#[derive(Clone)]
pub struct RiscvRegFile<G: GprEnum> {
    pub pc: Rc<RefCell<u32>>,
    pub regs: Rc<RefCell<Vec<u32>>>,
    pub csrs: Rc<RefCell<[u32; 4096]>>,
    _marker: PhantomData<G>,  // 表示这是G类型的寄存器文件
}

impl<G: GprEnum> RiscvRegFile<G> {
    fn init(&mut self, reset_vector: u32) {
        *self.pc.borrow_mut() = reset_vector;
        
        // 初始化CSR
        self.csrs.borrow_mut()[0x300] = 0x1800; // MSTATUS
        self.csrs.borrow_mut()[0xF11] = 0x79737978; // MVENDORID - "ysyx"
        self.csrs.borrow_mut()[0xF12] = 23060198; // MARCHID - my id
    }

    pub fn new(reset_vector: u32) -> Self {
        let mut result = RiscvRegFile {
            pc: Rc::new(RefCell::new(0)),
            regs: Rc::new(RefCell::new(vec![0; G::reg_count()])),
            csrs: Rc::new(RefCell::new([0; 4096])),
            _marker: PhantomData,
        };

        result.init(reset_vector);

        result
    }
}

impl<G: GprEnum + 'static> RegfileIo for RiscvRegFile<G>
where 
    Self: SyncRegOps,
{
    fn read_pc(&self) -> u32 {
        *self.pc.borrow()
    }

    fn write_pc(&mut self, value: u32) {
        *self.pc.borrow_mut() = value;
    }

    fn read_gpr(&self, index: u32) -> ProcessResult<u32> {
        let index = log_err!(G::from_index(index), ProcessError::Recoverable)?;
        Ok(self.regs.borrow()[index.into()])
    }

    fn write_gpr(&mut self, index: u32, value: u32) -> ProcessResult<()> {
        let index = log_err!(G::from_index(index), ProcessError::Recoverable)?;
        if !index.is_zero() {
            self.regs.borrow_mut()[index.into()] = value;
        }
        Ok(())
    }

    fn get_gprs(&self) -> Vec<u32> {
        self.regs.borrow().clone()
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

        if let Ok(index) = G::from_str(name) {
            return Ok(self.regs.borrow()[index.into()]);
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
        self.write_pc(value);
        Ok(())
    }

    fn print_gpr(&self, index: Option<RegIdentifier>) -> ProcessResult<()> {
        match index {
            Some(identifier) => {
                let index = log_err!(G::from_identifier(identifier), ProcessError::Recoverable)?;
                let name = index.to_str();
                self.print_format(name, self.regs.borrow()[index.into()]);
            }

            None => {
                for i in 0..G::reg_count() as u32 {
                    if let Ok(gpr) = G::from_index(i) {
                        let name = gpr.to_str();
                        self.print_format(name, self.regs.borrow()[i as usize]);
                    }
                }
            }
        }

        Ok(())
    }

    fn set_gpr(&mut self, index: RegIdentifier, value: u32) -> ProcessResult<()> {
        let index = log_err!(G::from_identifier(index), ProcessError::Recoverable)?;
        if !index.is_zero() {
            self.regs.borrow_mut()[index.into()] = value;
        }
        Ok(())
    }

    fn print_csr(&self, index: Option<RegIdentifier>) -> ProcessResult<()> {
        match index {
            Some(identifier) => {
                let index = RvCsrEnum::csr_identifier_converter(identifier)?;
                let name = index.into();
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

    fn check(&self, dut_regfile: &AnyRegfile, flags: CheckFlags4reg) -> ProcessResult<()> {
        let mut errors = Vec::new();

        // 检查 PC
        if flags.contains(CheckFlags4reg::pc) {
            if self.read_pc() != dut_regfile.read_pc() {
                errors.push(format!(
                    "Dut PC: {:#010x}, Ref PC: {:#010x}",
                    dut_regfile.read_pc(),
                    self.read_pc()
                ));
            }
        }

        // 检查 GPR
        if flags.contains(CheckFlags4reg::gpr) {
            let ref_gprs = dut_regfile.get_gprs();
            let gprs = self.get_gprs();
            
            for (i, (a, b)) in gprs.iter().zip(ref_gprs.iter()).enumerate() {
                if a != b {
                    errors.push(format!(
                        "Dut GPR[{}]: {:#010x}, Ref GPR[{}]: {:#010x}",
                        i, b, i, a
                    ));
                }
            }
        }

        // 检查 CSR
        if flags.contains(CheckFlags4reg::csr) {
            let csr_errors: Vec<_> = RvCsrEnum::iter()
                .filter_map(|csr| {
                    let index = csr as u32;
                    if self.csrs.borrow()[index as usize] != 
                       match dut_regfile {
                           AnyRegfile::Rv32e(rf) => rf.csrs.borrow()[index as usize],
                           AnyRegfile::Rv32i(rf) => rf.csrs.borrow()[index as usize],
                       } {
                        Some(format!(
                            "Dut CSR[{}]: {:#010x}, Ref CSR[{}]: {:#010x}",
                            index, 
                            match dut_regfile {
                                AnyRegfile::Rv32e(rf) => rf.csrs.borrow()[index as usize],
                                AnyRegfile::Rv32i(rf) => rf.csrs.borrow()[index as usize],
                            },
                            index, 
                            self.csrs.borrow()[index as usize]
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

        Ok(())
    }
    
    // 使用SyncRegOps的do_sync方法进行同步
    fn sync_reg(&mut self, target: &AnyRegfile) {
        SyncRegOps::do_sync(self, target);
    }
} 