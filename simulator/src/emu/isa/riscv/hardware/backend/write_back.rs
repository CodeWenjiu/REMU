use remu_macro::{log_err, log_error};
use remu_utils::{ProcessError, ProcessResult};
use state::{cache::BRMsg, reg::{riscv::{RvCsrEnum, Trap}, RegfileIo}};

use crate::emu::{isa::riscv::BasicStageMsg, EmuHardware};

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
    pub msg: BasicStageMsg,

    pub result: u32,
    pub csr_rdata: u32,

    pub br: BRMsg,

    pub gpr_waddr: u8,
    pub csr_waddr: u16,

    pub wb_ctrl: WbCtrl,
}

#[derive(Default, Clone, Debug, PartialEq)]
pub enum WbControl {
    #[default]
    BPError,
    BPRight,
    Trap,
}

#[derive(Default, Clone, Debug)]
pub struct Wbout {
    pub pc: u32,
    pub next_pc: u32,
    pub wb_ctrl: WbControl,
    pub wb_bypass: (u8, u32),
    pub br: BRMsg,
}

impl EmuHardware {
    pub fn write_back_rv32i(&mut self, stage: ToWbStage) -> ProcessResult<Wbout> {
        let mut out = Wbout {
            pc: stage.msg.pc,
            next_pc: 0,
            wb_ctrl: WbControl::BPRight,
            wb_bypass: (0, 0),
            br: stage.br
        };

        let regfile = &mut self.states.regfile;
        let pc: u32 = stage.msg.pc;
        let mut next_pc = pc.wrapping_add(4);

        if let Some(trap) = stage.msg.trap {
            next_pc = if trap == Trap::Ebreak {
                (self.callback.yield_)(); // just for now
                return Err(ProcessError::Recoverable);
            } else if trap == Trap::Mret {
                log_err!(regfile.read_csr(RvCsrEnum::MEPC.into()), ProcessError::Recoverable)?
            } else {
                regfile.trap(pc, trap as u32)?
            };

            regfile.write_pc(next_pc);
            
            out.next_pc = next_pc;

            out.wb_ctrl = WbControl::Trap;

            return Ok(out);
        }

        out.wb_bypass.0 = stage.gpr_waddr;

        match stage.wb_ctrl {
            WbCtrl::WriteGpr => {
                out.wb_bypass.1 = stage.result;
            }

            WbCtrl::Jump => {
                out.wb_bypass.1 = next_pc;
                next_pc = stage.result;
                self.times.branched_cycles += 1;
            }

            WbCtrl::Csr => {
                out.wb_bypass.1 = stage.csr_rdata;
                log_err!(regfile.write_csr(stage.csr_waddr.into(), stage.result), ProcessError::Recoverable)?;
            }

            WbCtrl::DontCare => {
                log_error!(format!("WbCtrl::None should not be used at pc: {:#08x}", pc));
                return Err(ProcessError::Recoverable);
            },
        }

        regfile.write_gpr(out.wb_bypass.0.into(), out.wb_bypass.1)?;
        regfile.write_pc(next_pc);
            
        out.next_pc = next_pc;
        if stage.msg.npc != next_pc {
            out.wb_ctrl = WbControl::BPError;
        }
        
        Ok(out)
    }
}
