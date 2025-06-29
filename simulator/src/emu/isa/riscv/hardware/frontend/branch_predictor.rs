use std::fmt::{self, Display};

use crate::emu::{extract_bits, isa::riscv::hardware::frontend::ToIfStage, EmuHardware};

// BTB条目
#[derive(Clone)]
pub struct BTBEntry {
    pub tag: u32,
    pub target: u32,
}

// BTB结构
pub struct BTB {
    entries: Vec<BTBEntry>,
    set_bits: u32,
}

impl BTB {
    pub fn new(size: usize) -> Self {
        assert!(size.is_power_of_two() && size > 0, "BTB size must be a power of two and > 0");
        let set_bits = size.trailing_zeros();
        Self {
            entries: vec![BTBEntry { tag: 0, target: 0 }; size],
            set_bits,
        }
    }
    fn set(&self, pc: u32) -> usize {
        // (pc & ((1 << self.set_bits) - 1)) as usize
        extract_bits(pc, 2..(self.set_bits + 1)) as usize
    }
    fn tag(&self, pc: u32) -> u32 {
        pc >> (self.set_bits + 2)
    }
    pub fn get(&self, pc: u32) -> Option<u32> {
        let set = self.set(pc);
        let tag = self.tag(pc);
        let entry = &self.entries[set];
        if entry.tag == tag {
            Some(entry.target)
        } else {
            None
        }
    }
    pub fn update(&mut self, pc: u32, target: u32) {
        let set = self.set(pc);
        let tag = self.tag(pc);
        self.entries[set] = BTBEntry { tag, target };
    }
}

impl Display for BTB {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "BTB ({} entries):", self.entries.len())?;
        for (i, entry) in self.entries.iter().enumerate() {
            writeln!(f, "  idx {:02x}: tag={:x}, target={:08x}", i, entry.tag, entry.target)?;
        }
        Ok(())
    }
}

impl Default for BTB {
    fn default() -> Self {
        Self::new(16) // 默认16项
    }
}

impl EmuHardware {
    fn get_btb_mut(&mut self) -> &mut BTB {
        &mut self.btb
    }
    fn get_btb(&self) -> &BTB {
        &self.btb
    }

    fn branch_predict(&self) -> u32 {
        let pc = self.pipeline.pipeline_pc;
        if let Some(target) = self.get_btb().get(pc) {
            target
        } else {
            pc.wrapping_add(4)
        }
    }

    pub fn self_pipeline_branch_predict(&self) -> ToIfStage {
        let pc = self.pipeline.pipeline_pc;
        let npc = self.branch_predict();
        ToIfStage::new(pc, npc)
    }

    pub fn self_pipeline_branch_predict_update(&mut self) {
        let npc = self.branch_predict();
        self.pipeline.pipeline_pc = npc;
    }

    pub fn self_pipeline_branch_predict_flush(&mut self, pc: u32, target: u32) {
        self.get_btb_mut().update(pc, target);
        self.pipeline.pipeline_pc = target;
    }
}