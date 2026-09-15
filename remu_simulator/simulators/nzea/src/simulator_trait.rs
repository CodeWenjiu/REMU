//! Nzea simulator: DPI bus_read/bus_write dispatch via global pointer; lifecycle only at init/drop.
//! Supports multiple ISAs (riscv32i, riscv32im); the model is selected by the Policy's ISA.

use std::ffi::{CString, c_void};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use remu_state::{State, StateCmd};
use remu_types::{ExitCode, TraceFlags, TraceKind, TracerDyn};

use remu_simulator::{
    BreakpointErrorKind, SimulatorCore, SimulatorDut, SimulatorInnerError, SimulatorOption,
    SimulatorPolicy, StatEntry, StatFilter, from_state_error,
};

use remu_state::bus::ObserverEvent;

use crate::NzeaIsa;
use crate::Watchdog;
use crate::{CommitMsg, NzeaDpi, clear_nzea, set_nzea};
use crate::{ensure_nzea_loaded, get_nzea_fns};
use remu_isa::isa::reg::{Csr as CsrKind, RegAccess};
use remu_isa::WordOps;
use remu_isa::Xlen;

/// True after the first time wavetrace is enabled in this process; then we do not open trace.fst again,
/// so a later run with wavetrace off does not overwrite the file.
static WAVETRACE_FILE_OPENED: AtomicBool = AtomicBool::new(false);

/// Collects nzea RTL stat_* counters streamed from `nzea_iter_stats` via callback.
#[derive(Default)]
pub(crate) struct StatCollector {
    /// Raw signal name → value (for derived-rule lookups and display).
    pub raw: std::collections::HashMap<String, u64>,
}

/// C callback: record one stat_* counter into the [`StatCollector`] behind `userdata`.
/// # Safety
/// `userdata` must point to a live `StatCollector` for the duration of the enumeration.
pub(crate) unsafe extern "C" fn collect_stat_cb(
    name: *const std::ffi::c_char,
    value: u32,
    userdata: *mut std::ffi::c_void,
) {
    let collector = unsafe { &mut *(userdata as *mut StatCollector) };
    let name = unsafe { std::ffi::CStr::from_ptr(name) }
        .to_string_lossy()
        .into_owned();
    collector.raw.insert(name, value as u64);
}

pub struct SimulatorNzea<P, const IS_DUT: bool>
where
    P: SimulatorPolicy + 'static,
    P::ISA: NzeaIsa,
{
    state: State<P>,
    sim_ptr: *mut c_void,
    /// C string for model key (`core|tile` + ISA); kept alive for FFI calls.
    model_c: CString,
    /// Function table loaded from libnzea.so
    fns: &'static crate::NzeaFns,
    tracer: TracerDyn,
    commit_buffer: Vec<CommitMsg>,
    interrupt: Arc<std::sync::atomic::AtomicBool>,
    /// Whether the last committed instruction accessed an MMIO device.
    last_commit_is_mmio: bool,
    /// Optional deadlock watchdog; fed every 1024 cycles with the current cycle count.
    watchdog: Option<Watchdog>,
    /// Breakpoint PCs; no duplicates.
    breakpoints: Vec<u32>,
    /// When true: on breakpoint hit, apply normally. When false: return BreakpointHit. Toggles on each hit.
    breakpoint_apply_next: bool,
    /// Set when DPI bus_write hits sifive_test_finisher; consumed by step_once.
    pending_exit_code: Option<ExitCode>,
    /// Total clock cycles executed (each cycle() = one clock).
    cycle_count: u64,
}

impl<P, const IS_DUT: bool> SimulatorCore<P> for SimulatorNzea<P, IS_DUT>
where
    P: SimulatorPolicy + 'static,
    P::ISA: NzeaIsa,
{
    fn new(
        opt: SimulatorOption,
        tracer: TracerDyn,
        interrupt: Arc<std::sync::atomic::AtomicBool>,
    ) -> Self {
        let backend_args = opt
            .backend_args()
            .unwrap_or_else(|e| panic!("invalid --sim-opt: {e}"));
        backend_args
            .assert_known_keys(&["target", "watchdog"])
            .unwrap_or_else(|e| panic!("invalid --sim-opt for nzea: {e}"));
        let target = backend_args
            .get("target")
            .map(|s| {
                s.parse::<crate::NzeaTarget>()
                    .unwrap_or_else(|e| panic!("invalid --sim-opt nzea.target: {e}"))
            })
            .unwrap_or_default();
        let watchdog_spec = backend_args.get("watchdog");

        let model_key = format!("{}:{}", target.as_str(), <P::ISA as NzeaIsa>::NZEA_ISA_STR);
        let model_c = CString::new(model_key.as_str()).expect("nzea model key contains null");

        let isa_str = <P::ISA as NzeaIsa>::NZEA_ISA_STR;
        if let Err(e) = ensure_nzea_loaded(&target, isa_str) {
            panic!(
                "nzea .so load failed for {}:{}: {e}",
                target.as_str(),
                isa_str
            );
        }
        let fns = get_nzea_fns(target.as_str(), isa_str);
        let sim_ptr = unsafe { (fns.create)(model_c.as_ptr() as *const i8) };
        assert!(
            !sim_ptr.is_null(),
            "nzea_create failed for model {}",
            model_key
        );

        let state = State::new(opt.state.clone(), tracer.clone(), IS_DUT);
        let watchdog = Watchdog::from_spec(watchdog_spec, Arc::clone(&interrupt))
            .unwrap_or_else(|e| panic!("invalid --sim-opt for nzea: {e}"));
        Self {
            state,
            sim_ptr,
            model_c,
            fns,
            tracer,
            commit_buffer: Vec::new(),
            interrupt,
            last_commit_is_mmio: false,
            watchdog,
            breakpoints: Vec::new(),
            breakpoint_apply_next: false,
            pending_exit_code: None,
            cycle_count: 0,
        }
    }

    fn init(&mut self) {
        unsafe {
            set_nzea(self as *mut Self as *mut dyn NzeaDpi);
        }
        let model_ptr = self.model_c.as_ptr();
        unsafe {
            (self.fns.set_reset)(self.sim_ptr, model_ptr, 1);
            for _ in 0..100 {
                (self.fns.set_clock)(self.sim_ptr, model_ptr, 0);
                (self.fns.eval)(self.sim_ptr, model_ptr);
                (self.fns.set_clock)(self.sim_ptr, model_ptr, 1);
                (self.fns.eval)(self.sim_ptr, model_ptr);
            }
            (self.fns.set_reset)(self.sim_ptr, model_ptr, 0);
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
        if self.last_commit_is_mmio {
            vec![ObserverEvent::MmioAccess]
        } else {
            vec![]
        }
    }

    fn step_once<const TRACE: u64>(&mut self) -> Result<(), remu_simulator::SimulatorInnerError> {
        // NZEA must be updated before each step: when set_nzea runs in init(), dut_model is still
        // a local in Harness::new; it is then moved into the Harness struct and the old address
        // becomes invalid. In step_once, self is the final location, so we must set it again.
        unsafe {
            set_nzea(self as *mut Self as *mut dyn NzeaDpi);
        }
        let mut cycle_count: u64 = 0;
        while self.commit_buffer.is_empty() {
            if let Some(ec) = self.pending_exit_code.take() {
                return Err(SimulatorInnerError::ProgramExit(ec));
            }
            self.cycle::<TRACE>();
            cycle_count += 1;
            if cycle_count % 1024 == 0 {
                if let Some(ref wd) = self.watchdog {
                    wd.feed(self.cycle_count);
                }
                if self.interrupt.load(Ordering::Relaxed) {
                    self.interrupt.store(false, Ordering::Relaxed);
                    return Err(remu_simulator::SimulatorInnerError::Interrupted);
                }
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
            let inst = self.state.bus.read_32(pc.to_usize()).unwrap_or(0);
            self.tracer.borrow().disasm(pc.to_u64(), inst);
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
                (self.fns.trace_open)(self.sim_ptr, self.model_c.as_ptr(), path_c.as_ptr());
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
        let model_ptr = self.model_c.as_ptr();
        unsafe {
            (self.fns.set_clock)(self.sim_ptr, model_ptr, 0);
            (self.fns.eval)(self.sim_ptr, model_ptr);
            if TraceFlags::waveform(TRACE_CYCLE) && IS_DUT {
                (self.fns.trace_dump)(self.sim_ptr);
            }
            (self.fns.set_clock)(self.sim_ptr, model_ptr, 1);
            (self.fns.eval)(self.sim_ptr, model_ptr);
            if TraceFlags::waveform(TRACE_CYCLE) && IS_DUT {
                (self.fns.trace_dump)(self.sim_ptr);
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
        self.last_commit_is_mmio = msg.is_mmio;
        *self.state.reg.pc = <<<P as remu_state::StatePolicy>::ISA as remu_isa::isa::RvIsa>::XLEN as Xlen>::from_u64(msg.next_pc as u64).into();
        if msg.csr_valid {
            if let Some(csr) = CsrKind::from_repr(msg.csr_addr as u16) {
                self.state.reg.csr.write(csr, msg.csr_data);
            }
        }
        if msg.gpr_addr < 32 && msg.gpr_addr != 0 {
            self.state.reg.gpr.raw_write(
                msg.gpr_addr as usize,
                Xlen::from_u64(msg.gpr_data as u64),
            );
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
            (self.fns.destroy)(self.sim_ptr, self.model_c.as_ptr());
            clear_nzea();
        }
    }
}

impl<P> SimulatorDut for SimulatorNzea<P, true>
where
    P: SimulatorPolicy + 'static,
    P::ISA: NzeaIsa,
{
    type Policy = P;

    fn set_breakpoint(&mut self, addr: u32) -> Result<(), SimulatorInnerError> {
        if addr % 4 != 0 {
            return Err(SimulatorInnerError::BreakpointError(
                BreakpointErrorKind::NotAligned,
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
            Err(SimulatorInnerError::BreakpointError(
                BreakpointErrorKind::NotFound(addr),
            ))
        }
    }

    fn print_breakpoints(&self) {
        self.tracer.borrow().breakpoint_print(&self.breakpoints);
    }

    fn platform_stats(&self, filter: &StatFilter) -> Vec<StatEntry> {
        // nzea RTL stat_* counters via VPI callback enumeration (dynamic — nzea
        // adds new counters without remu changes). Signals exist only in
        // sim=true RTL; on FPGA builds nzea_iter_stats returns -1 and no
        // entries are added.
        let mut collector = crate::StatCollector::default();
        let _n = unsafe {
            (self.fns.iter_stats)(
                self.sim_ptr,
                Some(crate::collect_stat_cb),
                &mut collector as *mut _ as *mut std::ffi::c_void,
            )
        };
        // Which raw signals to show: all of them (All/Raw), or only those a
        // group's derive rules depend on (Group).
        let group_rules: Vec<&crate::stat_derive::StatDeriveRule> = match filter {
            StatFilter::Group(name) => crate::stat_derive::NZEA_DERIVE_RULES
                .iter()
                .filter(|r| r.group == name)
                .collect(),
            _ => vec![],
        };
        let show_all_raw = !matches!(filter, StatFilter::Group(_));
        let group_deps: std::collections::HashSet<&str> = group_rules
            .iter()
            .flat_map(|r| r.deps.iter().copied())
            .collect();
        // Raw counters: one Named entry per selected signal, keeping the
        // platform-given signal name.
        let mut v = Vec::new();
        for (name, value) in &collector.raw {
            if show_all_raw || group_deps.contains(name.as_str()) {
                v.push(StatEntry::Named {
                    name: name.clone(),
                    value: value.to_string(),
                });
            }
        }
        // Derived entries: table-driven semantics (see stat_derive.rs).
        for rule in crate::stat_derive::NZEA_DERIVE_RULES {
            let in_group = match filter {
                StatFilter::All => true,
                StatFilter::Raw => false,
                StatFilter::Group(name) => rule.group == name,
            };
            if !in_group {
                continue;
            }
            let deps: Option<Vec<u64>> = rule
                .deps
                .iter()
                .map(|d| collector.raw.get(*d).copied())
                .collect();
            if let Some(deps) = deps {
                v.push(StatEntry::Derived {
                    name: rule.name.to_string(),
                    value: (rule.derive)(&deps),
                });
            }
        }
        v
    }
}
