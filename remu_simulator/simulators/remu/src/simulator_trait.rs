use remu_state::{State, StateCmd, StateError};
use remu_types::{DifftestMismatchItem, RegGroup, TracerDyn};

use remu_simulator::{
    SimulatorInnerError, SimulatorOption, SimulatorPolicy, SimulatorPolicyOf, SimulatorTrait,
    from_state_error,
};

use crate::icache::Icache;
use crate::riscv::inst::{decode, execute};
use crate::{Func, FuncCmd};

const ICACHE_SIZE: usize = 1 << 16;

pub struct SimulatorRemu<P: SimulatorPolicy, const IS_DUT: bool> {
    state: State<P>,
    func: Func,
    tracer: TracerDyn,
    icache: Icache<ICACHE_SIZE>,
}

impl<P: SimulatorPolicy, const IS_DUT: bool> SimulatorPolicyOf for SimulatorRemu<P, IS_DUT> {
    type Policy = P;
}

impl<P: SimulatorPolicy, const IS_DUT: bool> SimulatorTrait<P, IS_DUT>
    for SimulatorRemu<P, IS_DUT>
{
    const ENABLE: bool = true;

    fn new(opt: SimulatorOption, tracer: TracerDyn) -> Self {
        Self {
            state: State::new(opt.state.clone(), tracer.clone(), IS_DUT),
            func: Func::new(),
            tracer,
            icache: Icache::new(),
        }
    }

    fn state(&self) -> &State<P> {
        &self.state
    }

    fn state_mut(&mut self) -> &mut State<P> {
        &mut self.state
    }

    #[inline(always)]
    fn step_once(&mut self) -> Result<(), SimulatorInnerError> {
        let pc = *self.state.reg.pc;
        let entry = self.icache.get_entry_mut(pc);
        if entry.addr == pc {
            execute(&mut self.state, &entry.decoded).map_err(from_state_error)?;
            return Ok(());
        }
        let inst = self
            .state
            .bus
            .read_32(pc as usize)
            .map_err(|e| from_state_error(StateError::from(e)))?;
        if self.func.trace.instruction && IS_DUT {
            self.tracer.borrow().disasm(pc as u64, inst);
        }
        let d = decode::<P>(inst);
        entry.addr = pc;
        entry.decoded = d;
        execute(&mut self.state, &d).map_err(from_state_error)?;
        Ok(())
    }

    #[inline(always)]
    fn sync_from(&mut self, dut: &State<P>) {
        self.state.reg.pc = dut.reg.pc;
        self.state.reg.gpr = dut.reg.gpr;
        self.state.reg.fpr = dut.reg.fpr;
        self.state.reg.csr = dut.reg.csr.clone();
    }

    #[inline(always)]
    fn regs_match(&self, dut: &State<P>) -> bool {
        self.regs_diff(dut).is_empty()
    }

    fn regs_diff(&self, dut: &State<P>) -> Vec<DifftestMismatchItem> {
        use remu_types::isa::reg::RegDiff;
        let mut out = Vec::new();
        let (r, d) = (&self.state.reg, &dut.reg);
        for (name, ref_val, dut_val) in
            <P::ISA as remu_types::isa::RvIsa>::PcState::diff(&r.pc, &d.pc)
        {
            out.push(DifftestMismatchItem {
                group: RegGroup::Pc,
                name,
                ref_val,
                dut_val,
            });
        }
        for (name, ref_val, dut_val) in
            <P::ISA as remu_types::isa::RvIsa>::GprState::diff(&r.gpr, &d.gpr)
        {
            out.push(DifftestMismatchItem {
                group: RegGroup::Gpr,
                name,
                ref_val,
                dut_val,
            });
        }
        for (name, ref_val, dut_val) in
            <P::ISA as remu_types::isa::RvIsa>::FprState::diff(&r.fpr, &d.fpr)
        {
            out.push(DifftestMismatchItem {
                group: RegGroup::Fpr,
                name,
                ref_val,
                dut_val,
            });
        }
        out
    }

    fn func_exec(&mut self, subcmd: &FuncCmd) {
        self.func.execute(subcmd);
    }

    fn state_exec(&mut self, subcmd: &StateCmd) -> Result<(), SimulatorInnerError> {
        self.state.execute(subcmd).map_err(from_state_error)?;
        Ok(())
    }
}
