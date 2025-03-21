use enum_dispatch::enum_dispatch;
use logger::Logger;
use owo_colors::OwoColorize;
use riscv::Rv32eRegFile;

remu_macro::mod_pub!(riscv);

#[enum_dispatch]
pub trait RegfileIo {
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
        println!("{}: \t{:#010x}", "PC".purple(), self.read_pc().blue());
    }

    fn print_gpr(&self) {
        Logger::todo();
    }

    fn print_csr(&self) {
        Logger::todo();
    }
}

#[enum_dispatch(RegfileIo)]
pub enum AnyRegfile {
    Rv32e(Rv32eRegFile),
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

pub fn regfile_io_factory(isa: &str) -> Result<Box<dyn RegfileIo>, ()> {
    match isa {
        "rv32e" => Ok(Box::new(Rv32eRegFile::new())),
        _ => {
            Logger::show(&format!("Unknown ISA: {}", isa), Logger::ERROR);
            Err(())
        }
    }
}
