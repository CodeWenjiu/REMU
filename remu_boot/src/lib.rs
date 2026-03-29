//! Debugger entry: wires [`DebuggerOption`] to the correct simulator + ISA generic.
//!
//! ISA classification uses [`IsaKind`](remu_types::isa::IsaKind): [`RemuIsaKind`](remu_harness::RemuIsaKind)
//! and [`NzeaIsaKind`](remu_simulator_nzea::NzeaIsaKind). This crate only dispatches on [`Platform`].

use std::sync::Arc;

use remu_debugger::{DebuggerOption, DebuggerRunner};
use remu_harness::{RemuIsaKind, SimulatorNzea, SimulatorRemu};
use remu_simulator_nzea::{NzeaIsa, NzeaIsaKind};
use remu_state::{StateFastProfile, StateMmioProfile};
use remu_types::{
    DifftestRef, Platform,
    isa::{
        IsaKind,
        RvIsa,
        extension_enum::{
            RV32I, RV32I_wjCus0, RV32I_zve32x_zvl128b, RV32IM, RV32IM_wjCus0, RV32IM_zve32x_zvl128b,
        },
    },
};

fn boot_with_isa_remu<ISA, Run>(
    option: DebuggerOption,
    runner: Run,
    interrupt: Arc<std::sync::atomic::AtomicBool>,
) where
    ISA: RvIsa,
    Run: DebuggerRunner,
{
    match option.difftest {
        None => runner.run::<SimulatorRemu<StateFastProfile<ISA>, true>, ()>(option, Arc::clone(&interrupt)),
        Some(DifftestRef::Remu) => runner.run::<SimulatorRemu<StateMmioProfile<ISA>, true>, SimulatorRemu<StateMmioProfile<ISA>, false>>(
            option,
            Arc::clone(&interrupt),
        ),
        Some(DifftestRef::Spike) => runner.run::<
            SimulatorRemu<StateMmioProfile<ISA>, true>,
            remu_harness::SimulatorSpike<StateMmioProfile<ISA>>,
        >(option, interrupt),
    }
}

fn boot_with_isa_nzea<ISA, Run>(
    option: DebuggerOption,
    runner: Run,
    interrupt: Arc<std::sync::atomic::AtomicBool>,
) where
    ISA: RvIsa + NzeaIsa,
    Run: DebuggerRunner,
{
    match option.difftest {
        None => runner.run::<SimulatorNzea<StateFastProfile<ISA>, true>, ()>(option, interrupt),
        Some(DifftestRef::Remu) => runner.run::<
            SimulatorNzea<StateMmioProfile<ISA>, true>,
            SimulatorRemu<StateMmioProfile<ISA>, false>,
        >(option, interrupt),
        Some(DifftestRef::Spike) => {
            panic!("nzea + difftest spike not supported yet")
        }
    }
}

pub fn boot<R: DebuggerRunner>(
    option: DebuggerOption,
    runner: R,
    interrupt: Arc<std::sync::atomic::AtomicBool>,
) {
    let platform = option.platform;

    if platform == Platform::Nzea {
        match NzeaIsaKind::from_isa_spec_or_panic(&option.isa) {
            NzeaIsaKind::Rv32I => boot_with_isa_nzea::<RV32I, R>(option, runner, interrupt),
            NzeaIsaKind::Rv32Im => boot_with_isa_nzea::<RV32IM, R>(option, runner, interrupt),
            NzeaIsaKind::Rv32IWjCus0 => boot_with_isa_nzea::<RV32I_wjCus0, R>(option, runner, interrupt),
            NzeaIsaKind::Rv32ImWjCus0 => boot_with_isa_nzea::<RV32IM_wjCus0, R>(option, runner, interrupt),
        }
    } else {
        match RemuIsaKind::from_isa_spec_or_panic(&option.isa) {
            RemuIsaKind::Rv32I => boot_with_isa_remu::<RV32I, R>(option, runner, interrupt),
            RemuIsaKind::Rv32Im => boot_with_isa_remu::<RV32IM, R>(option, runner, interrupt),
            RemuIsaKind::Rv32IWjCus0 => boot_with_isa_remu::<RV32I_wjCus0, R>(option, runner, interrupt),
            RemuIsaKind::Rv32ImWjCus0 => boot_with_isa_remu::<RV32IM_wjCus0, R>(option, runner, interrupt),
            RemuIsaKind::Rv32IZve32xZvl128b => {
                boot_with_isa_remu::<RV32I_zve32x_zvl128b, R>(option, runner, interrupt)
            }
            RemuIsaKind::Rv32ImZve32xZvl128b => {
                boot_with_isa_remu::<RV32IM_zve32x_zvl128b, R>(option, runner, interrupt)
            }
        }
    }
}
