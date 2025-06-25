use state::reg::riscv::Trap;

remu_macro::mod_pub!(hardware, direct_map, instruction);

#[derive(Default, Clone, Copy, Debug)]
pub struct BasicStageMsg {
    pc: u32,
    npc: u32, // for branch prediction

    trap: Option<Trap>,
}
