use remu_utils::{ProcessError, ProcessResult};
use state::reg::{riscv::RvCsrEnum, RegfileIo};

use crate::emu::Emu;

use super::super::Trap;

#[derive(Default)]
pub struct ToWbStage {
    pub pc: u32,
    pub next_pc: u32,
    pub gpr_wmsg: (u8, u32),
    pub csr_wmsg: (bool, u32, u32),
    pub trap: Option<Trap>,
}

impl Emu {
    pub fn write_back_rv32i(&mut self, stage: ToWbStage) -> ProcessResult<()> {
        let regfile = &mut self.states.regfile;
        let pc = stage.pc;

        if let Some(trap) = stage.trap {
            regfile.write_csr(RvCsrEnum::MEPC.into(), pc)?;
            regfile.write_csr(RvCsrEnum::MCAUSE.into(), trap as u32)?;
            
            if trap == Trap::Ebreak {
                (self.callback.trap)();
            }

            return Err(ProcessError::Recoverable);
        }

        let gpr_wmsg = stage.gpr_wmsg;
        regfile.write_gpr(gpr_wmsg.0.into(), gpr_wmsg.1)?;

        let csr_wmsg = stage.csr_wmsg;
        if csr_wmsg.0 {
            regfile.write_csr(csr_wmsg.1.into(), csr_wmsg.2)?;
        }

        regfile.write_pc(stage.next_pc);

        Ok(())
    }
}
