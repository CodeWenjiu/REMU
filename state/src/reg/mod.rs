use enum_dispatch::enum_dispatch;
use logger::Logger;
use owo_colors::OwoColorize;
use remu_macro::{log_error, log_todo};
use remu_utils::{ProcessError, ProcessResult, ISA};
use riscv::{Rv32eGprEnum, Rv32eRegFile, Rv32iGprEnum, Rv32iRegFile, RvCsrEnum};

use crate::CheckFlags4reg;

remu_macro::mod_pub!(riscv);

#[derive(Clone, Debug)]
pub enum ALLGPRIdentifier {
    Rv32eGprEnum(Rv32eGprEnum),
    Rv32iGprEnum(Rv32iGprEnum),
}

#[derive(Clone, Debug)]
pub enum ALLCSRIdentifier {
    RISCV(RvCsrEnum),
}

#[derive(Clone, Debug)]
pub enum RegIdentifier {
    Index(u32),
    Name(String),
}

#[enum_dispatch(AnyRegfile)]
pub trait RegfileIo {
    fn print_format(&self, name: &str, data: u32) {
        println!("{}: \t{:#010x}", name.purple(), data.blue());
    }

    fn read_pc(&self) -> u32 {
        log_todo!();
        0
    }

    fn write_pc(&mut self, _value: u32) {
        log_todo!();
    }

    fn read_gpr(&self, _index: u32) -> ProcessResult<u32> {
        log_todo!();
        Err(ProcessError::Recoverable)
    }

    fn write_gpr(&mut self, _index: u32, _value: u32) -> ProcessResult<()> {
        log_todo!();
        Err(ProcessError::Recoverable)
    }

    fn get_gprs(&self) -> Vec<u32> {
        log_todo!();
        Vec::new()
    }

    fn read_csr(&self, _index: u32) -> ProcessResult<u32> {
        log_todo!();
        Err(ProcessError::Recoverable)
    }

    fn write_csr(&mut self, _index: u32, _value: u32) -> ProcessResult<()> {
        log_todo!();
        Err(ProcessError::Recoverable)
    }

    fn print_pc(&self) {
        self.print_format("PC", self.read_pc());
    }

    fn print_gpr(&self, _index: Option<RegIdentifier>) -> ProcessResult<()> {
        log_todo!();
        Err(ProcessError::Recoverable)
    }

    fn print_csr(&self, _index: Option<RegIdentifier>) -> ProcessResult<()> {
        log_todo!();
        Err(ProcessError::Recoverable)
    }

    fn set_reg(&mut self, _target: &AnyRegfile) {
        log_todo!()
    }

    fn check(&self, regfile: &AnyRegfile, flags: CheckFlags4reg) -> ProcessResult<()> {
        if flags.contains(CheckFlags4reg::pc) {
            if self.read_pc() != regfile.read_pc() {
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
                        i, a, i, b
                    ));
                    return Err(ProcessError::Recoverable);
                }
            }
        }
        if flags.contains(CheckFlags4reg::csr) {
            log_todo!();
        }
        Ok(())
    }
}

#[enum_dispatch]
#[derive(Clone)]
pub enum AnyRegfile {
    Rv32e(Rv32eRegFile),
    Rv32i(Rv32iRegFile),
}

impl AnyRegfile {
    pub fn gpr_into_str (&self, index: u32) -> &str {
        match self {
            AnyRegfile::Rv32e(_) => {
                Rv32eGprEnum::try_from(index).unwrap().into()
            },
            AnyRegfile::Rv32i(_) => {
                Rv32eGprEnum::try_from(index).unwrap().into()
            }
        }
    }
}

#[derive(Debug, snafu::Snafu)]
pub enum RegError {
    #[snafu(display("Invalid generou purpose register index {}", index))]
    InvalidGPRIndex { index: u32 },
    
    #[snafu(display("Invalid generou purpose register name {}", name))]
    InvalidGPRName { name: String },

    #[snafu(display("Invalid CSR index {}", index))]
    InvalidCSRIndex { index: u32 },

    #[snafu(display("Invalid CSR name {}", name))]
    InvalidCSRName { name: String },
}

type RegResult<T> = Result<T, RegError>;

pub fn regfile_io_factory(isa: ISA, reset_vector: u32) -> Result<AnyRegfile, ()> {
    match isa {
        ISA::RV32E => Ok(Rv32eRegFile::new(reset_vector).into()),
        ISA::RV32I => Ok(Rv32iRegFile::new(reset_vector).into()),
        ISA::RV32IM => Ok(Rv32iRegFile::new(reset_vector).into()),
    }
}
