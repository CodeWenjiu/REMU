use remu_utils::{ProcessError, ProcessResult};
use state::reg::RegfileIo;

use crate::emu::{extract_bits, isa::riscv::{backend::{AlCtrl, ToAlStage, ToLsStage, WbCtrl}, InstMsg, InstPattern, Trap}, sig_extend, Emu, InstructionSetFlags};

use super::{
    super::{ImmType, Priv, Zicsr, RISCV, RV32I, RV32IAL, RV32ILS, RV32M, },
    RV32_IAL_PATTERN_ITER, RV32_ILS_PATTERN_ITER, RV32_M_PATTERN_ITER, RV_PRIV_PATTERN_ITER, RV_ZICSR_PATTERN_ITER
};

#[derive(Default, Clone, Copy)]
pub struct ToIdStage {
    pub pc: u32,
    pub inst: u32,
}

#[derive(Default)]
pub struct IdOutStage {
    pub pc: u32,
    pub inst: RISCV,
    pub msg: InstMsg, 
}

pub enum IdOutStagen {
    AL(ToAlStage),
    LS(ToLsStage),
}

impl Default for IdOutStagen {
    fn default() -> Self {
        Self::AL(ToAlStage::default())
    }
}

impl Emu {
    /// Decode an instruction as RV32I
    fn rv32_i_al_decode(inst: u32) -> Option<(RV32IAL, ImmType)> {
        for (opcode, imm_type, (mask, value)) in RV32_IAL_PATTERN_ITER {
            if (inst & mask) == *value {
                return Some((*opcode, *imm_type));
            }
        }
        None
    }

    fn rv32_i_ls_decode(inst: u32) -> Option<(RV32ILS, ImmType)> {
        for (opcode, imm_type, (mask, value)) in RV32_ILS_PATTERN_ITER {
            if (inst & mask) == *value {
                return Some((*opcode, *imm_type));
            }
        }
        None
    }

    fn rv32_i_decode(inst: u32) -> Option<(RV32I, ImmType)> {
        if let Some((opcode, imm_type)) = Self::rv32_i_al_decode(inst) {
            return Some((RV32I::AL(opcode), imm_type));
        } else if let Some((opcode, imm_type)) = Self::rv32_i_ls_decode(inst) {
            return Some((RV32I::LS(opcode), imm_type));
        }

        None
    }

    /// Decode an instruction as RV32M
    fn rv32_m_decode(inst: u32) -> Option<(RV32M, ImmType)> {
        for (opcode, imm_type, (mask, value)) in RV32_M_PATTERN_ITER {
            if (inst & mask) == *value {
                return Some((*opcode, *imm_type));
            }
        }
        None
    }

    /// Decode an instruction as Zicsr
    fn rv_zicsr_decode(inst: u32) -> Option<(Zicsr, ImmType)> {
        for (opcode, imm_type, (mask, value)) in RV_ZICSR_PATTERN_ITER {
            if (inst & mask) == *value {
                return Some((*opcode, *imm_type));
            }
        }
        None
    }

    /// Decode an instruction as privileged
    fn rv_priv_decode(inst: u32) -> Option<(Priv, ImmType)> {
        for (opcode, imm_type, (mask, value)) in RV_PRIV_PATTERN_ITER {
            if (inst & mask) == *value {
                return Some((*opcode, *imm_type));
            }
        }
        None
    }

    /// Decode an instruction based on the enabled instruction set extensions
    fn isa_decode(&self, inst: u32) -> Option<(RISCV, ImmType)> {
        let isa = self.instruction_set;
        
        // Try to decode as RV32I first (most common)
        if isa.contains(InstructionSetFlags::RV32I) {
            if let Some((opcode, imm_type)) = Self::rv32_i_decode(inst) {
                return Some((RISCV::RV32I(opcode), imm_type));
            }
        }

        if isa.contains(InstructionSetFlags::RV32E) {
            if let Some((opcode, imm_type)) = Self::rv32_i_decode(inst) {
                return Some((RISCV::RV32E(opcode), imm_type));
            }
        }

        // Try to decode as RV32M
        if isa.contains(InstructionSetFlags::RV32M) {
            if let Some((opcode, imm_type)) = Self::rv32_m_decode(inst) {
                return Some((RISCV::RV32M(opcode), imm_type));
            }
        }

        // Try to decode as Zicsr
        if isa.contains(InstructionSetFlags::ZICSR) {
            if let Some((opcode, imm_type)) = Self::rv_zicsr_decode(inst) {
                return Some((RISCV::Zicsr(opcode), imm_type));
            }
        }

        // Try to decode as privileged
        if isa.contains(InstructionSetFlags::PRIV) {
            if let Some((opcode, imm_type)) = Self::rv_priv_decode(inst) {
                return Some((RISCV::Priv(opcode), imm_type));
            }
        }

        // Failed to decode
        None
    }

    /// Extract and sign-extend immediate value based on instruction type
    fn get_imm(inst: u32, imm_type: ImmType) -> u32 {
        match imm_type {
            // I-type: Load, ALU immediate, JALR
            ImmType::I => {
                let range = 20..31;
                let imm = extract_bits(inst, range.clone());
                sig_extend(imm, range.end as u8 - range.start as u8 + 1)
            },
            
            // S-type: Store
            ImmType::S => {
                let imm = (extract_bits(inst, 25..31) << 5) | extract_bits(inst, 7..11);
                sig_extend(imm, 12)
            },
            
            // B-type: Branch
            ImmType::B => {
                let imm = (extract_bits(inst, 31..31) << 12) | 
                          (extract_bits(inst, 25..30) << 5) | 
                          (extract_bits(inst, 8..11) << 1) | 
                          (extract_bits(inst, 7..7) << 11);
                sig_extend(imm, 13)
            },
            
            // U-type: LUI, AUIPC
            ImmType::U => {
                extract_bits(inst, 12..31) << 12
            },
            
            // J-type: JAL
            ImmType::J => {
                let imm = (extract_bits(inst, 31..31) << 20) | 
                          (extract_bits(inst, 12..19) << 12) | 
                          (extract_bits(inst, 20..20) << 11) | 
                          (extract_bits(inst, 21..30) << 1);
                sig_extend(imm, 21)
            },
            
            // R-type: Register-register operations (no immediate)
            ImmType::R => 0,
            
            // N-type: No operands
            ImmType::N => 0,
        }
    }

    /// Decode an instruction into its components
    pub fn decode(&self, msg: ToIdStage) -> ProcessResult<InstPattern> {
        let pc = msg.pc;
        let inst = msg.inst;
        if let Some((opcode, imm_type)) = self.isa_decode(inst) {
            // Extract register fields
            let rs1 = extract_bits(inst, 15..19) as u8;
            let rs2 = extract_bits(inst, 20..24) as u8;
            let rd = extract_bits(inst, 7..11) as u8;
            
            // Extract immediate value
            let imm = Self::get_imm(inst, imm_type);

            let regfile = &self.states.regfile;
            let rs1: u32 = regfile.read_gpr(rs1.into()).map_err(|_| ProcessError::Recoverable)?;
            let rs2: u32 = regfile.read_gpr(rs2.into()).map_err(|_| ProcessError::Recoverable)?;

            // Create instruction pattern
            Ok(InstPattern::new(opcode, rs1, rs2, rd, imm))
        } else {
            // Decoding failed
            (self.callback.decode_failed)(pc, inst);
            Err(ProcessError::Recoverable)
        }
    }

    // pub fn instruction_issue(&mut self, msg: InstPattern) -> ProcessResult<IdOutStagen> {
    //     let inst = msg.name;

    //     match inst {
    //         RISCV::RV32I(RV32I::AL(inst)) => {
    //             let stage = ToAlStage {
    //                 pc: msg.pc,
    //                 inst,
    //                 msg,
    //             };

    //             Ok(IdOutStagen::AL(stage))
    //         }

    //         _ => unreachable!()
    //     }
    // }

    pub fn instruction_decode(&mut self, stage: ToIdStage) -> ProcessResult<IdOutStagen> {
        let inst = self.decode(stage)?;

        let msg = inst.msg;

        let rs1_val = msg.rs1;
        let rs2_val: u32 = msg.rs2;
        let mut gpr_waddr = msg.rd_addr;
        let imm = msg.imm;

        let inst = inst.name;

        let pc = stage.pc;
        let mut srca = 0;
        let mut srcb = 0;
        let mut ctrl = AlCtrl::Add;
        let mut wb_ctrl = WbCtrl::WriteGpr;

        let mut trap = None;

        match inst {
            RISCV::RV32I(RV32I::AL(inst)) => {
                match inst {
                    RV32IAL::Lui => {
                        srca = imm;
                    }

                    RV32IAL::Auipc => {
                        srca = pc;
                        srcb = imm;
                    }

                    RV32IAL::Jal => {
                        wb_ctrl = WbCtrl::Jump;
                        srca = pc;
                        srcb = imm;
                    }

                    RV32IAL::Jalr => {
                        wb_ctrl = WbCtrl::Jump;
                        srca = rs1_val;
                        srcb = imm;
                    }

                    // logic work should move to IS stage in the future
                    RV32IAL::Beq => {
                        wb_ctrl = WbCtrl::Jump;
                        gpr_waddr = 0; // there is no rd need to link address to register
                        srca = pc;
                        srcb = if rs1_val == rs2_val { imm } else { 4 };
                    }

                    RV32IAL::Bne => {
                        wb_ctrl = WbCtrl::Jump;
                        gpr_waddr = 0; 
                        srca = pc;
                        srcb = if rs1_val != rs2_val { imm } else { 4 };
                    }

                    RV32IAL::Blt => {
                        wb_ctrl = WbCtrl::Jump;
                        gpr_waddr = 0; 
                        srca = pc;
                        srcb = if (rs1_val as i32) < (rs2_val as i32) { imm } else { 4 };
                    }

                    RV32IAL::Bge => {
                        wb_ctrl = WbCtrl::Jump;
                        gpr_waddr = 0; 
                        srca = pc;
                        srcb = if (rs1_val as i32) >= (rs2_val as i32) { imm } else { 4 };
                    }

                    RV32IAL::Bltu => {
                        wb_ctrl = WbCtrl::Jump;
                        gpr_waddr = 0; 
                        srca = pc;
                        srcb = if rs1_val < rs2_val { imm } else { 4 };
                    }

                    RV32IAL::Bgeu => {
                        wb_ctrl = WbCtrl::Jump;
                        gpr_waddr = 0; 
                        srca = pc;
                        srcb = if rs1_val >= rs2_val { imm } else { 4 };
                    }

                    RV32IAL::Addi => {
                        srca = rs1_val;
                        srcb = imm;
                    }

                    RV32IAL::Slti => {
                        srca = if (rs1_val as i32) < (imm as i32) { 1 } else { 0 };
                        srcb = 0;
                    }

                    RV32IAL::Sltiu => {
                        srca = if rs1_val < imm { 1 } else { 0 };
                        srcb = 0;
                    }

                    RV32IAL::Xori => {
                        ctrl = AlCtrl::Xor;
                        srca = rs1_val;
                        srcb = imm;
                    }

                    RV32IAL::Ori => {
                        ctrl = AlCtrl::Or;
                        srca = rs1_val;
                        srcb = imm;
                    }

                    RV32IAL::Andi => {
                        ctrl = AlCtrl::And;
                        srca = rs1_val;
                        srcb = imm;
                    }

                    RV32IAL::Slli => {
                        ctrl = AlCtrl::Sll;
                        srca = rs1_val;
                        srcb = imm;
                    }

                    RV32IAL::Srli => {
                        ctrl = AlCtrl::Srl;
                        srca = rs1_val;
                        srcb = imm;
                    }

                    RV32IAL::Srai => {
                        ctrl = AlCtrl::Sra;
                        srca = rs1_val;
                        srcb = imm;
                    }

                    RV32IAL::Add => {
                        srca = rs1_val;
                        srcb = rs2_val;
                    }

                    RV32IAL::Sub => {
                        ctrl = AlCtrl::Sub;
                        srca = rs1_val;
                        srcb = rs2_val;
                    }

                    RV32IAL::Xor => {
                        ctrl = AlCtrl::Xor;
                        srca = rs1_val;
                        srcb = rs2_val;
                    }

                    RV32IAL::Or => {
                        ctrl = AlCtrl::Or;
                        srca = rs1_val;
                        srcb = rs2_val;
                    }

                    RV32IAL::And => {
                        ctrl = AlCtrl::And;
                        srca = rs1_val;
                        srcb = rs2_val;
                    }

                    RV32IAL::Slt => {
                        srca = if (rs1_val as i32) < (rs2_val as i32) { 1 } else { 0 };
                        srcb = 0;
                    }

                    RV32IAL::Sltu => {
                        srca = if rs1_val < rs2_val { 1 } else { 0 };
                        srcb = 0;
                    }

                    RV32IAL::Sll => {
                        ctrl = AlCtrl::Sll;
                        srca = rs1_val;
                        srcb = rs2_val;
                    }

                    RV32IAL::Srl => {
                        ctrl = AlCtrl::Srl;
                        srca = rs1_val;
                        srcb = rs2_val;
                    }

                    RV32IAL::Sra => {
                        ctrl = AlCtrl::Sra;
                        srca = rs1_val;
                        srcb = rs2_val;
                    }

                    RV32IAL::Ecall => {
                        trap = Some(Trap::EcallM);
                    }
        
                    RV32IAL::Ebreak => {
                        trap = Some(Trap::Ebreak);
                    }
        
                    RV32IAL::Fence => {
                        gpr_waddr = 0; // do nothing for now
                    }
                }
            }

            RISCV::RV32I(RV32I::LS(inst)) => {
                return Ok(IdOutStagen::LS(ToLsStage {
                    pc,
                    inst,
                    rd_addr: gpr_waddr,

                    addr: rs1_val.wrapping_add(imm),
                    data: rs2_val,
                }));
            }

            RISCV::RV32M(inst) => {
                match inst {
                    RV32M::Mul => {
                        ctrl = AlCtrl::Mul;
                        srca = rs1_val;
                        srcb = rs2_val;
                    }

                    RV32M::Mulh => {
                        ctrl = AlCtrl::Mulh;
                        srca = rs1_val;
                        srcb = rs2_val;
                    }

                    RV32M::Mulhsu => {
                        ctrl = AlCtrl::Mulhsu;
                        srca = rs1_val;
                        srcb = rs2_val;
                    }

                    RV32M::Mulhu => {
                        ctrl = AlCtrl::Mulhu;
                        srca = rs1_val;
                        srcb = rs2_val;
                    }

                    RV32M::Div => {
                        ctrl = AlCtrl::Div;
                        srca = rs1_val;
                        srcb = rs2_val;
                    }

                    RV32M::Divu => {
                        ctrl = AlCtrl::Divu;
                        srca = rs1_val;
                        srcb = rs2_val;
                    }

                    RV32M::Rem => {
                        ctrl = AlCtrl::Rem;
                        srca = rs1_val;
                        srcb = rs2_val;
                    }

                    RV32M::Remu => {
                        ctrl = AlCtrl::Remu;
                        srca = rs1_val;
                        srcb = rs2_val;
                    }
                }
            }

            _ => unreachable!()
        };

        Ok(IdOutStagen::AL(ToAlStage { pc, srca, srcb, ctrl, wb_ctrl, gpr_waddr, trap }))
    }
}
