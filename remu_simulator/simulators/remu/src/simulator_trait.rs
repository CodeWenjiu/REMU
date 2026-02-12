use remu_state::{State, StateCmd, StateError};
use remu_types::{DifftestMismatchItem, RegGroup, TracerDyn};

use remu_simulator::{
    SimulatorInnerError, SimulatorOption, SimulatorPolicy, SimulatorPolicyOf, SimulatorTrait,
    from_state_error,
};

use std::cell::Cell;
use std::ptr;

use crate::icache::Icache;
use crate::riscv::inst::{decode, execute};

const ICACHE_SIZE: usize = 1 << 16;

thread_local! {
    /// Icache pointer for fence.i (cold path only). Set by SimulatorRemu, cleared on drop.
    static FENCE_I_ICACHE: Cell<*mut ()> = const { Cell::new(ptr::null_mut()) };
}

/// Called only from fence.i execution path. Flushes the thread-local Icache if set.
#[inline(never)]
pub(crate) fn fence_i_flush_icache() {
    FENCE_I_ICACHE.with(|c| {
        let p = c.get();
        if !p.is_null() {
            unsafe { (*p.cast::<Icache<ICACHE_SIZE>>()).flush() }
        }
    });
}

pub(crate) fn set_fence_i_icache(ptr: *mut ()) {
    FENCE_I_ICACHE.with(|c| c.set(ptr));
}

pub(crate) fn clear_fence_i_icache(ptr: *mut ()) {
    FENCE_I_ICACHE.with(|c| {
        if c.get() == ptr {
            c.set(ptr::null_mut());
        }
    });
}

pub struct SimulatorRemu<P: SimulatorPolicy, const IS_DUT: bool> {
    state: State<P>,
    tracer: TracerDyn,
    icache: Icache<ICACHE_SIZE>,
}

impl<P: SimulatorPolicy, const IS_DUT: bool> SimulatorPolicyOf for SimulatorRemu<P, IS_DUT> {
    type Policy = P;
}

impl<P: SimulatorPolicy, const IS_DUT: bool> Drop for SimulatorRemu<P, IS_DUT> {
    fn drop(&mut self) {
        clear_fence_i_icache((&mut self.icache) as *mut _ as *mut ());
    }
}

impl<P: SimulatorPolicy, const IS_DUT: bool> SimulatorTrait<P, IS_DUT>
    for SimulatorRemu<P, IS_DUT>
{
    const ENABLE: bool = true;

    fn new(opt: SimulatorOption, tracer: TracerDyn) -> Self {
        let mut icache = Icache::new();
        set_fence_i_icache(&mut icache as *mut _ as *mut ());
        Self {
            state: State::new(opt.state.clone(), tracer.clone(), IS_DUT),
            tracer,
            icache,
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
            execute(&mut self.state, &entry.decoded).map_err(from_state_error)?;
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
        execute(&mut self.state, &d).map_err(from_state_error)?;
        Ok(())
    }

    fn step_n<const ITRACE: bool>(&mut self, n: usize) -> Result<usize, SimulatorInnerError> {
        let mut executed = 0usize;
        while executed < n {
            let pc = *self.state.reg.pc;
            let entry = self.icache.get_entry_mut(pc);
            if entry.addr == pc {
                execute(&mut self.state, &entry.decoded).map_err(from_state_error)?;
                executed += 1;
                if ITRACE && IS_DUT {
                    let inst = self
                        .state
                        .bus
                        .read_32(pc as usize)
                        .map_err(|e| from_state_error(StateError::from(e)))
                        .unwrap();
                    self.tracer.borrow().disasm(pc as u64, inst);
                }
                continue;
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
            execute(&mut self.state, &d).map_err(from_state_error)?;
            executed += 1;
        }
        Ok(executed)
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
