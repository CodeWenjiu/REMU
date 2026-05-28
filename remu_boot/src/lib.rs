//! Debugger entry: wires [`DebuggerOption`] to the correct simulator + ISA generic.

remu_macro::mod_flat!(config);

use std::sync::Arc;

use remu_debugger::{DebuggerOption, DebuggerRunner};
use remu_harness::RemuIsaKind;
use remu_isa::isa::IsaKind;
use remu_isa::isa::extension_enum::{
    RV32I, RV32I_wjCus0, RV32I_zve32x_zvl128b, RV32IM, RV32IM_wjCus0, RV32IM_zve32x_zvl128b,
};
use remu_simulator_nzea::NzeaIsaKind;
use remu_types::{DifftestRef, Platform};

use crate::config::{NzeaFast, NzeaMmioRemu, RemuFast, RemuMmioRemu, RemuMmioSpike};

macro_rules! dispatch_remu {
    ($kind:expr, $Config:ident, $runner:expr, $opt:expr, $irq:expr) => {
        match $kind {
            RemuIsaKind::Rv32I => $runner.run_with_config::<$Config<RV32I>>($opt, $irq),
            RemuIsaKind::Rv32Im => $runner.run_with_config::<$Config<RV32IM>>($opt, $irq),
            RemuIsaKind::Rv32IWjCus0 => {
                $runner.run_with_config::<$Config<RV32I_wjCus0>>($opt, $irq)
            }
            RemuIsaKind::Rv32ImWjCus0 => {
                $runner.run_with_config::<$Config<RV32IM_wjCus0>>($opt, $irq)
            }
            RemuIsaKind::Rv32IZve32xZvl128b => {
                $runner.run_with_config::<$Config<RV32I_zve32x_zvl128b>>($opt, $irq)
            }
            RemuIsaKind::Rv32ImZve32xZvl128b => {
                $runner.run_with_config::<$Config<RV32IM_zve32x_zvl128b>>($opt, $irq)
            }
        }
    };
}

macro_rules! dispatch_nzea {
    ($kind:expr, $Config:ident, $runner:expr, $opt:expr, $irq:expr) => {
        match $kind {
            NzeaIsaKind::Rv32I => $runner.run_with_config::<$Config<RV32I>>($opt, $irq),
            NzeaIsaKind::Rv32Im => $runner.run_with_config::<$Config<RV32IM>>($opt, $irq),
            NzeaIsaKind::Rv32IWjCus0 => {
                $runner.run_with_config::<$Config<RV32I_wjCus0>>($opt, $irq)
            }
            NzeaIsaKind::Rv32ImWjCus0 => {
                $runner.run_with_config::<$Config<RV32IM_wjCus0>>($opt, $irq)
            }
        }
    };
}

pub fn boot<R: DebuggerRunner>(
    option: DebuggerOption,
    runner: R,
    interrupt: Arc<std::sync::atomic::AtomicBool>,
) {
    if option.platform == Platform::Nzea {
        let kind = NzeaIsaKind::from_isa_spec_or_panic(&option.isa);
        match option.difftest {
            None => dispatch_nzea!(kind, NzeaFast, runner, option, interrupt),
            Some(DifftestRef::Remu) => {
                dispatch_nzea!(kind, NzeaMmioRemu, runner, option, interrupt)
            }
            Some(DifftestRef::Spike) => panic!("nzea + difftest spike not supported yet"),
        }
    } else {
        let kind = RemuIsaKind::from_isa_spec_or_panic(&option.isa);
        match option.difftest {
            None => dispatch_remu!(kind, RemuFast, runner, option, interrupt),
            Some(DifftestRef::Remu) => {
                dispatch_remu!(kind, RemuMmioRemu, runner, option, interrupt)
            }
            Some(DifftestRef::Spike) => {
                dispatch_remu!(kind, RemuMmioSpike, runner, option, interrupt)
            }
        }
    }
}
