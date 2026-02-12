remu_macro::mod_pub!(riscv);
remu_macro::mod_flat!(icache, simulator_trait, func);

pub use remu_simulator::FuncCmd;
pub use simulator_trait::SimulatorRemu;
