use mmu::MMU;
use reg::RegfileIo;
use remu_utils::ISA;

remu_macro::mod_pub!(mmu, reg);

pub struct States {
    pub regfile: Box<dyn RegfileIo>,
    pub mmu: MMU,
}

impl States {
    pub fn new(isa: ISA, reset_vector: u32) -> Result<Self, ()> {
        let regfile = reg::regfile_io_factory(isa, reset_vector)?;

        let mmu = MMU::new();

        Ok(States { regfile, mmu })
    }
}
