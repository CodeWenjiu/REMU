use remu_state::{StateFastProfile, StateMmioProfile, StatePolicy};
use remu_types::isa::RvIsa;

pub trait SimulatorPolicy: StatePolicy {}

impl<ISA> SimulatorPolicy for StateFastProfile<ISA> where ISA: RvIsa {}

impl<ISA> SimulatorPolicy for StateMmioProfile<ISA> where ISA: RvIsa {}

/// 从具体模拟器类型中取出其使用的 `SimulatorPolicy`，供 Harness 等在不显式写泛型 P 时约束 D/R。
pub trait SimulatorPolicyOf {
    type Policy: SimulatorPolicy;
}
