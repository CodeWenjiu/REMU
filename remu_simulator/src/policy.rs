use std::marker::PhantomData;

use remu_state::bus::{BusObserver, FastObserver, MmioObserver};
use remu_types::isa::RvIsa;

pub trait SimulatorPolicy {
    type ISA: RvIsa;
    type Observer: BusObserver;
}

pub struct SimulatorFastProfile<ISA>
where
    ISA: RvIsa,
{
    _marker: PhantomData<ISA>,
}

impl<ISA> SimulatorPolicy for SimulatorFastProfile<ISA>
where
    ISA: RvIsa,
{
    type ISA = ISA;
    type Observer = FastObserver;
}

pub struct SimulatorMmioProfile<ISA>
where
    ISA: RvIsa,
{
    _marker: PhantomData<ISA>,
}

impl<ISA> SimulatorPolicy for SimulatorMmioProfile<ISA>
where
    ISA: RvIsa,
{
    type ISA = ISA;
    type Observer = MmioObserver;
}
