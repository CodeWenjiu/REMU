use remu_macro::{log_err, log_error};
use remu_utils::{ProcessError, ProcessResult};
use state::reg::{riscv::{RvCsrEnum, Trap}, RegfileIo};

use crate::emu::Emu;

#[derive(Default, Clone, Copy, Debug)] 
pub enum WbCtrl{
    #[default]
    DontCare,

    WriteGpr,
    Jump,
    Csr,
}

#[derive(Default, Clone, Debug)]
pub struct ToWbStage {
    pub pc: u32,
    pub result: u32,
    pub csr_rdata: u32,

    pub gpr_waddr: u8,
    pub csr_waddr: u16,

    pub wb_ctrl: WbCtrl,

    pub trap: Option<Trap>,
}

#[derive(Default, Clone, Debug)]
pub struct IsOut {
    pub next_pc: u32,
    pub wb_bypass: (u8, u32),
}

impl Emu {
    pub fn write_back_rv32i(&mut self, stage: ToWbStage) -> ProcessResult<IsOut> {
        let mut out = IsOut {
            next_pc: 0,
            wb_bypass: (0, 0),
        };

        let regfile = &mut self.states.regfile;
        let pc = stage.pc;
        let mut next_pc = pc.wrapping_add(4);

        if let Some(trap) = stage.trap {
            log_err!(regfile.write_csr(RvCsrEnum::MEPC.into(), pc), ProcessError::Recoverable)?;
            log_err!(regfile.write_csr(RvCsrEnum::MCAUSE.into(), trap as u32), ProcessError::Recoverable)?;

            next_pc = log_err!(regfile.read_csr(RvCsrEnum::MTVEC.into()), ProcessError::Recoverable)?;

            if trap == Trap::Ebreak {
                (self.callback.trap)(); // just for now
                return Err(ProcessError::Recoverable);
            }

            regfile.write_pc(next_pc);
            
            out.next_pc = next_pc;

            return Ok(out);
        }

        out.wb_bypass.0 = stage.gpr_waddr;

        match stage.wb_ctrl {
            WbCtrl::WriteGpr => {
                out.wb_bypass.1 = stage.result;
                regfile.write_gpr(stage.gpr_waddr.into(), stage.result)?;
            }

            WbCtrl::Jump => {
                out.wb_bypass.1 = next_pc;
                regfile.write_gpr(stage.gpr_waddr.into(), next_pc)?;
                next_pc = stage.result;
            }

            WbCtrl::Csr => {
                out.wb_bypass.1 = stage.result;
                regfile.write_gpr(stage.gpr_waddr.into(), stage.csr_rdata)?;
                log_err!(regfile.write_csr(stage.csr_waddr.into(), stage.result), ProcessError::Recoverable)?;
            }

            WbCtrl::DontCare => {
                log_error!(format!("WbCtrl::None should not be used at pc: {:#08x}", pc));
                return Err(ProcessError::Recoverable);
            },
        }

        regfile.write_pc(next_pc);
            
        out.next_pc = next_pc;
        
        Ok(out)
    }
}
