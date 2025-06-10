use remu_utils::ProcessResult;
use state::model::BaseStageCell;

use crate::emu::{isa::riscv::{backend::{ToAlStage, ToLsStage, ToWbStage}, frontend::{ToIdStage, ToIfStage, ToIsStage}}, Emu};

struct PipelineStage {
    ex_wb: (ToWbStage, bool),
    is_ls: (ToLsStage, bool, bool),
    is_al: (ToAlStage, bool),
    id_is: (ToIsStage, bool),
    if_id: (ToIdStage, bool),
}

impl PipelineStage {
    pub fn new() -> Self {
        Self {
            ex_wb: (ToWbStage::default(), false),
            is_ls: (ToLsStage::default(), false, false),
            is_al: (ToAlStage::default(), false),
            id_is: (ToIsStage::default(), false),
            if_id: (ToIdStage::default(), false),
        }
    }
}

pub struct Pipeline {
    stages: PipelineStage,
}

impl Pipeline {
    pub fn new() -> Self {
        Self {
            stages: PipelineStage::new(),
        }
    }
}

impl Emu {
    pub fn self_step_cycle_pipeline(&mut self) -> ProcessResult<()> {
        let (to_wb, wb_valid) = &self.pipeline.stages.ex_wb;

        if *wb_valid {
            let (pc, inst) = self.states.pipe_state.get()?;

            let next_pc = self.write_back_rv32i(to_wb.clone())?;

            self.pipeline.stages.ex_wb.1 = false;
            
            (self.callback.instruction_complete)(pc, next_pc, inst)?;
        }

        let (to_ls, ls_valid, ls_ena) = &self.pipeline.stages.is_ls;

        if *ls_ena && *ls_valid {
            self.states.pipe_state.trans(BaseStageCell::IsLs, BaseStageCell::ExWb)?;

            let to_wb = self.load_store_rv32i(to_ls.clone())?;
            
            self.pipeline.stages.ex_wb.0 = to_wb;
            self.pipeline.stages.ex_wb.1 = true;

            self.pipeline.stages.is_ls.1 = false;
            self.pipeline.stages.is_ls.2 = false;
        }

        Ok(())
    }

    pub fn self_pipeline_fetch_instruction(&mut self, pc: u32) -> ProcessResult<()> {
        self.pipeline.stages.if_id.1 = true;
        let to_id = self.instruction_fetch_rv32i(ToIfStage{pc})?;

        self.states.pipe_state.send((pc, to_id.inst), BaseStageCell::IfId)?;
        self.pipeline.stages.if_id.0 = to_id;

        Ok(())
    }
}