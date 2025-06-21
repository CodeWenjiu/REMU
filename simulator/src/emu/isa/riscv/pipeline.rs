use std::fmt::Display;

use remu_macro::log_error;
use remu_utils::{ProcessError, ProcessResult};
use state::model::BaseStageCell;

use crate::emu::{isa::riscv::{backend::{ToAlStage, ToLsStage, ToWbStage}, frontend::{IsOutStage, ToIdStage, ToIfStage, ToIsStage}}, Emu};
use owo_colors::OwoColorize;

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
    ls_ena_pre: bool,
    ls_ena: bool,
    pipeline_pc: u32,
}

impl Display for PipelineStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PipelineStage {{\n")?;
        
        // Handle each stage separately
        let stages_data = [
            ("if_id", format!("{:08x?}", self.if_id.0), self.if_id.1),
            ("id_is", format!("{:08x?}", self.id_is.0), self.id_is.1),
            ("is_ls", format!("{:08x?}", self.is_ls.0), self.is_ls.1),
            ("is_al", format!("{:08x?}", self.is_al.0), self.is_al.1),
            ("ex_wb", format!("{:08x?}", self.ex_wb.0), self.ex_wb.1),
        ];
        
        for (i, (name, data, valid)) in stages_data.iter().enumerate() {
            let colored = if *valid { 
                data.style(owo_colors::Style::new().green())
            } else { 
                data.style(owo_colors::Style::new().blue())
            };
            let comma = if i == stages_data.len() - 1 { "" } else { "," };
            write!(f, "  {}: {}{}\n", name, colored, comma)?;
        }
        
        write!(f, "}}")
    }
}

impl Display for Pipeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.stages.fmt(f)
    }
}

impl Pipeline {
    pub fn new(reset_vector: u32) -> Self {
        Self {
            stages: PipelineStage::new(),
            if_ena: false,
            ls_ena_pre: false,
            ls_ena: false,
            pipeline_pc: reset_vector,
        }
    }

    fn is_gpr_raw(&self, rs1_addr: u8, rs2_addr: u8) -> bool {
        let (to_wb, wb_valid) = &self.stages.ex_wb;
        let (to_al, al_valid) = &self.stages.is_al;
        let (to_ls, ls_valid) = &self.stages.is_ls;
        let (to_is, is_valid) = &self.stages.id_is;

        let conflict_gpr = |rd: u8| {
            (rd != 0) && 
            ((rd == rs1_addr) || (rd == rs2_addr))
        };

        (*wb_valid && conflict_gpr(to_wb.gpr_waddr)) ||
        (*al_valid && conflict_gpr(to_al.gpr_waddr)) ||
        (*ls_valid && conflict_gpr(to_ls.gpr_waddr)) ||
        (*is_valid && conflict_gpr(to_is.gpr_waddr))
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

        self.pipeline_pc != next_pc
    }

    fn flush_if_need(&mut self, next_pc: u32) -> bool {
        let need = self.is_flush_need(next_pc);

        if need {
            self.stages.is_ls.1 = false;
            self.stages.is_al.1 = false;
            self.stages.id_is.1 = false;
            self.stages.if_id.1 = false;

            self.if_ena = false;
            
            self.pipeline_pc = next_pc;
        }

        need
    }
}

impl Emu {
    fn self_pipeline_branch_predict(&self) -> u32 {
        let result = self.pipeline.pipeline_pc;

        result
    }

    fn self_pipeline_branch_predict_update(&mut self) {
        self.pipeline.pipeline_pc += 4; // need to be implemented
    }

    pub fn self_step_cycle_pipeline(&mut self) -> ProcessResult<()> {
        self.self_pipeline_ifena();

        if self.pipeline.ls_ena_pre {
            self.self_pipeline_lsena();
            self.pipeline.ls_ena_pre = false;
        }

        if self.pipeline.stages.is_ls.1 {
            self.self_pipeline_lsena_pre(); 
        }

        self.self_step_cycle_pipeline_without_enable(None)
    }

    pub fn self_step_cycle_pipeline_without_enable(&mut self, skip: Option<u32>) -> ProcessResult<()> {
        self.times.cycles += 1;

        let wb_msg = self.self_pipeline_update(skip)?;

        self.states.pipe_state.update()?;

        if let Some((pc, next_pc, inst)) = wb_msg {
            (self.callback.instruction_complete)(pc, next_pc, inst)?;
        }

        Ok(())
    }

    pub fn self_pipeline_update(&mut self, skip: Option<u32>) -> ProcessResult<Option<(u32, u32, u32)>> {
        let mut to_id = None;
        let mut to_is = None;
        let mut to_ls = None;
        let mut to_al = None;

        enum ToWb {
            FromLs(ToWbStage),
            FromAl(ToWbStage),
        }
        let mut to_wb: Option<ToWb> = None;

        let mut wb_msg = None;
        let mut next_pc = None;

        let mut gpr_raw_hazard = false;

        if self.pipeline.if_ena {
            let predict_pc = self.self_pipeline_branch_predict(); // need to be implemented
            
            let _id = self.instruction_fetch_rv32i(ToIfStage { pc: predict_pc })?;

            to_id = Some(_id);
        }

        if self.pipeline.stages.if_id.1 {
            let if_id = self.pipeline.stages.if_id.0.clone();
            let id_is = self.instruction_decode(if_id)?;
            to_is = Some(id_is);

            gpr_raw_hazard = self.pipeline.is_gpr_raw(id_is.rs1_addr, id_is.rs2_addr);
        }

        if self.pipeline.stages.id_is.1 {
            let id_is = self.pipeline.stages.id_is.0.clone();
            let is_out = self.instruction_issue(id_is)?;

            match is_out {
                IsOutStage::LS(is_ls) => {
                    to_ls = Some(is_ls);
                },
                IsOutStage::AL(is_al) => {
                    to_al = Some(is_al);
                },
            }
        }

        if self.pipeline.stages.is_al.1 {
            let is_al = self.pipeline.stages.is_al.0.clone();
            to_wb = Some(ToWb::FromAl(self.arithmetic_logic_rv32(is_al)?));
        } else if self.pipeline.stages.is_ls.1 {
            let is_ls = self.pipeline.stages.is_ls.0.clone();
            if self.pipeline.ls_ena {
                to_wb = Some(ToWb::FromLs(
                    if let Some(skip_val) = skip {
                        self.load_store_rv32i_with_skip(is_ls, skip_val)?
                    } else {
                        self.load_store_rv32i(is_ls)?
                    }
                ));
            }
        }

        if self.pipeline.stages.ex_wb.1 {
            let ex_wb = self.pipeline.stages.ex_wb.0.clone();
            next_pc = Some(self.write_back_rv32i(ex_wb)?);
        }

        if let Some(next_pc) = next_pc {
            let (pc, inst) = self.states.pipe_state.get()?; // need to used to check

            if pc != self.pipeline.stages.ex_wb.0.pc {
                log_error!(format!("EX 2 WB PC mismatch: fetched {:#08x}, expected {:#08x}", pc, self.pipeline.stages.ex_wb.0.pc));
                return Err(ProcessError::Recoverable);
            }

            self.pipeline.stages.ex_wb.1 = false;

            wb_msg = Some((pc, next_pc, inst));

            if self.pipeline.flush_if_need(next_pc) {
                self.states.pipe_state.flush();
                return Ok(wb_msg);
            }
        }

        if let Some(to_wb) = to_wb {
            match to_wb {
                ToWb::FromAl(from_al) => {
                    let (pc, _inst) = self.states.pipe_state.fetch(BaseStageCell::IsAl)?; // need to used to check
                    if pc != from_al.pc {
                        log_error!(format!("AL 2 WB PC mismatch: fetched {:#08x}, expected {:#08x}", pc, from_al.pc));
                        return Err(ProcessError::Recoverable);
                    }

                    self.pipeline.stages.is_al.1 = false;
                    self.pipeline.stages.ex_wb.0 = from_al;
                    self.pipeline.stages.ex_wb.1 = true;

                    self.states.pipe_state.trans(BaseStageCell::IsAl, BaseStageCell::ExWb)?;
                }

                ToWb::FromLs(from_ls) => {
                    let (pc, _inst) = self.states.pipe_state.fetch(BaseStageCell::IsLs)?; // need to used to check
                    if pc != from_ls.pc {
                        log_error!(format!("LS 2 WB PC mismatch: fetched {:#08x}, expected {:#08x}", pc, from_ls.pc));
                        return Err(ProcessError::Recoverable);
                    }

                    self.pipeline.stages.is_ls.1 = false;
                    self.pipeline.stages.ex_wb.0 = from_ls;
                    self.pipeline.stages.ex_wb.1 = true;

                    self.states.pipe_state.trans(BaseStageCell::IsLs, BaseStageCell::ExWb)?;
                },
            }
            self.times.instructions += 1;
        }

        if let Some(to_ls) = to_ls {
            let (pc, _inst) = self.states.pipe_state.fetch(BaseStageCell::IdIs)?; // need to used to check
            if pc != to_ls.pc {
                log_error!(format!("IS 2 LS PC mismatch: fetched {:#08x}, expected {:#08x}", pc, to_ls.pc));
                return Err(ProcessError::Recoverable);
            }

            self.pipeline.stages.id_is.1 = false;
            self.pipeline.ls_ena = false;

            self.pipeline.stages.is_ls.0 = to_ls;
            self.pipeline.stages.is_ls.1 = true;

            self.states.pipe_state.trans(BaseStageCell::IdIs, BaseStageCell::IsLs)?;

        }

        if self.pipeline.stages.is_ls.1 {
            return Ok(wb_msg); // LS Hazardx
        }

        if let Some(to_al) = to_al {
            let (pc, _inst) = self.states.pipe_state.fetch(BaseStageCell::IdIs)?; // need to used to check
            if pc != to_al.pc {
                log_error!(format!("IS 2 AL PC mismatch: fetched {:#08x}, expected {:#08x}", pc, to_al.pc));
                return Err(ProcessError::Recoverable);
            }

            self.pipeline.stages.id_is.1 = false;

            self.pipeline.stages.is_al.0 = to_al;
            self.pipeline.stages.is_al.1 = true;

            self.states.pipe_state.trans(BaseStageCell::IdIs, BaseStageCell::IsAl)?;
        }

        if gpr_raw_hazard {
            return Ok(wb_msg);
        }

        if let Some(to_is) = to_is {
            self.pipeline.stages.if_id.1 = false;
            
            self.pipeline.stages.id_is.0 = to_is;
            self.pipeline.stages.id_is.1 = true;
            
            self.states.pipe_state.trans(BaseStageCell::IfId, BaseStageCell::IdIs)?;
        }
        
        if let Some(to_id) = to_id {
            self.pipeline.if_ena = false;

            self.pipeline.stages.if_id.0 = to_id;
            self.pipeline.stages.if_id.1 = true;

            self.states.pipe_state.send((to_id.pc, to_id.inst), BaseStageCell::IfId)?;
            
            self.self_pipeline_branch_predict_update();
        }

        Ok(wb_msg)

        // let mut pre_to_is = None;
        // let mut raw_hazard = false;

        // if self.pipeline.stages.if_id.1 {
        //     let mit = self.pipeline.stages.if_id.0.clone();
        //     let mit = self.instruction_decode(mit)?;
        //     raw_hazard = self.pipeline.is_gpr_raw(mit.rs1_addr, mit.rs2_addr);
        //     pre_to_is = Some(mit);
        // } 

        // let (to_wb, wb_valid) = &self.pipeline.stages.ex_wb;
        
        // let mut wb_msg = None;

        // if *wb_valid {
        //     let (pc, inst) = self.states.pipe_state.get()?;

        //     let next_pc = self.write_back_rv32i(to_wb.clone())?;

        //     wb_msg = Some((pc, next_pc, inst));

        //     self.pipeline.stages.ex_wb.1 = false;

        //     if self.pipeline.flush_if_need(next_pc) {
        //         self.states.pipe_state.flush();
        //     }

        //     self.times.instructions += 1;
        // }

        // let (to_ls, ls_valid) = &self.pipeline.stages.is_ls;
        
        // if *ls_valid {
        //     let ls_ena = self.pipeline.ls_ena;
            
        //     if ls_ena {
        //         let (pc, _inst) = self.states.pipe_state.fetch(BaseStageCell::IsLs)?; // need to used to check

        //         let to_wb = if let Some(skip_val) = skip {
        //             self.load_store_rv32i_with_skip(to_ls.clone(), skip_val)?
        //         } else {
        //             self.load_store_rv32i(to_ls.clone())?
        //         };

        //         if pc != to_wb.pc {
        //             log_error!(format!("LS 2 WB PC mismatch: fetched {:#08x}, expected {:#08x}", pc, to_wb.pc));
        //             return Err(ProcessError::Recoverable);
        //         }

        //         self.pipeline.stages.is_ls.1 = false;
        //         self.pipeline.stages.ex_wb.0 = to_wb;
        //         self.pipeline.stages.ex_wb.1 = true;

        //         self.states.pipe_state.trans(BaseStageCell::IsLs, BaseStageCell::ExWb)?;

        //         self.pipeline.ls_ena = false; 
        //     }

        //     return Ok(wb_msg); // Ls Hazard
        // }

        // let (to_al, al_valid) = &self.pipeline.stages.is_al;

        // if *al_valid {
        //     let (pc, _inst) = self.states.pipe_state.fetch(BaseStageCell::IsAl)?; // need to used to check

        //     let to_wb = self.arithmetic_logic_rv32(to_al.clone())?;

        //     if pc != to_wb.pc {
        //         log_error!(format!("AL 2 WB PC mismatch: fetched {:#08x}, expected {:#08x}", pc, to_wb.pc));
        //         return Err(ProcessError::Recoverable);
        //     }

        //     self.pipeline.stages.is_al.1 = false;
        //     self.pipeline.stages.ex_wb.0 = to_wb;
        //     self.pipeline.stages.ex_wb.1 = true;

        //     self.states.pipe_state.trans(BaseStageCell::IsAl, BaseStageCell::ExWb)?;
        // }

        // let (to_is, is_valid) = &self.pipeline.stages.id_is;

        // if *is_valid {
        //     let (pc, _inst) = self.states.pipe_state.fetch(BaseStageCell::IdIs)?; // need to used to check

        //     let to_ex = self.instruction_issue(to_is.clone())?;

        //     self.pipeline.stages.id_is.1 = false;
        //     match to_ex {
        //         IsOutStage::AL(to_al) => {
        //             if pc != to_al.pc {
        //                 log_error!(format!("IS 2 AL PC mismatch: fetched {:#08x}, expected {:#08x}", pc, to_al.pc));
        //                 return Err(ProcessError::Recoverable);
        //             }

        //             self.pipeline.stages.is_al.0 = to_al;
        //             self.pipeline.stages.is_al.1 = true;

        //             self.states.pipe_state.trans(BaseStageCell::IdIs, BaseStageCell::IsAl)?;
        //         },
        //         IsOutStage::LS(to_ls) => {
        //             if pc != to_ls.pc {
        //                 log_error!(format!("IS 2 LS PC mismatch: fetched {:#08x}, expected {:#08x}", pc, to_ls.pc));
        //                 return Err(ProcessError::Recoverable);
        //             }

        //             self.pipeline.stages.is_ls.0 = to_ls;
        //             self.pipeline.stages.is_ls.1 = true;

        //             self.states.pipe_state.trans(BaseStageCell::IdIs, BaseStageCell::IsLs)?;
        //         },
        //     }
        // }

        // let (to_id, id_valid) = &self.pipeline.stages.if_id; 

        // if raw_hazard {
        //     return Ok(wb_msg);
        // }

        // if *id_valid {
        //     let (pc, _inst) = self.states.pipe_state.fetch(BaseStageCell::IfId)?; // need to used to check

        //     let to_is = pre_to_is.unwrap();

        //     if pc != to_id.pc {
        //         log_error!(format!("IF 2 ID PC mismatch: fetched {:#08x}, expected {:#08x}", pc, to_id.pc));
        //         return Err(ProcessError::Recoverable);
        //     }

        //     self.pipeline.stages.id_is.0 = to_is;
        //     self.pipeline.stages.id_is.1 = true;

        //     self.pipeline.stages.if_id.1 = false;

        //     self.states.pipe_state.trans(BaseStageCell::IfId, BaseStageCell::IdIs)?;
        // }

        // if self.pipeline.if_ena {
        //     let predict_pc = self.self_pipeline_branch_predict(); // need to be implemented
    
        //     let to_id = self.instruction_fetch_rv32i(ToIfStage{pc: predict_pc})?;

        //     self.pipeline.stages.if_id.0 = to_id;
        //     self.pipeline.stages.if_id.1 = true;

        //     self.states.pipe_state.send((predict_pc, to_id.inst), BaseStageCell::IfId)?;

        //     self.pipeline.if_ena = false;
        // }

        // return Ok(wb_msg);
    }

    pub fn self_pipeline_ifena(&mut self) {
        self.pipeline.if_ena = true;
        (self.callback.instruction_fetch)();
    }

    pub fn self_pipeline_lsena_pre(&mut self) {
        self.pipeline.ls_ena_pre = true;
    }

    pub fn self_pipeline_lsena(&mut self) {
        self.pipeline.ls_ena = true;
        (self.callback.load_store)();
    }
}