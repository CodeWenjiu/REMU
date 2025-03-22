use crate::Simulator;

pub struct Emu {
    rv32i_ena: bool,
    rv32m_ena: bool,
    rv32e_ena: bool,
    zicsr_ena: bool,
    priv_ena: bool,
}

impl Simulator for Emu {
}

impl Emu {
    pub fn new() -> Self {
        Self {
            rv32i_ena: true,
            rv32m_ena: true,
            rv32e_ena: true,
            zicsr_ena: true,
            priv_ena: true,
        }
    }
}
