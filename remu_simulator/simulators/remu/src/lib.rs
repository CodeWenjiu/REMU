remu_macro::mod_pub!(crate, riscv);
remu_macro::mod_prv!(icache, simulator_trait);

pub(crate) use simulator_trait::ExecuteContext;
pub use simulator_trait::SimulatorRemu;
