use enum_dispatch::enum_dispatch;
use logger::Logger;
use owo_colors::OwoColorize;
use remu_utils::ISA;
use riscv::{Rv32eRegFile, Rv32iRegFile};

use crate::CheckFlags4reg;

remu_macro::mod_pub!(riscv);

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
        Logger::todo();
        0
    }

    fn write_pc(&mut self, _value: u32) {
        Logger::todo();
    }

    fn read_gpr(&self, _index: u32) -> RegIoResult<u32> {
        Logger::todo();
        Err(())
    }

    fn write_gpr(&mut self, _index: u32, _value: u32) -> RegIoResult<()> {
        Logger::todo();
        Err(())
    }

    fn read_csr(&self, _index: u32) -> RegIoResult<u32> {
        Logger::todo();
        Err(())
    }

    fn write_csr(&mut self, _index: u32, _value: u32) -> RegIoResult<()> {
        Logger::todo();
        Err(())
    }

    fn print_pc(&self) {
        self.print_format("PC", self.read_pc());
    }

    fn print_gpr(&self, _index: Option<RegIdentifier>) {
        Logger::todo();
    }

    fn print_csr(&self, _index: Option<RegIdentifier>) {
        Logger::todo();
    }

    fn check(&self, _regfile: AnyRegfile, _flags: CheckFlags4reg) -> Result<(), ()> {
        Logger::todo();
        Ok(())
    }
}

#[enum_dispatch]
#[derive(Clone)]
pub enum AnyRegfile {
    Rv32e(Rv32eRegFile),
    Rv32i(Rv32iRegFile),
}

#[derive(Debug, snafu::Snafu)]
pub enum RegError {
    #[snafu(display("Invalid generou purpose register index"))]
    InvalidGPRIndex,

    #[snafu(display("Invalid CSR index"))]
    InvalidCSRIndex,
}

type RegResult<T> = Result<T, RegError>;
type RegIoResult<T> = Result<T, ()>;

pub fn regfile_io_factory(isa: ISA, reset_vector: u32) -> Result<AnyRegfile, ()> {
    match isa {
        ISA::RV32E => Ok(Rv32eRegFile::new(reset_vector).into()),
        ISA::RV32I => Ok(Rv32iRegFile::new(reset_vector).into()),
        _ => {
            let isa: &str = From::from(isa);
            Logger::show(&format!("Unknown ISA: {}", isa), Logger::ERROR);
            Err(())
        }
    }
}
