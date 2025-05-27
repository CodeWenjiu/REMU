use remu_macro::log_err;
use logger::Logger;
use remu_utils::{ProcessError, ProcessResult};
use state::mmu::Mask;

use crate::emu::Emu;

use super::{ToWbStage, WbCtrl, };

#[derive(Default, Clone, Copy)]
pub enum LsCtrl {
    #[default]
    Lb,
    Lh,
    Lw,
    Lbu,
    Lhu,
    Sb,
    Sh,
    Sw,
}

#[derive(Default)]
pub struct ToLsStage {
    pub pc: u32,
    pub ls_ctrl: LsCtrl,
    pub gpr_waddr: u8,

    pub addr: u32,
    pub data: u32,
}

impl Emu {
    pub fn load_store_rv32i(&mut self, stage: ToLsStage) -> ProcessResult<ToWbStage> {
        let result;

        let pc = stage.pc;
        let mut gpr_waddr = stage.gpr_waddr;

        let ctrl = stage.ls_ctrl;
        let addr = stage.addr;
        let data: u32 = stage.data;

        let is_difftest_skip;
        let mmu = &mut self.states.mmu;

        match ctrl {
            LsCtrl::Lb => {
                let read_result = log_err!(mmu.read(addr, Mask::Byte), ProcessError::Recoverable)?;
                is_difftest_skip = read_result.0;
                result = read_result.1 as i8 as u32;
            }

            LsCtrl::Lh => {
                let read_result = log_err!(mmu.read(addr, Mask::Half), ProcessError::Recoverable)?;
                is_difftest_skip = read_result.0;
                result = read_result.1 as i16 as u32;
            }

            LsCtrl::Lw => {
                let read_result = log_err!(mmu.read(addr, Mask::Word), ProcessError::Recoverable)?;
                is_difftest_skip = read_result.0;
                result = read_result.1;
            }

            LsCtrl::Lbu => {
                let read_result = log_err!(mmu.read(addr, Mask::Byte), ProcessError::Recoverable)?;
                is_difftest_skip = read_result.0;
                result = read_result.1;
            }

            LsCtrl::Lhu => {
                let read_result = log_err!(mmu.read(addr, Mask::Half), ProcessError::Recoverable)?;
                is_difftest_skip = read_result.0;
                result = read_result.1;
            }

            LsCtrl::Sb => {
                gpr_waddr = 0;
                result = 0;
                is_difftest_skip = log_err!(mmu.write(addr, data, Mask::Byte), ProcessError::Recoverable)?;
            }

            LsCtrl::Sh => {
                gpr_waddr = 0;
                result = 0;
                is_difftest_skip = log_err!(mmu.write(addr, data, Mask::Half), ProcessError::Recoverable)?;
            }

            LsCtrl::Sw => {
                gpr_waddr = 0;
                result = 0;
                is_difftest_skip = log_err!(mmu.write(addr, data, Mask::Word), ProcessError::Recoverable)?;
            }
        }

        if is_difftest_skip {
            (self.callback.difftest_skip)();
        };

        Ok(ToWbStage { pc, result, csr_rdata: 0, gpr_waddr, csr_waddr: 0, wb_ctrl: WbCtrl::default(), trap: None })
    }
}
