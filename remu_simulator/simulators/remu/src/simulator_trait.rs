use remu_state::{State, StateCmd, StateError};
use remu_types::{DifftestMismatchItem, RegGroup, TracerDyn};

use remu_simulator::{
    from_state_error, SimulatorInnerError, SimulatorOption, SimulatorPolicy, SimulatorPolicyOf,
    SimulatorTrait,
};

use crate::icache::Icache;
use crate::riscv::inst::decode;
use remu_state::StatePolicy;

const ICACHE_SIZE: usize = 1 << 16;

/// Execution context for decode+execute: provides state and optional icache flush (fence.i).
pub(crate) trait ExecuteContext<P: StatePolicy> {
    fn state_mut(&mut self) -> &mut State<P>;
    #[inline]
    fn flush_icache(&mut self) {}
}

pub struct SimulatorRemu<P: SimulatorPolicy, const IS_DUT: bool> {
    state: State<P>,
    tracer: TracerDyn,
    icache: Icache<ICACHE_SIZE>,
}

impl<P: SimulatorPolicy, const IS_DUT: bool> SimulatorPolicyOf for SimulatorRemu<P, IS_DUT> {
    type Policy = P;
}

impl<P: SimulatorPolicy, const IS_DUT: bool> ExecuteContext<P> for SimulatorRemu<P, IS_DUT> {
    fn state_mut(&mut self) -> &mut State<P> {
        SimulatorTrait::state_mut(self)
    }
    fn flush_icache(&mut self) {
        self.icache.flush();
    }
}

impl<P: SimulatorPolicy, const IS_DUT: bool> SimulatorRemu<P, IS_DUT> {
    #[inline(always)]
    fn execute_inst(
        &mut self,
        decoded: &crate::riscv::inst::DecodedInst,
    ) -> Result<(), StateError> {
        crate::riscv::inst::execute(self, decoded)
    }
}

impl<P: SimulatorPolicy, const IS_DUT: bool> SimulatorTrait<P, IS_DUT>
    for SimulatorRemu<P, IS_DUT>
{
    const ENABLE: bool = true;

    fn new(opt: SimulatorOption, tracer: TracerDyn) -> Self {
        Self {
            state: State::new(opt.state.clone(), tracer.clone(), IS_DUT),
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
    fn step_once<const ITRACE: bool>(&mut self) -> Result<(), SimulatorInnerError> {
        let pc = *self.state.reg.pc;
        let entry = self.icache.get_entry_mut(pc);
        if entry.addr == pc {
            let decoded = entry.decoded;
            self.execute_inst(&decoded).map_err(from_state_error)?;
            if ITRACE && IS_DUT {
                let inst = self
                    .state
                    .bus
                    .read_32(pc as usize)
                    .map_err(|e| from_state_error(StateError::from(e)))
                    .unwrap();
                self.tracer.borrow().disasm(pc as u64, inst);
            }
            return Ok(());
        }
        let inst = self
            .state
            .bus
            .read_32(pc as usize)
            .map_err(|e| from_state_error(StateError::from(e)))?;
        if ITRACE && IS_DUT {
            self.tracer.borrow().disasm(pc as u64, inst);
        }
        let d = decode::<P>(inst);
        entry.addr = pc;
        entry.decoded = d;
        self.execute_inst(&d).map_err(from_state_error)?;
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

    fn state_exec(&mut self, subcmd: &StateCmd) -> Result<(), SimulatorInnerError> {
        self.state.execute(subcmd).map_err(from_state_error)?;
        Ok(())
    }
}
