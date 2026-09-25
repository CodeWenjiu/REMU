//! Nzea simulator: DPI bus_read/bus_write dispatch via global pointer; lifecycle only at init/drop.
//! Supports multiple ISAs (riscv32i, riscv32im); the model is selected by the Policy's ISA.

use std::ffi::{CString, c_void};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use remu_state::{State, StateCmd};
use remu_types::{ExitCode, TraceFlags, TraceKind, TracerDyn};

use remu_simulator::{
    BreakpointErrorKind, SimulatorCore, SimulatorDut, SimulatorInnerError, SimulatorOption,
    SimulatorPolicy, StatCmd, StatEntry, from_state_error,
};

use remu_state::bus::ObserverEvent;

use crate::NzeaIsa;
use crate::Watchdog;
use crate::stat_schema::StatSchema;
use crate::{CommitMsg, NzeaDpi, clear_nzea, set_nzea};
use crate::{ensure_nzea_loaded, get_nzea_fns};
use remu_isa::WordOps;
use remu_isa::Xlen;
use remu_isa::isa::reg::{Csr as CsrKind, RegAccess};

/// True after the first time wavetrace is enabled in this process; then we do not open trace.fst again,
/// so a later run with wavetrace off does not overwrite the file.
static WAVETRACE_FILE_OPENED: AtomicBool = AtomicBool::new(false);

/// Collects nzea RTL stat_* counters streamed from `nzea_iter_stats` via callback.
#[derive(Default)]
struct StatCollector {
    /// Signal name → assembled value (for derived-rule lookups and display).
    raw: std::collections::HashMap<String, u64>,
    /// Signal name → actual RTL width, cross-checked against the schema.
    width: std::collections::HashMap<String, u32>,
}

/// C callback: record one stat_* counter into the [`StatCollector`] behind `userdata`.
/// # Safety
/// `userdata` must point to a live `StatCollector` for the duration of the enumeration.
unsafe extern "C" fn collect_stat_cb(
    name: *const std::ffi::c_char,
    lo: u32,
    hi: u32,
    width: u32,
    userdata: *mut std::ffi::c_void,
) {
    let collector = unsafe { &mut *(userdata as *mut StatCollector) };
    let name = unsafe { std::ffi::CStr::from_ptr(name) }
        .to_string_lossy()
        .into_owned();
    let value = crate::assemble_stat_value(lo, hi, width);
    collector.raw.insert(name.clone(), value);
    collector.width.insert(name, width);
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
    /// The RTL's stats declaration (`<Design>.stats.toml` next to `filelist.f`),
    /// loaded once per model build/load. `Ok(None)` = this build exposes no
    /// stats; `Err` = the file is present but unusable. Both are reported when
    /// statistics are requested, never at startup.
    schema: Result<Option<StatSchema>, String>,
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
        // The RTL's stats declaration is written by the same dump that produced
        // the model, next to `filelist.f`. Load it once here (handoff R1).
        let schema = StatSchema::load(
            &crate::verilog_dir(target.as_str(), isa_str),
            target.top_module(),
        );
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
            schema,
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
            self.state
                .reg
                .gpr
                .raw_write(msg.gpr_addr as usize, Xlen::from_u64(msg.gpr_data as u64));
        }
    }

    /// Read every `stat_*` signal the RTL exposes — assembled value plus actual
    /// width — and check it against the schema. A declared counter that the RTL
    /// does not expose, or exposes at a different width, is a build mismatch;
    /// an exposed signal the schema does not declare is ignored with a warning
    /// (the schema is the contract).
    fn sample_counters(&self, schema: &StatSchema) -> Result<StatCollector, String> {
        let mut collector = StatCollector::default();
        let n = unsafe {
            (self.fns.iter_stats)(
                self.sim_ptr,
                Some(collect_stat_cb),
                &mut collector as *mut _ as *mut std::ffi::c_void,
            )
        };
        if n < 0 {
            return Err("nzea_iter_stats failed: VPI unavailable (is --vpi enabled?)".to_string());
        }

        for c in &schema.counters {
            let Some(actual) = collector.width.get(&c.name) else {
                return Err(format!(
                    "schema declares counter `{}` but the RTL exposes no such signal (build mismatch)",
                    c.name
                ));
            };
            if *actual != c.width {
                return Err(format!(
                    "schema declares counter `{}` as {}-bit but the RTL exposes it as {}-bit (build mismatch)",
                    c.name, c.width, actual
                ));
            }
        }

        // The collector hands them out in hash order, so sort: warnings must not
        // reorder between runs.
        let mut undeclared: Vec<&str> = collector
            .raw
            .keys()
            .map(String::as_str)
            .filter(|n| schema.counter_index(n).is_none())
            .collect();
        undeclared.sort_unstable();
        for name in undeclared {
            eprintln!(
                "warning: nzea RTL exposes `{name}` but the schema does not declare it; ignoring"
            );
        }
        Ok(collector)
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

    /// Schema-backed statistics: the RTL declares counters, regions and derived
    /// expressions; remu reads them, evaluates them and renders them. Nothing
    /// here is hardcoded — the schema is the contract.
    fn platform_stats(&self, cmd: &StatCmd) -> Result<Vec<StatEntry>, String> {
        let schema = match &self.schema {
            Ok(Some(s)) => s,
            Ok(None) => {
                return Err(
                    "this build exposes no stats (no <Design>.stats.toml next to filelist.f)"
                        .to_string(),
                );
            }
            Err(e) => return Err(e.clone()),
        };
        let counters = self.sample_counters(schema)?;

        // Derived values: evaluated once, in file order (references only point
        // upwards, so a single pass resolves every entry).
        let values = schema.eval_derived(&counters.raw);
        let counter_entry = |i: usize| StatEntry::Named {
            name: schema.counters[i].name.clone(),
            value: counters.raw[&schema.counters[i].name].to_string(),
        };
        let derived_entry = |i: usize| StatEntry::Derived {
            name: schema.derived[i].name.clone(),
            value: schema.derived[i].render(&values[i]),
        };

        // Display order is the declaration order (R4); a query resolves an
        // exact entry name first, then a region name (R6/R7).
        let mut out = Vec::new();
        match cmd {
            StatCmd::Raw => {
                for i in 0..schema.counters.len() {
                    out.push(counter_entry(i));
                }
            }
            StatCmd::All => {
                for i in 0..schema.counters.len() {
                    out.push(counter_entry(i));
                }
                for i in 0..schema.derived.len() {
                    out.push(derived_entry(i));
                }
            }
            StatCmd::Query(q) => {
                if let Some(i) = schema.counter_index(q) {
                    out.push(counter_entry(i));
                } else if let Some(i) = schema.derived_index(q) {
                    // A single entry focuses its transitive dependencies too, so
                    // `stat ipc` shows the counters it is computed from.
                    let (counter_set, derived_set) = schema.dependency_closure(i);
                    for (j, keep) in counter_set.iter().enumerate() {
                        if *keep {
                            out.push(counter_entry(j));
                        }
                    }
                    for (j, keep) in derived_set.iter().enumerate() {
                        if *keep {
                            out.push(derived_entry(j));
                        }
                    }
                } else if schema.has_region(q) {
                    for (i, c) in schema.counters.iter().enumerate() {
                        if c.region == *q {
                            out.push(counter_entry(i));
                        }
                    }
                    for (i, d) in schema.derived.iter().enumerate() {
                        if d.region == *q {
                            out.push(derived_entry(i));
                        }
                    }
                } else {
                    return Err(format!(
                        "unknown statistic `{q}`; available — {}",
                        schema.available_names()
                    ));
                }
            }
        }
        Ok(out)
    }
}
