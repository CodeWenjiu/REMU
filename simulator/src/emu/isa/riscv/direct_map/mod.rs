use comfy_table::{Cell, Color, Table};
use remu_macro::{log_err, log_todo};
use remu_utils::{ProcessError, ProcessResult};
use state::{mmu::Mask, reg::{riscv::{RvCsrEnum, Trap}, RegfileIo}, States};

use crate::{emu::{extract_bits, isa::riscv::instruction::{DecodeResult, ImmGet}, InstructionSetFlags}, SimulatorCallback};

use super::instruction::{InstMsg, InstPattern, Priv, Zicsr, RISCV, RV32I, RV32IAL, RV32ILS, RV32M};

pub struct EmuDmTimes {
    pub instructions: u64,
}

impl EmuDmTimes {
    pub fn print(&self) {
        let mut table = Table::new();

        table
            .add_row(vec![
                Cell::new("Instructions").fg(Color::Blue),
            ])
            .add_row(vec![
                Cell::new(self.instructions.to_string()).fg(Color::Green),
            ]);

        println!("{table}");
    }
}

/// RISC-V Emulator implementation
pub struct EmuDirectMap {
    /// Enabled instruction set extensions
    pub instruction_set: InstructionSetFlags,
    
    /// Emulator state (registers, memory, etc.)
    pub states: States,
    
    /// Callbacks for emulator events
    pub callback: SimulatorCallback,

    /// Emulator times
    pub times: EmuDmTimes,
}

impl ImmGet for EmuDirectMap {}

impl EmuDirectMap {
    pub fn new(
        instruction_set: InstructionSetFlags,
        states: States,
        callback: SimulatorCallback,
    ) -> Self {
        Self {
            instruction_set,
            states,
            callback,
            times: EmuDmTimes {
                instructions: 0,
            },
        }
    }
}

impl EmuDirectMap {
    fn rv32_i_execute(&mut self, name: RV32I, mut msg: InstMsg) -> ProcessResult<u32> {
        let regfile = &mut self.states.regfile;
        let rs1: u32 = regfile.read_gpr(msg.rs1.into()).map_err(|_| ProcessError::Recoverable)?;
        let rs2: u32 = regfile.read_gpr(msg.rs2.into()).map_err(|_| ProcessError::Recoverable)?;
        
        let mut rd_val: u32 = 0;

        let pc: u32 = regfile.read_pc();
        let mut next_pc = pc.wrapping_add(4);

        let imm: u32 = msg.imm;

        let mmu = &mut self.states.mmu;

        match name {
            RV32I::AL(inst) => {
                match inst {
                    RV32IAL::Lui => {
                        rd_val = imm;
                    }
        
                    RV32IAL::Auipc => {
                        rd_val = pc.wrapping_add(imm);
                    }
        
                    RV32IAL::Jal => {
                        rd_val = next_pc;
                        next_pc = pc.wrapping_add(imm);
                    }
        
                    RV32IAL::Jalr => {
                        rd_val = next_pc;
                        next_pc = rs1.wrapping_add(imm);
                    }
        
                    RV32IAL::Beq => {
                        msg.rd_addr = 0;
                        if rs1 == rs2 {
                            next_pc = pc.wrapping_add(imm);
                        }
                    }
        
                    RV32IAL::Bne => {
                        msg.rd_addr = 0;
                        if rs1 != rs2 {
                            next_pc = pc.wrapping_add(imm);
                        }
                    }
        
                    RV32IAL::Blt => {
                        msg.rd_addr = 0;
                        if (rs1 as i32) < (rs2 as i32) {
                            next_pc = pc.wrapping_add(imm);
                        }
                    }
        
                    RV32IAL::Bge => {
                        msg.rd_addr = 0;
                        if (rs1 as i32) >= (rs2 as i32) {
                            next_pc = pc.wrapping_add(imm);
                        }
                    }
        
                    RV32IAL::Bltu => {
                        msg.rd_addr = 0;
                        if rs1 < rs2 {
                            next_pc = pc.wrapping_add(imm);
                        }
                    }
        
                    RV32IAL::Bgeu => {
                        msg.rd_addr = 0;
                        if rs1 >= rs2 {
                            next_pc = pc.wrapping_add(imm);
                        }
                    }

                    RV32IAL::Addi => {
                        rd_val = rs1.wrapping_add(imm);
                    }
        
                    RV32IAL::Slti => {
                        rd_val = if (rs1 as i32) < (imm as i32) { 1 } else { 0 };
                    }
        
                    RV32IAL::Sltiu => {
                        rd_val = if rs1 < imm { 1 } else { 0 };
                    }
        
                    RV32IAL::Xori => {
                        rd_val = rs1 ^ imm;
                    }
        
                    RV32IAL::Ori => {
                        rd_val = rs1 | imm;
                    }
        
                    RV32IAL::Andi => {
                        rd_val = rs1 & imm;
                    }
        
                    RV32IAL::Slli => {
                        rd_val = rs1.wrapping_shl(imm);
                    }
        
                    RV32IAL::Srli => {
                        rd_val = rs1.wrapping_shr(imm);
                    }
        
                    RV32IAL::Srai => {
                        rd_val = (rs1 as i32).wrapping_shr(imm) as u32;
                    }
        
                    RV32IAL::Add => {
                        rd_val = rs1.wrapping_add(rs2);
                    }
        
                    RV32IAL::Sub => {
                        rd_val = rs1.wrapping_sub(rs2);
                    }
        
                    RV32IAL::Xor => {
                        rd_val = rs1 ^ rs2;
                    }
        
                    RV32IAL::Or => {
                        rd_val = rs1 | rs2;
                    }
        
                    RV32IAL::And => {
                        rd_val = rs1 & rs2;
                    }
        
                    RV32IAL::Slt => {
                        rd_val = if (rs1 as i32) < (rs2 as i32) { 1 } else { 0 };
                    }
        
                    RV32IAL::Sltu => {
                        rd_val = if rs1 < rs2 { 1 } else { 0 };
                    }
        
                    RV32IAL::Sll => {
                        rd_val = rs1.wrapping_shl(rs2 & 0x1F);
                    }
        
                    RV32IAL::Srl => {
                        rd_val = rs1.wrapping_shr(rs2 & 0x1F);
                    }
        
                    RV32IAL::Sra => {
                        rd_val = (rs1 as i32).wrapping_shr(rs2 & 0x1F) as u32;
                    }
        
                    RV32IAL::Ecall => {
                        msg.rd_addr = 0;
                        log_err!(regfile.write_csr(RvCsrEnum::MEPC.into(), pc), ProcessError::Recoverable)?;
                        log_err!(regfile.write_csr(RvCsrEnum::MCAUSE.into(), 0x0000000b), ProcessError::Recoverable)?;
                        next_pc = log_err!(regfile.read_csr(RvCsrEnum::MTVEC.into()), ProcessError::Recoverable)?;
                    }
        
                    RV32IAL::Ebreak => unreachable!("WTF"),
        
                    RV32IAL::Fence => {
                        msg.rd_addr = 0;
                        // Do nothing
                    }
                }
            },

            RV32I::LS(inst) => {
                match inst {
                    RV32ILS::Lb => {
                        let addr = rs1.wrapping_add(imm);
                        let data = log_err!(mmu.read(addr, Mask::Byte), ProcessError::Recoverable)?;
                        rd_val = data.1 as i8 as u32;
                        if data.0 == true {
                            (self.callback.difftest_skip)(0);
                        }
                    }
        
                    RV32ILS::Lh => {
                        let addr = rs1.wrapping_add(imm);
                        let data = log_err!(mmu.read(addr, Mask::Half), ProcessError::Recoverable)?;
                        rd_val = data.1 as i16 as u32;
                        if data.0 == true {
                            (self.callback.difftest_skip)(0);
                        }
                    }
        
                    RV32ILS::Lw => {
                        let addr = rs1.wrapping_add(imm);
                        let data = log_err!(mmu.read(addr, Mask::Word), ProcessError::Recoverable)?;
                        rd_val = data.1;
                        if data.0 == true {
                            (self.callback.difftest_skip)(0);
                        }
                    }
        
                    RV32ILS::Lbu => {
                        let addr = rs1.wrapping_add(imm);
                        let data = log_err!(mmu.read(addr, Mask::Byte), ProcessError::Recoverable)?;
                        rd_val = data.1;
                        if data.0 == true {
                            (self.callback.difftest_skip)(0);
                        }
                    }
        
                    RV32ILS::Lhu => {
                        let addr = rs1.wrapping_add(imm);
                        let data = log_err!(mmu.read(addr, Mask::Half), ProcessError::Recoverable)?;
                        rd_val = data.1;
                        if data.0 == true {
                            (self.callback.difftest_skip)(0);
                        }
                    }
        
                    RV32ILS::Sb => {
                        msg.rd_addr = 0;
                        let addr = rs1.wrapping_add(imm);
                        if log_err!(mmu.write(addr, rs2, Mask::Byte), ProcessError::Recoverable)? == true {
                            (self.callback.difftest_skip)(0);
                        }
                    }
        
                    RV32ILS::Sh => {
                        msg.rd_addr = 0;
                        let addr = rs1.wrapping_add(imm);
                        if log_err!(mmu.write(addr, rs2, Mask::Half), ProcessError::Recoverable)? == true {
                            (self.callback.difftest_skip)(0);
                        }
                    }
        
                    RV32ILS::Sw => {
                        msg.rd_addr = 0;
                        let addr = rs1.wrapping_add(imm);
                        if log_err!(mmu.write(addr, rs2, Mask::Word), ProcessError::Recoverable)? == true {
                            (self.callback.difftest_skip)(0);
                        }
                    }
                }
            }
        }
        
        regfile.write_gpr(msg.rd_addr.into(), rd_val)?;

        regfile.write_pc(next_pc);

        Ok(next_pc)
    }

    fn rv32_e_execute(&mut self, name: RV32I, mut msg: InstMsg) -> ProcessResult<u32> {
        msg.rs1 &= 0xF;
        msg.rs2 &= 0xF;
        msg.rd_addr &= 0xF;
        self.rv32_i_execute(name, msg)
    }

    fn rv32_m_execute(&mut self, _name: RV32M, msg: InstMsg) -> ProcessResult<u32> {
        let regfile = &mut self.states.regfile;
        let rs1: u32 = regfile.read_gpr(msg.rs1.into()).map_err(|_| ProcessError::Recoverable)?;
        let rs2: u32 = regfile.read_gpr(msg.rs2.into()).map_err(|_| ProcessError::Recoverable)?;
        
        let rd_val: u32;

        let pc: u32 = regfile.read_pc();
        let next_pc = pc.wrapping_add(4);

        match _name {
            RV32M::Mul => {
                rd_val = rs1.wrapping_mul(rs2);
            }

            RV32M::Mulh => {
                rd_val = (rs1 as i64).wrapping_mul(rs2 as i64).wrapping_shr(32) as u32;
            }

            RV32M::Mulhsu => {
                rd_val = (rs1 as i32 as i64).wrapping_mul(rs2 as u32 as i64).wrapping_shr(32) as u32;
            }

            RV32M::Mulhu => {
                rd_val = (rs1 as u64).wrapping_mul(rs2 as u64).wrapping_shr(32) as u32;
            }

            RV32M::Div => {
                if rs2 == 0 {
                    rd_val = 0xFFFFFFFF;
                } else {
                    rd_val = (rs1 as i32).wrapping_div(rs2 as i32) as u32;
                }
            }

            RV32M::Divu => {
                if rs2 == 0 {
                    rd_val = 0xFFFFFFFF;
                } else {
                    rd_val = rs1.wrapping_div(rs2);
                }
            }

            RV32M::Rem => {
                if rs2 == 0 {
                    rd_val = rs1;
                } else {
                    rd_val = (rs1 as i32).wrapping_rem(rs2 as i32) as u32;
                }
            }

            RV32M::Remu => {
                if rs2 == 0 {
                    rd_val = rs1;
                } else {
                    rd_val = rs1.wrapping_rem(rs2);
                }
            }
        }
        
        regfile.write_gpr(msg.rd_addr.into(), rd_val).map_err(|_| ProcessError::Recoverable)?;

        regfile.write_pc(next_pc);

        Ok(next_pc)
    }

    fn rv32_priv_execute(&mut self, _name: Priv, _msg: InstMsg) -> ProcessResult<u32> {
        let regfile = &mut self.states.regfile;

        let next_pc;
        
        match _name {
            Priv::Mret => {
                next_pc = log_err!(regfile.read_csr(RvCsrEnum::MEPC.into()), ProcessError::Recoverable)?;
            }
        }

        regfile.write_pc(next_pc);

        Ok(next_pc)
    }

    fn rv32_zicsr_execute(&mut self, _name: Zicsr, msg: InstMsg) -> ProcessResult<u32> {
        let regfile = &mut self.states.regfile;
        let rs1: u32 = regfile.read_gpr(msg.rs1.into()).map_err(|_| ProcessError::Recoverable)?;
        
        let mut rd_val: u32 = 0;

        let pc: u32 = regfile.read_pc();
        let next_pc = pc.wrapping_add(4);

        let imm: u32 = msg.imm;
        let csr_addr = imm & 0xFFF;
        let csr_val = log_err!(regfile.read_csr(csr_addr), ProcessError::Recoverable)?;
        let mut csr_wdata = rs1;

        match _name {
            Zicsr::Csrrw => {
                rd_val = csr_val;
            }

            Zicsr::Csrrs => {
                rd_val = csr_val;
                csr_wdata |= csr_val;
            }

            _ => log_todo!(), 
        }
        
        regfile.write_gpr(msg.rd_addr.into(), rd_val)?;
        log_err!(regfile.write_csr(csr_addr, csr_wdata), ProcessError::Recoverable)?;

        regfile.write_pc(next_pc);

        Ok(next_pc)
    }
    
    fn rv32_decode(&mut self, inst: u32) -> ProcessResult<InstPattern> {
        let decode = self.instruction_set.instruction_parse(inst);

        Ok(match decode {
            DecodeResult::Result((name, msg)) => {
                // Extract register fields
                let rs1_addr = extract_bits(inst, 15..19);
                let rs2_addr = extract_bits(inst, 20..24);
                let rd_addr  = extract_bits(inst, 7..11) as u8;

                // Extract immediate value
                let imm = Self::get_imm(inst, msg );
        
                InstPattern::Normal { 
                    name, 
                    msg: InstMsg {
                        rs1: rs1_addr,
                        rs2: rs2_addr,
                        rd_addr,
                        imm,
                    } }
            }

            DecodeResult::Trap(trap) => {
                InstPattern::Trap(trap)
            }
        })
    }

    pub fn rv32_execute(&mut self, inst: InstPattern) -> ProcessResult<u32> {
        match inst {
            InstPattern::Normal { name, msg } => {
                let belongs_to = name;
                if !self.instruction_set.enable(belongs_to) {
                    return Err(ProcessError::Recoverable)
                }

                match belongs_to {
                    RISCV::RV32I(name) => {
                        return self.rv32_i_execute(name, msg);
                    }

                    RISCV::RV32E(name) => {
                        return self.rv32_e_execute(name, msg);
                    }

                    RISCV::RV32M(name) => {
                        return self.rv32_m_execute(name, msg);
                    }

                    RISCV::Priv(name) => {
                        return self.rv32_priv_execute(name, msg);
                    }

                    RISCV::Zicsr(name) => {
                        return self.rv32_zicsr_execute(name, msg);
                    }
                }
            }

            InstPattern::Trap(trap) => {
                if trap == Trap::Ebreak {
                    (self.callback.yield_)(); // just for now
                    return Err(ProcessError::Recoverable);
                }
                
                let regfile = &mut self.states.regfile;
                
                let pc = regfile.read_pc();
            
                let next_pc = regfile.trap(pc, trap as u32)?;

                regfile.write_pc(next_pc);

                Ok(next_pc)
            }
        }
    }

    /// Execute a single cycle in the emulator
    pub fn step_instruction(&mut self) -> ProcessResult<()> {
        // 1. Fetch: Read the PC and fetch the instruction
        let pc = self.states.regfile.read_pc();
        let inst = log_err!(
            self.states.mmu.read(pc, state::mmu::Mask::Word), 
            ProcessError::Recoverable
        )?;

        // 2. Decode: Decode the instruction
        let decode = self.rv32_decode(inst.1)?;
        
        // 3. Execute: Execute the instruction
        let next_pc = self.rv32_execute(decode)?;

        // 4. Notify completion and return
        (self.callback.instruction_complete)(pc, next_pc, inst.1)?;

        self.times.instructions += 1;

        Ok(())
    }

    pub fn times(&self) {
        self.times.print();
    }
}
