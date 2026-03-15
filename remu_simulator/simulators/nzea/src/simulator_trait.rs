//! Nzea simulator: DPI bus_read/bus_write dispatch via global pointer; lifecycle only at init/drop.
//! Supports multiple ISAs (riscv32i, riscv32im); the model is selected by the Policy's ISA.

use std::ffi::{CString, c_void};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use remu_state::{State, StateCmd};
use remu_types::{ExitCode, TraceFlags, TraceKind, TracerDyn};

use remu_simulator::{
    SimulatorCore, SimulatorDut, SimulatorInnerError, SimulatorOption, SimulatorPolicy,
    SimulatorPolicyOf, StatContext, StatEntry, from_state_error,
};

use remu_state::bus::ObserverEvent;

use crate::dpi::{self, CommitMsg, NzeaDpi};
use crate::nzea_ffi::{self, NzeaIsa};
use remu_types::isa::reg::{Csr as CsrKind, RegAccess};

/// True after the first time wavetrace is enabled in this process; then we do not open trace.fst again,
/// so a later run with wavetrace off does not overwrite the file.
static WAVETRACE_FILE_OPENED: AtomicBool = AtomicBool::new(false);

pub struct SimulatorNzea<P, const IS_DUT: bool>
where
    P: SimulatorPolicy + 'static,
    P::ISA: NzeaIsa,
{
    state: State<P>,
    sim_ptr: *mut c_void,
    /// C string for ISA; kept alive for nzea_* FFI calls.
    isa_c: CString,
    tracer: TracerDyn,
    commit_buffer: Vec<CommitMsg>,
    interrupt: Arc<std::sync::atomic::AtomicBool>,
    /// Pending memory write events; instruction commit and mem access may be out of sync.
    event_buffer: Vec<ObserverEvent>,
    /// mem_count of the last applied commit; used by take_observer_events to pop the right number of ops.
    last_commit_mem_count: u32,
    /// is_load of the last applied commit; when true, take_observer_events pops 0 (load needs no diff).
    last_commit_is_load: bool,
    /// Breakpoint PCs; no duplicates.
    breakpoints: Vec<u32>,
    /// When true: on breakpoint hit, apply normally. When false: return BreakpointHit. Toggles on each hit.
    breakpoint_apply_next: bool,
    /// Set when DPI bus_write hits sifive_test_finisher; consumed by step_once.
    pending_exit_code: Option<ExitCode>,
    /// Total clock cycles executed (each cycle() = one clock).
    cycle_count: u64,
}

impl<P, const IS_DUT: bool> SimulatorPolicyOf for SimulatorNzea<P, IS_DUT>
where
    P: SimulatorPolicy,
    P::ISA: NzeaIsa,
{
    type Policy = P;
}

impl<P, const IS_DUT: bool> SimulatorCore<P> for SimulatorNzea<P, IS_DUT>
where
    P: SimulatorPolicy + 'static,
    P::ISA: NzeaIsa,
{
    fn new(opt: SimulatorOption, tracer: TracerDyn, interrupt: Arc<std::sync::atomic::AtomicBool>) -> Self {
        let isa_c = CString::new(<P::ISA as NzeaIsa>::NZEA_ISA_STR).expect("nzea ISA str contains null");
        let sim_ptr = unsafe { nzea_ffi::nzea_create(isa_c.as_ptr()) };
        assert!(!sim_ptr.is_null(), "nzea_create failed for ISA {}", <P::ISA as NzeaIsa>::NZEA_ISA_STR);

        let state = State::new(opt.state.clone(), tracer.clone(), IS_DUT);
        Self {
            state,
            sim_ptr,
            isa_c,
            tracer,
            commit_buffer: Vec::new(),
            interrupt,
            event_buffer: Vec::new(),
            last_commit_mem_count: 0,
            last_commit_is_load: false,
            breakpoints: Vec::new(),
            breakpoint_apply_next: false,
            pending_exit_code: None,
            cycle_count: 0,
        }
    }

    fn init(&mut self) {
        unsafe {
            dpi::set_nzea(self as *mut Self as *mut dyn NzeaDpi);
        }
        let isa_ptr = self.isa_c.as_ptr();
        unsafe {
            nzea_ffi::nzea_set_reset(self.sim_ptr, isa_ptr, 1);
            for _ in 0..100 {
                nzea_ffi::nzea_set_clock(self.sim_ptr, isa_ptr, 0);
                nzea_ffi::nzea_eval(self.sim_ptr, isa_ptr);
                nzea_ffi::nzea_set_clock(self.sim_ptr, isa_ptr, 1);
                nzea_ffi::nzea_eval(self.sim_ptr, isa_ptr);
            }
            nzea_ffi::nzea_set_reset(self.sim_ptr, isa_ptr, 0);
        }
        // Waveform file is opened in on_trace_change() when Wavetrace is first enabled,
        // so a run with wavetrace disabled does not overwrite an existing trace.fst.
    }

    fn state(&self) -> &State<P> {
        &self.state
    }

    fn state_mut(&mut self) -> &mut State<P> {
        &mut self.state
    }

    fn take_observer_events(&mut self) -> Vec<ObserverEvent> {
        let state_events = self.state_mut().bus.take_observer_events();
        self.event_buffer.extend(state_events);
        let n = if self.last_commit_is_load {
            let first_is_mmio = self
                .event_buffer
                .first()
                .map_or(false, |e| matches!(e, ObserverEvent::MmioAccess));
            if first_is_mmio {
                self.last_commit_mem_count as usize
            } else {
                0
            }
        } else {
            self.last_commit_mem_count as usize
        };
        self.event_buffer
            .drain(..n.min(self.event_buffer.len()))
            .collect()
    }

    fn step_once<const TRACE: u64>(&mut self) -> Result<(), remu_simulator::SimulatorInnerError> {
        // NZEA must be updated before each step: when set_nzea runs in init(), dut_model is still
        // a local in Harness::new; it is then moved into the Harness struct and the old address
        // becomes invalid. In step_once, self is the final location, so we must set it again.
        unsafe {
            dpi::set_nzea(self as *mut Self as *mut dyn NzeaDpi);
        }
        let mut cycle_count: u64 = 0;
        while self.commit_buffer.is_empty() {
            if let Some(ec) = self.pending_exit_code.take() {
                return Err(SimulatorInnerError::ProgramExit(ec));
            }
            self.cycle::<TRACE>();
            cycle_count += 1;
            if cycle_count % 1024 == 0 && self.interrupt.load(Ordering::Relaxed) {
                self.interrupt.store(false, Ordering::Relaxed);
                return Err(remu_simulator::SimulatorInnerError::Interrupted);
            }
        }
        if let Some(ec) = self.pending_exit_code.take() {
            return Err(SimulatorInnerError::ProgramExit(ec));
        }
        let msg = self.commit_buffer.remove(0);
        if IS_DUT && self.breakpoints.contains(&msg.next_pc) {
            if !self.breakpoint_apply_next {
                self.breakpoint_apply_next = true;
                self.commit_buffer.insert(0, msg);
                return Err(SimulatorInnerError::BreakpointHit(msg.next_pc));
            }
            self.breakpoint_apply_next = false;
        }
        if TraceFlags::instruction(TRACE) && IS_DUT {
            let pc = *self.state.reg.pc;
            let inst = self.state.bus.read_32(pc as usize).unwrap_or(0);
            self.tracer.borrow().disasm(pc as u64, inst);
        }
        self.apply_commit(msg);
        Ok(())
    }

    fn state_exec(&mut self, subcmd: &StateCmd) -> Result<(), SimulatorInnerError> {
        self.state.execute(subcmd).map_err(from_state_error)?;
        Ok(())
    }

    fn on_trace_change(&mut self, kind: TraceKind, enabled: bool) {
        if IS_DUT
            && kind == TraceKind::Wavetrace
            && enabled
            && WAVETRACE_FILE_OPENED
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
        {
            let trace_path = Self::trace_path();
            let path_c = CString::new(trace_path.to_string_lossy().as_ref()).unwrap();
            unsafe {
                nzea_ffi::nzea_trace_open(self.sim_ptr, self.isa_c.as_ptr(), path_c.as_ptr());
            }
        }
    }
}

impl<P, const IS_DUT: bool> SimulatorNzea<P, IS_DUT>
where
    P: SimulatorPolicy + 'static,
    P::ISA: NzeaIsa,
{
    /// Push a commit from DPI; used by dpi_commit_trace.
    pub(crate) fn push_commit_impl(&mut self, msg: CommitMsg) {
        self.commit_buffer.push(msg);
    }

    /// Set by dpi_write_32 when sifive_test_finisher is written; consumed by step_once.
    pub(crate) fn set_pending_exit_code(&mut self, ec: ExitCode) {
        self.pending_exit_code = Some(ec);
    }

    /// Run one clock cycle (low + high phase). TRACE_CYCLE const selects whether to dump; trace_dump is DCE'd when false.
    fn cycle<const TRACE_CYCLE: u64>(&mut self) {
        self.cycle_count += 1;
        let isa_ptr = self.isa_c.as_ptr();
        unsafe {
            nzea_ffi::nzea_set_clock(self.sim_ptr, isa_ptr, 0);
            nzea_ffi::nzea_eval(self.sim_ptr, isa_ptr);
            if TraceFlags::waveform(TRACE_CYCLE) && IS_DUT {
                nzea_ffi::nzea_trace_dump(self.sim_ptr);
            }
            nzea_ffi::nzea_set_clock(self.sim_ptr, isa_ptr, 1);
            nzea_ffi::nzea_eval(self.sim_ptr, isa_ptr);
            if TraceFlags::waveform(TRACE_CYCLE) && IS_DUT {
                nzea_ffi::nzea_trace_dump(self.sim_ptr);
            }
        }
    }

    /// Path for waveform file (target/trace.fst when under cargo, else trace.fst in cwd or exe dir).
    fn trace_path() -> std::path::PathBuf {
        std::env::var_os("CARGO_TARGET_DIR")
            .map(std::path::PathBuf::from)
            .or_else(|| {
                std::env::current_exe().ok().and_then(|p| {
                    let exe_dir = p.parent()?;
                    let target = exe_dir.parent()?;
                    if exe_dir
                        .file_name()
                        .map(|n| n == "debug" || n == "release")
                        .unwrap_or(false)
                    {
                        Some(target.to_path_buf())
                    } else {
                        Some(exe_dir.to_path_buf())
                    }
                })
            })
            .map(|d| d.join("trace.fst"))
            .unwrap_or_else(|| std::path::PathBuf::from("trace.fst"))
    }

    /// Apply a commit to state (for difftest).
    fn apply_commit(&mut self, msg: CommitMsg) {
        self.last_commit_mem_count = msg.mem_count;
        self.last_commit_is_load = msg.is_load;
        *self.state.reg.pc = msg.next_pc;
        if msg.csr_valid {
            if let Some(csr) = CsrKind::from_repr(msg.csr_addr as u16) {
                self.state.reg.csr.write(csr, msg.csr_data);
            }
        }
        if msg.gpr_addr < 32 && msg.gpr_addr != 0 {
            self.state
                .reg
                .gpr
                .raw_write(msg.gpr_addr as usize, msg.gpr_data);
        }
    }
}

impl<P, const IS_DUT: bool> Drop for SimulatorNzea<P, IS_DUT>
where
    P: SimulatorPolicy + 'static,
    P::ISA: NzeaIsa,
{
    fn drop(&mut self) {
        unsafe {
            nzea_ffi::nzea_destroy(self.sim_ptr, self.isa_c.as_ptr());
            dpi::clear_nzea();
        }
    }
}

impl<P> SimulatorDut for SimulatorNzea<P, true>
where
    P: SimulatorPolicy + 'static,
    P::ISA: NzeaIsa,
{
    fn set_breakpoint(&mut self, addr: u32) -> Result<(), SimulatorInnerError> {
        if addr % 4 != 0 {
            return Err(SimulatorInnerError::BreakpointError(
                "breakpoint address must be 4-byte aligned".into(),
            ));
        }
        if !self.breakpoints.contains(&addr) {
            self.breakpoints.push(addr);
        }
        Ok(())
    }

    fn del_breakpoint(&mut self, addr: u32) -> Result<(), SimulatorInnerError> {
        if let Some(pos) = self.breakpoints.iter().position(|&x| x == addr) {
            self.breakpoints.remove(pos);
            Ok(())
        } else {
            Err(SimulatorInnerError::BreakpointError(format!(
                "breakpoint at 0x{addr:08x} not found"
            )))
        }
    }

    fn print_breakpoints(&self) {
        self.tracer.borrow().breakpoint_print(&self.breakpoints);
    }

    fn platform_stats(&self, ctx: &StatContext) -> Vec<StatEntry> {
        let mut v = vec![StatEntry::CycleCount(self.cycle_count)];
        if self.cycle_count > 0 {
            v.push(StatEntry::Ipc(ctx.inst_count as f64 / self.cycle_count as f64));
        }
        v
    }
}
