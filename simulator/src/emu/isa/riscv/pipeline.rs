use remu_macro::log_error;
use logger::Logger;
use remu_utils::{ProcessError, ProcessResult};
use state::model::BaseStageCell;

use crate::emu::{extract_bits, isa::riscv::{backend::{ToAlStage, ToLsStage, ToWbStage}, frontend::{IsOutStage, ToIdStage, ToIfStage, ToIsStage}}, Emu};

struct PipelineStage {
    ex_wb: (ToWbStage, bool),
    is_ls: (ToLsStage, bool),
    is_al: (ToAlStage, bool),
    id_is: (ToIsStage, bool),
    if_id: (ToIdStage, bool),
}

impl PipelineStage {
    pub fn new() -> Self {
        Self {
            ex_wb: (ToWbStage::default(), false),
            is_ls: (ToLsStage::default(), false),
            is_al: (ToAlStage::default(), false),
            id_is: (ToIsStage::default(), false),
            if_id: (ToIdStage::default(), false),
        }
    }
}

pub struct Pipeline {
    stages: PipelineStage,
    if_ena: bool,
    ls_ena: bool,
    pipeline_pc: u32,
}

impl Pipeline {
    pub fn new(reset_vector: u32) -> Self {
        Self {
            stages: PipelineStage::new(),
            if_ena: false,
            ls_ena: false,
            pipeline_pc: reset_vector,
        }
    }

    fn is_gpr_raw(&self) -> bool {
        let (to_wb, wb_valid) = &self.stages.ex_wb;
        let (to_al, al_valid) = &self.stages.is_al;
        let (to_ls, ls_valid) = &self.stages.is_ls;
        let (to_is, is_valid) = &self.stages.id_is;

        let inst = self.stages.if_id.0.inst;

        let rs1_addr = extract_bits(inst, 15..19) as u8;
        let rs2_addr = extract_bits(inst, 20..24) as u8;

        if *wb_valid {
            if rs1_addr == to_wb.gpr_waddr || rs2_addr == to_wb.gpr_waddr {
                return true;
            }
        }

        if *al_valid {
            if rs1_addr == to_al.gpr_waddr || rs2_addr == to_al.gpr_waddr {
                return true;
            }
        }

        if *ls_valid {
            if rs1_addr == to_ls.gpr_waddr || rs2_addr == to_ls.gpr_waddr {
                return true;
            }
        }

        if *is_valid {
            if rs1_addr == to_is.gpr_waddr || rs2_addr == to_is.gpr_waddr {
                return true;
            }
        }

        false
    }

    fn is_flush_need(&self, next_pc: u32) -> bool {
        let (to_al, al_valid) = &self.stages.is_al;
        let (to_ls, ls_valid) = &self.stages.is_ls;
        let (to_is, is_valid) = &self.stages.id_is;
        let (to_id, id_valid) = &self.stages.if_id;

        if *al_valid {
            return to_al.pc != next_pc;
        }

        if *ls_valid {
            return to_ls.pc != next_pc;
        }

        if *is_valid {
            return to_is.pc != next_pc;
        }

        if *id_valid {
            return to_id.pc != next_pc;
        }

        false
    }

    fn flush_if_need(&mut self, next_pc: u32) -> bool {
        let need = self.is_flush_need(next_pc);

        if need {
            self.stages.ex_wb.1 = false;
            self.stages.is_ls.1 = false;
            self.stages.is_al.1 = false;
            self.stages.id_is.1 = false;
            self.stages.if_id.1 = false;

            self.pipeline_pc = next_pc;
        }

        need
    }
}

impl Emu {
    fn self_pipeline_branch_predict(&mut self) -> ProcessResult<u32> {
        let result = self.pipeline.pipeline_pc;

        self.pipeline.pipeline_pc += 4;

        Ok(result)
    }

    pub fn self_step_cycle_pipeline(&mut self) -> ProcessResult<()> {
        self.self_pipeline_ifena();
        self.self_pipeline_lsena();

        self.self_step_cycle_pipeline_without_enable()
    }

    pub fn self_step_cycle_pipeline_without_enable(&mut self) -> ProcessResult<()> {
        self.self_pipeline_update()?;
        self.states.pipe_state.update()?;

        self.times.cycles += 1;

        Ok(())
    }

    pub fn self_pipeline_update(&mut self) -> ProcessResult<()> {
        let (to_wb, wb_valid) = &self.pipeline.stages.ex_wb;

        if *wb_valid {
            let (pc, inst) = self.states.pipe_state.get()?;

            let next_pc = self.write_back_rv32i(to_wb.clone())?;

            if self.pipeline.flush_if_need(next_pc) {
                self.states.pipe_state.flush();
                (self.callback.instruction_complete)(pc, next_pc, inst)?;
                return Ok(());
            }

            self.pipeline.stages.ex_wb.1 = false;
            
            self.times.instructions += 1;
            (self.callback.instruction_complete)(pc, next_pc, inst)?;
        }

        let ls_ena = self.pipeline.ls_ena;
        if ls_ena {
            let (to_ls, ls_valid) = &self.pipeline.stages.is_ls;

            if *ls_valid {
                let (pc, _inst) = self.states.pipe_state.fetch(BaseStageCell::IsLs)?; // need to used to check

                let to_wb = self.load_store_rv32i(to_ls.clone())?;

                if pc != to_wb.pc {
                    log_error!(format!("LS 2 WB PC mismatch: fetched {:#08x}, expected {:#08x}", pc, to_wb.pc));
                    return Err(ProcessError::Recoverable);
                }

                self.pipeline.stages.is_ls.1 = false;
                self.pipeline.stages.ex_wb.0 = to_wb;
                self.pipeline.stages.ex_wb.1 = true;

                self.states.pipe_state.trans(BaseStageCell::IsLs, BaseStageCell::ExWb)?;
            }
        }

        let (to_al, al_valid) = &self.pipeline.stages.is_al;

        if *al_valid {
            let (pc, _inst) = self.states.pipe_state.fetch(BaseStageCell::IsAl)?; // need to used to check

            let to_wb = self.arithmetic_logic_rv32(to_al.clone())?;

            if pc != to_wb.pc {
                log_error!(format!("AL 2 WB PC mismatch: fetched {:#08x}, expected {:#08x}", pc, to_wb.pc));
                return Err(ProcessError::Recoverable);
            }

            self.pipeline.stages.is_al.1 = false;
            self.pipeline.stages.ex_wb.0 = to_wb;
            self.pipeline.stages.ex_wb.1 = true;

            self.states.pipe_state.trans(BaseStageCell::IsAl, BaseStageCell::ExWb)?;
        }

        let (to_is, is_valid) = &self.pipeline.stages.id_is;

        if *is_valid {
            let (pc, _inst) = self.states.pipe_state.fetch(BaseStageCell::IdIs)?; // need to used to check

            let to_ex = self.instruction_issue(to_is.clone())?;

            self.pipeline.stages.id_is.1 = false;
            match to_ex {
                IsOutStage::AL(to_al) => {
                    if pc != to_al.pc {
                        log_error!(format!("IS 2 AL PC mismatch: fetched {:#08x}, expected {:#08x}", pc, to_al.pc));
                        return Err(ProcessError::Recoverable);
                    }

                    self.pipeline.stages.is_al.0 = to_al;
                    self.pipeline.stages.is_al.1 = true;

                    self.states.pipe_state.trans(BaseStageCell::IdIs, BaseStageCell::IsAl)?;
                },
                IsOutStage::LS(to_ls) => {
                    if pc != to_ls.pc {
                        log_error!(format!("IS 2 LS PC mismatch: fetched {:#08x}, expected {:#08x}", pc, to_ls.pc));
                        return Err(ProcessError::Recoverable);
                    }

                    self.pipeline.stages.is_ls.0 = to_ls;
                    self.pipeline.stages.is_ls.1 = true;

                    self.states.pipe_state.trans(BaseStageCell::IdIs, BaseStageCell::IsLs)?;
                },
            }
        }

        let (to_id, id_valid) = &self.pipeline.stages.if_id;

        if self.pipeline.is_gpr_raw() {
            return Ok(());
        }

        if *id_valid {
            let (pc, _inst) = self.states.pipe_state.fetch(BaseStageCell::IfId)?; // need to used to check

            let to_is = self.instruction_decode(to_id.clone())?;

            if pc != to_id.pc {
                log_error!(format!("IF 2 ID PC mismatch: fetched {:#08x}, expected {:#08x}", pc, to_id.pc));
                return Err(ProcessError::Recoverable);
            }

            self.pipeline.stages.id_is.0 = to_is;
            self.pipeline.stages.id_is.1 = true;

            self.states.pipe_state.trans(BaseStageCell::IfId, BaseStageCell::IdIs)?;
        }

        if self.pipeline.if_ena {
            let predict_pc = self.self_pipeline_branch_predict()?; // need to be implemented
    
            let to_id = self.instruction_fetch_rv32i(ToIfStage{pc: predict_pc})?;

            self.pipeline.stages.if_id.0 = to_id;
            self.pipeline.stages.if_id.1 = true;

            self.states.pipe_state.send((predict_pc, to_id.inst), BaseStageCell::IfId)?;

            self.pipeline.if_ena = false;
        }

        Ok(())
    }

    pub fn self_pipeline_ifena(&mut self) {
        self.pipeline.if_ena = true;
    }

    pub fn self_pipeline_lsena(&mut self) {
        self.pipeline.ls_ena = true;
    }
}