use std::marker::PhantomData;

use remu_types::isa::RvIsa;

use crate::bus::{BusObserver, FastObserver, MmioObserver};

pub trait StatePolicy {
    type ISA: RvIsa;
    type Observer: BusObserver;
}

pub struct StateProfile<ISA, O>
where
    ISA: RvIsa,
    O: BusObserver,
{
    _marker: PhantomData<(ISA, O)>,
}

impl<ISA, O> StatePolicy for StateProfile<ISA, O>
where
    ISA: RvIsa,
    O: BusObserver,
{
    type ISA = ISA;
    type Observer = O;
}

pub struct StateFastProfile<ISA>
where
    ISA: RvIsa,
{
    _marker: PhantomData<ISA>,
}

impl<ISA> StatePolicy for StateFastProfile<ISA>
where
    ISA: RvIsa,
{
    type ISA = ISA;
    type Observer = FastObserver;
}

pub struct StateMmioProfile<ISA>
where
    ISA: RvIsa,
{
    _marker: PhantomData<ISA>,
}

impl<ISA> StatePolicy for StateMmioProfile<ISA>
where
    ISA: RvIsa,
{
    type ISA = ISA;
    type Observer = MmioObserver;
}
