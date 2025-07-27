use remu_macro::{log_err, log_error};
use remu_utils::{ProcessError, ProcessResult};
use state::{cache::{BRMsg, CacheBase, DCacheData}, mmu::Mask};

use crate::emu::{isa::riscv::BasicStageMsg, EmuHardware};

use super::{ToWbStage, WbCtrl, };

#[derive(Default, Clone, Copy, Debug)]
pub enum LsCtrl {
    #[default]
    DontCare,
    
    Lb,
    Lh,
    Lw,
    Lbu,
    Lhu,
    Sb,
    Sh,
    Sw,
}

#[derive(Default, Clone, Debug)]
pub struct ToLsStage {
    pub msg: BasicStageMsg,

    pub ls_ctrl: LsCtrl,
    pub gpr_waddr: u8,

    pub addr: u32,
    pub data: u32,
}

impl EmuHardware {
    // just for pipeline ref difftest
    pub fn load_store_rv32i_with_skip(&mut self, stage: ToLsStage, skip_val: u32) -> ProcessResult<ToWbStage> {
        let result;

        let msg = stage.msg;
        let mut gpr_waddr = stage.gpr_waddr;

        let ctrl = stage.ls_ctrl;

        match ctrl {
            LsCtrl::Lb => {
                result = skip_val;
            }

            LsCtrl::Lh => {
                result = skip_val;
            }

            LsCtrl::Lw => {
                result = skip_val;
            }

            LsCtrl::Lbu => {
                result = skip_val;
            }

            LsCtrl::Lhu => {
                result = skip_val;
            }

            LsCtrl::Sb => {
                gpr_waddr = 0;
                result = 0;
            }

            LsCtrl::Sh => {
                gpr_waddr = 0;
                result = 0;
            }

            LsCtrl::Sw => {
                gpr_waddr = 0;
                result = 0;
            }

            LsCtrl::DontCare => {
                log_error!(format!("LsCtrl::None should not be used at pc: {:#08x}", msg.pc));
                return Err(ProcessError::Recoverable);
            },
        }

        let to_msg = BasicStageMsg {
            pc: msg.pc,
            npc: msg.pc.wrapping_add(4),
            trap: None,
        }; // wb's next pc is always pc + 4, in chisel will automatically optimize out

        Ok(ToWbStage { msg: to_msg, result, csr_rdata: 0, br: BRMsg { br_type: false, br_dir: false }, gpr_waddr, csr_waddr: 0, wb_ctrl: WbCtrl::WriteGpr })
    }

    fn try_write_cache(&mut self, addr: u32, data: u32, mask: Mask) -> ProcessResult<()> {
        if let Some(dcache) = self.states.cache.dcache.as_mut() {
            if let Err(()) = dcache.write(addr, data, mask) {
                // burst update
                let base_addr = addr & !((1 << dcache.base_bits) - 1);
                let mut replace_data = vec![];

                for i in 0..dcache.block_num {
                    let access_addr = base_addr + i * 4;
                    let data = log_err!(
                        self.states.mmu.read(access_addr, state::mmu::Mask::Word),
                        ProcessError::Recoverable
                    )?;

                    replace_data.push(DCacheData{data});
                }

                if let Some(writeback_data) = dcache.replace(addr, replace_data) {
                    for (i, data_block) in writeback_data.1.iter().enumerate() {
                        let access_addr = writeback_data.0 + i as u32 * 4;
                        log_err!(self.states.mmu.write(access_addr, data_block.data, Mask::Word), ProcessError::Recoverable)?;
                    }
                }

                dcache.write(addr, data, mask).unwrap(); // safe to unwrap, because replace will always succeed
            } else {
                self.times.data_cache_hit += 1;
            }
        } else {
            log_err!(self.states.mmu.write(addr, data, mask), ProcessError::Recoverable)?;
        }

        Ok(())
    }

    fn try_read_cache(&mut self, addr: u32, mask: Mask) -> ProcessResult<u32> {
        let read_result = if let Some(dcache) = self.states.cache.dcache.as_mut() {
            if let Some(data) = dcache.load_data(addr, mask) {
                self.times.data_cache_hit += 1;
                data
            } else {
                // burst update
                let base_addr = addr & !((1 << dcache.base_bits) - 1);

                let mut replace_data = vec![];
                for i in 0..dcache.block_num {
                    let access_addr = base_addr + i * 4;
                    let data = log_err!(
                        self.states.mmu.read(access_addr, state::mmu::Mask::Word),
                        ProcessError::Recoverable
                    )?;

                    replace_data.push(DCacheData{data});
                }

                if let Some(writeback_data) = dcache.replace(addr, replace_data) {
                    for (i, data_block) in writeback_data.1.iter().enumerate() {
                        let access_addr = writeback_data.0 + i as u32 * 4;
                        log_err!(self.states.mmu.write(access_addr, data_block.data, Mask::Word), ProcessError::Recoverable)?;
                    }
                }

                dcache.load_data(addr, mask).unwrap() // safe to unwrap, because data will always be found after replace
            }
        } else {
            log_err!(self.states.mmu.read(addr, mask), ProcessError::Recoverable)?
        };

        Ok(read_result)
    }

    pub fn load_store_rv32i(&mut self, stage: ToLsStage) -> ProcessResult<ToWbStage> {
        let msg = stage.msg;
        
        let result;
        
        let ctrl = stage.ls_ctrl;
        let addr = stage.addr;
        let data: u32 = stage.data;
        
        let mmu = &mut self.states.mmu;
        let is_difftest_skip = log_err!(mmu.is_dev(addr), ProcessError::Recoverable)?;
        
        let mask = match ctrl {
            LsCtrl::Lb | LsCtrl::Lbu | LsCtrl::Sb => Mask::Byte,
            LsCtrl::Lh | LsCtrl::Lhu | LsCtrl::Sh => Mask::Half,
            LsCtrl::Lw | LsCtrl::Sw => Mask::Word,
            LsCtrl::DontCare => {
                log_error!(format!("LsCtrl::None should not be used at pc: {:#08x}", msg.pc));
                return Err(ProcessError::Recoverable);
            },
        };

        let gpr_waddr = match ctrl {
            LsCtrl::Sb | LsCtrl::Sh | LsCtrl::Sw => 0,
            _ => stage.gpr_waddr,
        };

        let read_result = match ctrl {
            LsCtrl::Lb | LsCtrl::Lbu | LsCtrl::Lh | LsCtrl::Lhu | LsCtrl::Lw => {
                if is_difftest_skip {
                    log_err!(mmu.read(addr, mask), ProcessError::Recoverable)?
                } else {
                    self.try_read_cache(addr, mask)?
                }
            }

            _ => {
                if is_difftest_skip {
                    log_err!(mmu.write(addr, data, mask), ProcessError::Recoverable)?;
                } else {
                    self.try_write_cache(addr, data, mask)?;
                }
                0
            }
        };

        result = match ctrl {
            LsCtrl::Lb => {
                read_result as i8 as u32
            }

            LsCtrl::Lh => {
                read_result as i16 as u32
            }

            _ => read_result
        };

        if is_difftest_skip {
            (self.callback.difftest_skip)(result);
        };

        let to_msg = BasicStageMsg {
            pc: msg.pc,
            npc: msg.npc,
            trap: None,
        };

        self.times.load_store += 1;

        Ok(ToWbStage { msg: to_msg, result, csr_rdata: 0, br: BRMsg { br_type: false, br_dir: false }, gpr_waddr, csr_waddr: 0, wb_ctrl: WbCtrl::WriteGpr })
    }
}
