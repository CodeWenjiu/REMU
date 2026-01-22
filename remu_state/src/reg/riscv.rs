use crate::reg::{RegCmd, RegOption};

pub struct RiscvReg {
    pub pc: u32,
    gpr: [u32; 32],
    tracer: remu_types::TracerDyn,
}

impl RiscvReg {
    pub fn new(opt: RegOption, tracer: remu_types::TracerDyn) -> Self {
        Self {
            pc: opt.init_pc,
            gpr: [0; 32],
            tracer,
        }
    }

    #[inline(always)]
    pub fn read_gpr(&self, index: usize) -> u32 {
        unsafe { *self.gpr.get_unchecked(index) }
    }

    #[inline(always)]
    pub fn write_gpr(&mut self, index: usize, value: u32) {
        unsafe { *self.gpr.get_unchecked_mut(index) = value }
    }

    pub(crate) fn execute(&mut self, cmd: &RegCmd) {
        match cmd {
            RegCmd::Read { index } => {
                self.tracer
                    .borrow()
                    .reg_show(*index, self.read_gpr(index.idx()));
            }
            RegCmd::Print { range } => {
                // Build a stable (Gpr, u32) view for printing.
                // Avoid transmute; rely on known RISC-V naming ("x0".."x31") and the
                // `EnumString` implementation in `remu_types::Gpr`.
                let regs: Vec<(remu_types::Gpr, u32)> = self
                    .gpr
                    .iter()
                    .copied()
                    .enumerate()
                    .map(|(i, v)| {
                        let reg = format!("x{i}").parse::<remu_types::Gpr>().unwrap();
                        (reg, v)
                    })
                    .collect();

                self.tracer.borrow().reg_print(&regs, range.clone());
            }
            RegCmd::Write { index, value } => {
                self.write_gpr(index.idx(), *value);
            }
        }
    }
}
