use std::collections::HashMap;

use remu_state::{State, StateCmd, StateError};
use remu_types::{DifftestMismatchItem, RegGroup, TracerDyn};

use remu_simulator::{
    from_state_error, SimulatorCore, SimulatorDut, SimulatorInnerError, SimulatorOption,
    SimulatorPolicy, SimulatorPolicyOf, SimulatorRef,
};

use crate::icache::Icache;
use crate::riscv::inst::decode;
use remu_state::StatePolicy;

const ICACHE_SIZE: usize = 1 << 16;

/// RISC-V 32-bit ebreak encoding (imm[11]=1, opcode=system).
const EBREAK_INST: u32 = 0x0010_0073;

/// Breakpoint state machine: IDLE = stop on ebreak, Active = execute original instruction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BreakpointState {
    /// Default. When ebreak is hit, execution stops (breakpoint hit).
    #[default]
    Idle,
    /// Single-stepping over a breakpoint: when ebreak is hit, execute the original instruction.
    Active,
}

/// Execution context for decode+execute: provides state, icache flush, and ebreak handling.
pub(crate) trait ExecuteContext<P: StatePolicy> {
    fn state_mut(&mut self) -> &mut State<P>;
    #[inline]
    fn flush_icache(&mut self) {}

    /// Called when ebreak is executed. Default: stop (breakpoint hit).
    fn on_ebreak(&mut self, pc: u32) -> Result<(), StateError> {
        Err(StateError::BreakpointHit(pc))
    }
}

pub struct SimulatorRemu<P: SimulatorPolicy, const IS_DUT: bool> {
    state: State<P>,
    tracer: TracerDyn,
    icache: Icache<ICACHE_SIZE>,
    /// Breakpoint PC -> original instruction (only used when IS_DUT).
    breakpoints: HashMap<u32, u32>,
    /// When IDLE, ebreak stops; when Active, ebreak runs the original instruction (only used when IS_DUT).
    breakpoint_state: BreakpointState,
}

impl<P: SimulatorPolicy, const IS_DUT: bool> SimulatorPolicyOf for SimulatorRemu<P, IS_DUT> {
    type Policy = P;
}

impl<P: SimulatorPolicy, const IS_DUT: bool> ExecuteContext<P> for SimulatorRemu<P, IS_DUT> {
    fn state_mut(&mut self) -> &mut State<P> {
        SimulatorCore::state_mut(self)
    }
    fn flush_icache(&mut self) {
        self.icache.flush();
    }
    fn on_ebreak(&mut self, pc: u32) -> Result<(), StateError> {
        if !IS_DUT {
            return Err(StateError::BreakpointHit(pc));
        }
        match self.breakpoint_state {
            BreakpointState::Idle => {
                self.breakpoint_state = BreakpointState::Active;
                Err(StateError::BreakpointHit(pc))
            }
            BreakpointState::Active => {
                let orig = self.breakpoints.get(&pc).copied().unwrap();
                let decoded = decode::<P>(orig);
                self.execute_inst(&decoded)?;
                self.breakpoint_state = BreakpointState::Idle;
                Ok(())
            }
        }
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

impl<P: SimulatorPolicy, const IS_DUT: bool> SimulatorCore<P> for SimulatorRemu<P, IS_DUT> {
    fn new(opt: SimulatorOption, tracer: TracerDyn) -> Self {
        Self {
            state: State::new(opt.state.clone(), tracer.clone(), IS_DUT),
            tracer,
            icache: Icache::new(),
            breakpoints: HashMap::new(),
            breakpoint_state: BreakpointState::default(),
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
                let inst = if let Some(&orig) = self.breakpoints.get(&pc) {
                    orig
                } else {
                    self
                        .state
                        .bus
                        .read_32(pc as usize)
                        .map_err(|e| from_state_error(StateError::from(e)))
                        .unwrap()
                };
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
            let trace_inst = if let Some(&orig) = self.breakpoints.get(&pc) {
                orig
            } else {
                inst
            };
            self.tracer.borrow().disasm(pc as u64, trace_inst);
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
        self.state.reg.vr = dut.reg.vr.clone();
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
        for (name, ref_val, dut_val) in
            <<P::ISA as remu_types::isa::RvIsa>::VConfig as remu_types::isa::extension_v::VExtensionConfig>::VrState::diff(
                &r.vr, &d.vr,
            )
        {
            out.push(DifftestMismatchItem {
                group: RegGroup::Vr,
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

    fn mem_compare(&mut self, addr: usize, dut_data: &[u8]) -> Option<Box<[u8]>> {
        if IS_DUT {
            return None;
        }
        let mut buf = vec![0u8; dut_data.len()];
        SimulatorCore::state_mut(self).bus.read_bytes(addr, &mut buf).ok()?;
        if buf == dut_data {
            None
        } else {
            Some(buf.into_boxed_slice())
        }
    }
}

impl<P: SimulatorPolicy> SimulatorDut for SimulatorRemu<P, true> {
    fn set_breakpoint(&mut self, addr: u32) -> Result<(), SimulatorInnerError> {
        if addr % 4 != 0 {
            return Err(SimulatorInnerError::BreakpointError(
                "breakpoint address must be 4-byte aligned".into(),
            ));
        }
        if self.breakpoints.contains_key(&addr) {
            return Ok(());
        }
        let orig = self
            .state
            .bus
            .read_32_no_observer(addr as usize)
            .map_err(StateError::from)
            .map_err(SimulatorInnerError::from)?;
        self.state
            .bus
            .write_32_no_observer(addr as usize, EBREAK_INST)
            .map_err(StateError::from)
            .map_err(SimulatorInnerError::from)?;
        self.breakpoints.insert(addr, orig);
        self.icache.invalidate(addr);
        Ok(())
    }

    fn del_breakpoint(&mut self, addr: u32) -> Result<(), SimulatorInnerError> {
        if let Some(orig) = self.breakpoints.remove(&addr) {
            self.state
                .bus
                .write_32_no_observer(addr as usize, orig)
                .map_err(StateError::from)
                .map_err(SimulatorInnerError::from)?;
            self.icache.invalidate(addr);
            Ok(())
        } else {
            Err(SimulatorInnerError::BreakpointError(format!(
                "breakpoint at 0x{addr:x} not found"
            )))
        }
    }

    fn print_breakpoints(&self) {
        let mut addrs: Vec<u32> = self.breakpoints.keys().copied().collect();
        addrs.sort();
        self.tracer.borrow().breakpoint_print(&addrs);
    }
}

impl<P: SimulatorPolicy> SimulatorRef<P> for SimulatorRemu<P, false> {
    const ENABLE: bool = true;
}
