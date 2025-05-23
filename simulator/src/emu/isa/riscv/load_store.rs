use remu_macro::log_err;
use logger::Logger;
use remu_utils::{ProcessError, ProcessResult};
use state::mmu::Mask;

use crate::emu::Emu;

use super::{ToWbStage, RV32ILS};

#[derive(Default)]
pub struct ToLsStage {
    pub pc: u32,
    pub inst: RV32ILS,
    pub rd_addr: u8,

    pub addr: u32,
    pub data: u32,
}

impl Emu {
    pub fn load_store_rv32i(&mut self, stage: ToLsStage) -> ProcessResult<ToWbStage> {
        let pc = stage.pc;
        let next_pc = pc.wrapping_add(4);
        let mut rd_addr = stage.rd_addr;

        let inst = stage.inst;
        let addr = stage.addr;
        let data = stage.data;

        let mut is_difftest_skip = false;
        let mut rd_val = 0;

        let mmu = &mut self.states.mmu;

        match inst {
            RV32ILS::Lb => {
                let read_result = log_err!(mmu.read(addr, Mask::Byte), ProcessError::Recoverable)?;
                is_difftest_skip = read_result.0;
                rd_val = read_result.1 as i8 as u32;
            }

            RV32ILS::Lh => {
                let read_result = log_err!(mmu.read(addr, Mask::Half), ProcessError::Recoverable)?;
                is_difftest_skip = read_result.0;
                rd_val = read_result.1 as i16 as u32;
            }

            RV32ILS::Lw => {
                let read_result = log_err!(mmu.read(addr, Mask::Word), ProcessError::Recoverable)?;
                is_difftest_skip = read_result.0;
                rd_val = read_result.1;
            }

            RV32ILS::Lbu => {
                let read_result = log_err!(mmu.read(addr, Mask::Byte), ProcessError::Recoverable)?;
                is_difftest_skip = read_result.0;
                rd_val = read_result.1;
            }

            RV32ILS::Lhu => {
                let read_result = log_err!(mmu.read(addr, Mask::Half), ProcessError::Recoverable)?;
                is_difftest_skip = read_result.0;
                rd_val = read_result.1;
            }

            RV32ILS::Sb => {
                rd_addr = 0;
                if log_err!(mmu.write(addr, data, Mask::Byte), ProcessError::Recoverable)? == true {
                    (self.callback.difftest_skip)();
                }
            }

            RV32ILS::Sh => {
                rd_addr = 0;
                if log_err!(mmu.write(addr, data, Mask::Half), ProcessError::Recoverable)? == true {
                    (self.callback.difftest_skip)();
                }
            }

            RV32ILS::Sw => {
                rd_addr = 0;
                if log_err!(mmu.write(addr, data, Mask::Word), ProcessError::Recoverable)? == true {
                    (self.callback.difftest_skip)();
                }
            }
        };

        if is_difftest_skip {
            (self.callback.difftest_skip)();
        };

        let gpr_wmsg = (rd_addr, rd_val);

        Ok(ToWbStage {
            pc,
            next_pc,
            gpr_wmsg,
            csr_wmsg: (false, 0, 0),
            trap: None,
        })
    }
}
