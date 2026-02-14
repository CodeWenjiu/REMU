use crate::isa::extension_v::{NoV, Zve32xZvl128b};
use crate::isa::reg::csr::DIFFTEST_SLICES_BASE_AND_V;
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
    type VConfig = NoV;

    const ISA_STR: &'static str = "rv32i";
    const MISA: u32 = 0x4000_0100; // RV32, I
}

#[derive(Clone, Copy)]
pub struct ConfigRV32IM;
impl ArchConfig for ConfigRV32IM {
    type M = Enabled<()>;
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
    type VConfig = NoV;

    const ISA_STR: &'static str = "rv32im";
    const MISA: u32 = 0x4000_1100; // RV32, I, M
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
    type VConfig = NoV;

    const ISA_STR: &'static str = "rv32if";
    const MISA: u32 = 0x4000_0120; // RV32, I, F
}

/// RV32I + Zve32x + Zvl128b: embedded vector subset (VConfig = Zve32xZvl128b).
/// MISA.V is not set: Zve* is a subset profile; Spike (and spec) reserve MISA bit V for the
/// full V extension (single letter 'v' in base ISA). So MISA = RV32, I only.
#[allow(non_camel_case_types)]
#[derive(Clone, Copy)]
pub struct RV32I_zve32x_zvl128b;
impl RvIsa for RV32I_zve32x_zvl128b {
    type XLEN = u32;
    type Conf = ConfigRV32I;
    type PcState = PcState;
    type GprState = GprState;
    type FprState = ();
    type VConfig = Zve32xZvl128b;

    const ISA_STR: &'static str = "rv32i_zve32x_zvl128b";
    const MISA: u32 = 0x4000_0100; // RV32, I (Zve subset does not set MISA.V)

    fn csrs_for_difftest() -> &'static [&'static [crate::isa::reg::Csr]] {
        DIFFTEST_SLICES_BASE_AND_V
    }
}
