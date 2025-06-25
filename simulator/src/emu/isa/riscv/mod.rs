use state::reg::riscv::Trap;

remu_macro::mod_flat!(direct_map, singlecycle, pipeline);

remu_macro::mod_pub!(frontend, backend, instruction);

#[derive(Default, Clone, Copy, Debug)]
pub struct BasicStageMsg {
    pc: u32,
    npc: u32, // for branch prediction

    trap: Option<Trap>,
}
