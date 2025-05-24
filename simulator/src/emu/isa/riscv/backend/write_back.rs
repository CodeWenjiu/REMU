use remu_utils::{ProcessError, ProcessResult};
use state::reg::{riscv::RvCsrEnum, RegfileIo};

use crate::emu::Emu;

use super::super::Trap;

#[derive(Default)]
pub struct ToWbStage {
    pub pc: u32,
    pub next_pc: u32,
    pub gpr_wmsg: (u8, u32),
    pub csr_wmsg: Option<(u32, u32)>,
    pub trap: Option<Trap>,
}

#[derive(Default)] 
pub enum WbMove{
    #[default]
    WriteGpr,
    Jump,
    Csr,
    Trap,
}

#[derive(Default)]
pub struct ToWbStagen {
    pub pc: u32,
    pub result: u32,
    pub csr_rdata: u32,

    pub gpr_waddr: u8,
    pub csr_waddr: u16,

    pub move_type: WbMove,

    pub trap: Trap,
}

impl Emu {
    pub fn write_back_rv32in(&mut self, stage: ToWbStagen) -> ProcessResult<u32> {
        let regfile = &mut self.states.regfile;
        let pc = stage.pc;
        let mut next_pc = pc.wrapping_add(4);

        match stage.move_type {
            WbMove::WriteGpr => {
                regfile.write_gpr(stage.gpr_waddr.into(), stage.result)?;
            }

            WbMove::Jump => {
                regfile.write_gpr(stage.gpr_waddr.into(), next_pc)?;
                next_pc = stage.result;
            }

            WbMove::Csr => {
                regfile.write_gpr(stage.gpr_waddr.into(), stage.csr_rdata)?;
                regfile.write_csr(stage.csr_waddr.into(), stage.result)?;
            }

            WbMove::Trap => {
                regfile.write_csr(RvCsrEnum::MEPC.into(), pc)?;
                regfile.write_csr(RvCsrEnum::MCAUSE.into(), stage.trap as u32)?;

                if stage.trap == Trap::Ebreak {
                    (self.callback.trap)(); // just for now
                    return Err(ProcessError::Recoverable);
                }

                next_pc = regfile.read_csr(RvCsrEnum::MTVEC.into())?;
            }
        }

        regfile.write_pc(next_pc);
        
        Ok(next_pc)
    }

    pub fn write_back_rv32i(&mut self, stage: ToWbStage) -> ProcessResult<u32> {
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

        stage.csr_wmsg.map(|(csr, val)| regfile.write_csr(csr.into(), val));

        regfile.write_pc(stage.next_pc);
        Ok(stage.next_pc)
    }
}
