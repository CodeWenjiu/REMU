use remu_macro::log_error;
use logger::Logger;
use remu_utils::{ProcessError, ProcessResult};
use state::reg::{riscv::{RvCsrEnum, Trap}, RegfileIo};

use crate::emu::Emu;

#[derive(Default, Clone, Copy)] 
pub enum WbCtrl{
    #[default]
    DontCare,

    WriteGpr,
    Jump,
    Csr,
}

#[derive(Default, Clone)]
pub struct ToWbStage {
    pub pc: u32,
    pub result: u32,
    pub csr_rdata: u32,

    pub gpr_waddr: u8,
    pub csr_waddr: u16,

    pub wb_ctrl: WbCtrl,

    pub trap: Option<Trap>,
}

impl Emu {
    pub fn write_back_rv32i(&mut self, stage: ToWbStage) -> ProcessResult<u32> {
        let regfile = &mut self.states.regfile;
        let pc = stage.pc;
        let mut next_pc = pc.wrapping_add(4);

        if let Some(trap) = stage.trap {
            regfile.write_csr(RvCsrEnum::MEPC.into(), pc)?;
            regfile.write_csr(RvCsrEnum::MCAUSE.into(), trap as u32)?;

            next_pc = regfile.read_csr(RvCsrEnum::MTVEC.into())?;

            if trap == Trap::Ebreak {
                (self.callback.trap)(); // just for now
                return Err(ProcessError::Recoverable);
            }

            regfile.write_pc(next_pc);
            
            return Ok(next_pc);
        }

        match stage.wb_ctrl {
            WbCtrl::WriteGpr => {
                regfile.write_gpr(stage.gpr_waddr.into(), stage.result)?;
            }

            WbCtrl::Jump => {
                regfile.write_gpr(stage.gpr_waddr.into(), next_pc)?;
                next_pc = stage.result;
            }

            WbCtrl::Csr => {
                regfile.write_gpr(stage.gpr_waddr.into(), stage.csr_rdata)?;
                regfile.write_csr(stage.csr_waddr.into(), stage.result)?;
            }

            WbCtrl::DontCare => {
                log_error!(format!("WbCtrl::None should not be used at pc: {:#08x}", pc));
                return Err(ProcessError::Recoverable);
            },
        }

        regfile.write_pc(next_pc);
        
        Ok(next_pc)
    }
}
