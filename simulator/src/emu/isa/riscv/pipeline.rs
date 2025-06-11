use remu_utils::ProcessResult;
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
}

impl Emu {
    fn self_pipeline_branch_predict(&mut self) -> ProcessResult<u32> {
        let result = self.pipeline.pipeline_pc;

        self.pipeline.pipeline_pc += 4;

        Ok(result)
    }

    pub fn self_step_cycle_pipeline(&mut self) -> ProcessResult<()> {
        let predict_pc = self.self_pipeline_branch_predict()?; // need to be implemented

        self.self_pipeline_ifena();
        self.self_pipeline_lsena();
        self.self_pipeline_update(predict_pc)?;
        self.states.pipe_state.update()?;

        Ok(())
    }

    pub fn self_pipeline_update(&mut self, predict_pc: u32) -> ProcessResult<()> {
        let (to_wb, wb_valid) = &self.pipeline.stages.ex_wb;

        if *wb_valid {
            let (pc, inst) = self.states.pipe_state.get()?;

            let next_pc = self.write_back_rv32i(to_wb.clone())?;

            self.pipeline.stages.ex_wb.1 = false;
            
            (self.callback.instruction_complete)(pc, next_pc, inst)?;
        }

        let ls_ena = self.pipeline.ls_ena;
        if ls_ena {
            let (to_ls, ls_valid) = &self.pipeline.stages.is_ls;

            if *ls_valid {
                let (_pc, _inst) = self.states.pipe_state.fetch(BaseStageCell::IsLs)?; // need to used to check

                let to_wb = self.load_store_rv32i(to_ls.clone())?;

                self.pipeline.stages.is_ls.1 = false;
                self.pipeline.stages.ex_wb.0 = to_wb;
                self.pipeline.stages.ex_wb.1 = true;

                self.states.pipe_state.trans(BaseStageCell::IsLs, BaseStageCell::ExWb)?;
            }
        }

        let (to_al, al_valid) = &self.pipeline.stages.is_al;

        if *al_valid {
            let (_pc, _inst) = self.states.pipe_state.fetch(BaseStageCell::IsAl)?; // need to used to check

            let to_wb = self.arithmetic_logic_rv32(to_al.clone())?;

            self.pipeline.stages.is_al.1 = false;
            self.pipeline.stages.ex_wb.0 = to_wb;
            self.pipeline.stages.ex_wb.1 = true;

            self.states.pipe_state.trans(BaseStageCell::IsAl, BaseStageCell::ExWb)?;
        }

        let (to_is, is_valid) = &self.pipeline.stages.id_is;

        if *is_valid {
            let (_pc, _inst) = self.states.pipe_state.fetch(BaseStageCell::IdIs)?; // need to used to check

            let to_ex = self.instruction_issue(to_is.clone())?;

            self.pipeline.stages.id_is.1 = false;
            match to_ex {
                IsOutStage::AL(to_al) => {
                    self.pipeline.stages.is_al.0 = to_al;
                    self.pipeline.stages.is_al.1 = true;

                    self.states.pipe_state.trans(BaseStageCell::IdIs, BaseStageCell::IsAl)?;
                },
                IsOutStage::LS(to_ls) => {
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
            let (_pc, _inst) = self.states.pipe_state.fetch(BaseStageCell::IfId)?; // need to used to check

            let to_is = self.instruction_decode(to_id.clone())?;

            self.pipeline.stages.id_is.0 = to_is;
            self.pipeline.stages.id_is.1 = true;

            self.states.pipe_state.trans(BaseStageCell::IfId, BaseStageCell::IdIs)?;
        }

        if self.pipeline.if_ena {
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