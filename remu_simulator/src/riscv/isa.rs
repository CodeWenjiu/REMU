// use std::intrinsics::unreachable;

// use target_lexicon::Riscv32Architecture;

pub trait Isa: 'static + Copy {
    const HAS_M: bool;
}

#[derive(Debug, Clone, Copy)]
pub struct Rv32<const M: bool>;

impl<const M: bool> Isa for Rv32<M> {
    const HAS_M: bool = M;
}
