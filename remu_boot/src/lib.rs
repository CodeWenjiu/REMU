//! Debugger entry: wires [`DebuggerOption`] to the correct simulator + ISA generic.

remu_macro::mod_prv!(config);

use std::sync::Arc;

use remu_debugger::{DebuggerOption, DebuggerRunner};
use remu_harness::RemuIsaKind;
use remu_isa::isa::IsaKind;
use remu_isa::isa::extension_enum::{
    RV32I, RV32I_zve32x_zvl128b, RV32IM, RV32IM_zve32x_zvl128b, RV64I, RV64IM,
};
use remu_simulator_nzea::{NzeaAction, NzeaIsaKind};
use remu_types::{DifftestRef, Platform};

use crate::config::{NzeaFast, NzeaMmioRemu, RemuFast, RemuMmioRemu, RemuMmioSpike};

macro_rules! dispatch_remu {
    ($kind:expr, $Config:ident, $runner:expr, $opt:expr, $irq:expr) => {
        match $kind {
            RemuIsaKind::Rv32I => $runner.run_with_config::<$Config<RV32I>>($opt, $irq),
            RemuIsaKind::Rv32Im => $runner.run_with_config::<$Config<RV32IM>>($opt, $irq),
            RemuIsaKind::Rv32IZve32xZvl128b => {
                $runner.run_with_config::<$Config<RV32I_zve32x_zvl128b>>($opt, $irq)
            }
            RemuIsaKind::Rv32ImZve32xZvl128b => {
                $runner.run_with_config::<$Config<RV32IM_zve32x_zvl128b>>($opt, $irq)
            }
            RemuIsaKind::Rv64I => $runner.run_with_config::<$Config<RV64I>>($opt, $irq),
            RemuIsaKind::Rv64Im => $runner.run_with_config::<$Config<RV64IM>>($opt, $irq),
        }
    };
}

macro_rules! dispatch_nzea {
    ($kind:expr, $Config:ident, $runner:expr, $opt:expr, $irq:expr) => {
        match $kind {
            NzeaIsaKind::Rv32I => $runner.run_with_config::<$Config<RV32I>>($opt, $irq),
            NzeaIsaKind::Rv32Im => $runner.run_with_config::<$Config<RV32IM>>($opt, $irq),
            NzeaIsaKind::Rv64I => $runner.run_with_config::<$Config<RV64I>>($opt, $irq),
            NzeaIsaKind::Rv64Im => $runner.run_with_config::<$Config<RV64IM>>($opt, $irq),
        }
    };
}

pub fn boot<R: DebuggerRunner>(
    option: DebuggerOption,
    runner: R,
    interrupt: Arc<std::sync::atomic::AtomicBool>,
) -> anyhow::Result<()> {
    if option.platform == Platform::Nzea {
        // Deploy to the board before anything of the run harness exists: hand
        // the program to `probe` and stop. No simulator is built (nor an ISA
        // resolved), so `action=flash` never touches the run path.
        let backend = option
            .sim
            .sim
            .backend_args()
            .map_err(|e| anyhow::anyhow!(e))?;
        let action = match backend.get("action") {
            None => NzeaAction::Run,
            Some(value) => value
                .parse::<NzeaAction>()
                .map_err(|e| anyhow::anyhow!(e))?,
        };
        if action == NzeaAction::Flash {
            let elf = option
                .sim
                .sim
                .state
                .bus
                .elf
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("--sim-opt action=flash needs --elf"))?;
            remu_simulator_nzea::flash(elf).map_err(|e| anyhow::anyhow!(e))?;
            return Ok(());
        }
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
