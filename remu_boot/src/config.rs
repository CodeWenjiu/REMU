//! 单一泛型 `Config<Dut, Ref>` — 所有平台的配置自动从 Dut 推导 Policy。
//!
//! 具体组合只是一行 type alias，新增模拟器/Difftest 模式只需加 alias，无需写工厂代码。

use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use remu_harness::{
    PlatformConfig, SimulatorCore, SimulatorNzea, SimulatorOption, SimulatorRemu, SimulatorSpike,
};
use remu_state::{StateFastProfile, StateMmioProfile};
use remu_types::TracerDyn;

// ── 通用 Config ──

pub(crate) struct Config<Dut, Ref>
where
    Dut: remu_harness::SimulatorDut,
    Ref: remu_harness::SimulatorRef<Dut::Policy>,
{
    _marker: PhantomData<(Dut, Ref)>,
}

impl<Dut, Ref> PlatformConfig for Config<Dut, Ref>
where
    Dut: remu_harness::SimulatorDut,
    Ref: remu_harness::SimulatorRef<Dut::Policy>,
{
    type Policy = Dut::Policy;
    type Dut = Dut;
    type Ref = Ref;

    fn create_dut(opt: &SimulatorOption, tracer: TracerDyn, irq: Arc<AtomicBool>) -> Dut {
        let mut dut = Dut::new(opt.clone(), tracer, irq);
        <Dut as SimulatorCore<Dut::Policy>>::init(&mut dut);
        dut
    }

    fn create_ref(opt: &SimulatorOption, tracer: TracerDyn, irq: Arc<AtomicBool>) -> Ref {
        let mut r = Ref::new(opt.clone(), tracer, irq);
        <Ref as SimulatorCore<Dut::Policy>>::init(&mut r);
        r
    }
}

// ── 具体组合：一行 type alias ──

/// Remu DUT, fast observer, no difftest ref.
pub(crate) type RemuFast<ISA> = Config<SimulatorRemu<StateFastProfile<ISA>, true>, ()>;

/// Remu DUT, MMIO observer, difftest via another remu instance.
pub(crate) type RemuMmioRemu<ISA> =
    Config<SimulatorRemu<StateMmioProfile<ISA>, true>, SimulatorRemu<StateMmioProfile<ISA>, false>>;

/// Remu DUT, MMIO observer, difftest via Spike.
pub(crate) type RemuMmioSpike<ISA> =
    Config<SimulatorRemu<StateMmioProfile<ISA>, true>, SimulatorSpike<StateMmioProfile<ISA>>>;

/// Nzea DUT, fast observer, no difftest ref.
pub(crate) type NzeaFast<ISA> = Config<SimulatorNzea<StateFastProfile<ISA>, true>, ()>;

/// Nzea DUT, MMIO observer, difftest via remu.
pub(crate) type NzeaMmioRemu<ISA> =
    Config<SimulatorNzea<StateMmioProfile<ISA>, true>, SimulatorRemu<StateMmioProfile<ISA>, false>>;
