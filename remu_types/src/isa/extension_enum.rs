use crate::isa::reg::{FprRegs, GprState, PcState};
use crate::isa::{ArchConfig, RvIsa, extension::{Disabled, Enabled}};

#[derive(Clone, Copy)]
pub struct ConfigRV32I;
impl ArchConfig for ConfigRV32I {
    type M = Disabled;
    type F = Disabled;
}
#[derive(Clone, Copy)]
pub struct RV32I;
impl RvIsa for RV32I {
    type XLEN = u32;
    type Conf = ConfigRV32I;
    type PcState = PcState;
    type GprState = GprState;
    type FprState = ();
}

#[derive(Clone, Copy)]
pub struct ConfigRV32IM;
impl ArchConfig for ConfigRV32IM {
    type M = Disabled;
    type F = Disabled;
}
#[derive(Clone, Copy)]
pub struct RV32IM;
impl RvIsa for RV32IM {
    type XLEN = u32;
    type Conf = ConfigRV32IM;
    type PcState = PcState;
    type GprState = GprState;
    type FprState = ();
}

#[derive(Clone, Copy)]
pub struct ConfigRV32IF;
impl ArchConfig for ConfigRV32IF {
    type M = Disabled;
    type F = Enabled<FprRegs>;
}
#[derive(Clone, Copy)]
pub struct RV32IF;
impl RvIsa for RV32IF {
    type XLEN = u32;
    type Conf = ConfigRV32IF;
    type PcState = PcState;
    type GprState = GprState;
    type FprState = FprRegs;
}
