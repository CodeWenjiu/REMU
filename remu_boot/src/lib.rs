//! Debugger entry: wires [`DebuggerOption`] to the correct simulator + ISA generic.
//!
//! ISA classification uses [`IsaKind`](remu_isa::isa::IsaKind): [`RemuIsaKind`](remu_harness::RemuIsaKind)
//! and [`NzeaIsaKind`](remu_simulator_nzea::NzeaIsaKind). This crate only dispatches on [`Platform`].

remu_macro::mod_flat!(config);

use std::sync::Arc;

use remu_debugger::{DebuggerOption, DebuggerRunner};
use remu_harness::RemuIsaKind;
use remu_isa::isa::{
    IsaKind,
    extension_enum::{
        RV32I, RV32I_wjCus0, RV32I_zve32x_zvl128b, RV32IM, RV32IM_wjCus0, RV32IM_zve32x_zvl128b,
    },
};
use remu_simulator_nzea::NzeaIsaKind;
use remu_types::{DifftestRef, Platform};

use crate::config::{NzeaFast, NzeaMmioRemu, RemuFast, RemuMmioRemu, RemuMmioSpike};

macro_rules! with_config {
    ($runner:expr, $opt:expr, $irq:expr, $Config:ty $(,)?) => {
        $runner.run_with_config::<$Config>($opt, $irq)
    };
}

macro_rules! dispatch_remu {
    ($kind:expr, $Config:ident, $runner:expr, $opt:expr, $irq:expr) => {
        match $kind {
            RemuIsaKind::Rv32I => with_config!($runner, $opt, $irq, $Config<RV32I>),
            RemuIsaKind::Rv32Im => with_config!($runner, $opt, $irq, $Config<RV32IM>),
            RemuIsaKind::Rv32IWjCus0 => with_config!($runner, $opt, $irq, $Config<RV32I_wjCus0>),
            RemuIsaKind::Rv32ImWjCus0 => with_config!($runner, $opt, $irq, $Config<RV32IM_wjCus0>),
            RemuIsaKind::Rv32IZve32xZvl128b => {
                with_config!($runner, $opt, $irq, $Config<RV32I_zve32x_zvl128b>)
            }
            RemuIsaKind::Rv32ImZve32xZvl128b => {
                with_config!($runner, $opt, $irq, $Config<RV32IM_zve32x_zvl128b>)
            }
        }
    };
}

macro_rules! dispatch_nzea {
    ($kind:expr, $Config:ident, $runner:expr, $opt:expr, $irq:expr) => {
        match $kind {
            NzeaIsaKind::Rv32I => with_config!($runner, $opt, $irq, $Config<RV32I>),
            NzeaIsaKind::Rv32Im => with_config!($runner, $opt, $irq, $Config<RV32IM>),
            NzeaIsaKind::Rv32IWjCus0 => with_config!($runner, $opt, $irq, $Config<RV32I_wjCus0>),
            NzeaIsaKind::Rv32ImWjCus0 => with_config!($runner, $opt, $irq, $Config<RV32IM_wjCus0>),
        }
    };
}

pub fn boot<R: DebuggerRunner>(
    option: DebuggerOption,
    runner: R,
    interrupt: Arc<std::sync::atomic::AtomicBool>,
) {
    let platform = option.platform;

    if platform == Platform::Nzea {
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
