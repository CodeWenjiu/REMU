use remu_macro::log_err;
use remu_utils::{ProcessError, ProcessResult};
use state::cache::{CacheBase, ICacheData};

use crate::emu::{isa::riscv::BasicStageMsg, EmuHardware};

use super::ToIdStage;

#[derive(Default, Debug, Clone)]
pub struct ToIfStage {
    pub pc: u32,
    npc: u32,
}

impl ToIfStage {
    pub fn new(pc: u32, npc: u32) -> Self {
        Self { pc, npc }
    }
}

impl EmuHardware {
    pub fn instruction_fetch_rv32i(&mut self, stage: ToIfStage) -> ProcessResult<ToIdStage> {
        let msg = BasicStageMsg { pc: stage.pc, npc: stage.npc, trap: None };

        self.times.instruction_fetch += 1;

        let inst = if let Some(icache) = self.states.cache.icache.as_mut() {

            if let Some(data) = icache.read(msg.pc) {
                self.times.instruction_cache_hit += 1;
                let block_index = icache.table.get_block_num(msg.pc);
                data[block_index as usize].inst
            } else {
                // burst transfer
                let mut inst = 0;

                let base_addr = msg.pc & !((1 << icache.base_bits) - 1);
                let mut replace_data = vec![];
                
                for i in 0..icache.block_num {
                    let access_addr = base_addr + i * 4;
                    let data = log_err!(
                        self.states.mmu.read(access_addr, state::mmu::Mask::Word),
                        ProcessError::Recoverable
                    )?;

                    if access_addr == msg.pc {
                        inst = data;
                    }

                    replace_data.push(ICacheData { inst: data });
                }

                icache.replace(msg.pc, replace_data);

                inst
            }
        } else {
            log_err!(
                self.states.mmu.read(msg.pc, state::mmu::Mask::Word),
                ProcessError::Recoverable
            )?
        };

        Ok(ToIdStage {
            msg,
            inst,
        })
    }
}
