//! Nzea simulator: DPI bus_read/bus_write dispatch via global pointer; lifecycle only at init/drop.

use std::ffi::{CString, c_void};

use remu_state::{State, StateCmd};
use remu_types::{TraceFlags, TracerDyn};

use remu_simulator::{
    from_state_error, SimulatorCore, SimulatorDut, SimulatorInnerError, SimulatorOption,
    SimulatorPolicy, SimulatorPolicyOf,
};

use crate::dpi::{self, NzeaDpi};
use crate::nzea_ffi;

pub struct SimulatorNzea<P: SimulatorPolicy + 'static, const IS_DUT: bool> {
    state: State<P>,
    sim_ptr: *mut c_void,
    _tracer: TracerDyn,
}

impl<P: SimulatorPolicy, const IS_DUT: bool> SimulatorPolicyOf for SimulatorNzea<P, IS_DUT> {
    type Policy = P;
}

impl<P: SimulatorPolicy + 'static, const IS_DUT: bool> SimulatorCore<P>
    for SimulatorNzea<P, IS_DUT>
{
    fn new(opt: SimulatorOption, tracer: TracerDyn) -> Self {
        let sim_ptr = unsafe { nzea_ffi::nzea_create() };
        assert!(!sim_ptr.is_null(), "nzea_create failed");

        let state = State::new(opt.state.clone(), tracer.clone(), IS_DUT);
        Self {
            state,
            sim_ptr,
            _tracer: tracer,
        }
    }

    fn init(&mut self) {
        unsafe {
            dpi::set_nzea(self as *mut Self as *mut dyn NzeaDpi);
        }
        unsafe {
            nzea_ffi::nzea_set_reset(self.sim_ptr, 1);
            for _ in 0..100 {
                nzea_ffi::nzea_set_clock(self.sim_ptr, 0);
                nzea_ffi::nzea_eval(self.sim_ptr);
                nzea_ffi::nzea_set_clock(self.sim_ptr, 1);
                nzea_ffi::nzea_eval(self.sim_ptr);
            }
            nzea_ffi::nzea_set_reset(self.sim_ptr, 0);
        }
        unsafe {
            // Write waveform to target/trace.fst, not next to the executable
            let trace_path = std::env::var_os("CARGO_TARGET_DIR")
                .map(std::path::PathBuf::from)
                .or_else(|| {
                    std::env::current_exe().ok().and_then(|p| {
                        let exe_dir = p.parent()?;
                        let target = exe_dir.parent()?;
                        // When under target/debug or target/release, use target/trace.fst
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
                .unwrap_or_else(|| std::path::PathBuf::from("trace.fst"));
            let path_c = CString::new(trace_path.to_string_lossy().as_ref()).unwrap();
            nzea_ffi::nzea_trace_open(self.sim_ptr, path_c.as_ptr());
        }
    }

    fn state(&self) -> &State<P> {
        &self.state
    }

    fn state_mut(&mut self) -> &mut State<P> {
        &mut self.state
    }

    fn step_once<const TRACE: u64>(&mut self) -> Result<(), remu_simulator::SimulatorInnerError> {
        // NZEA must be updated before each step: when set_nzea runs in init(), dut_model is still
        // a local in Harness::new; it is then moved into the Harness struct and the old address
        // becomes invalid. In step_once, self is the final location, so we must set it again.
        unsafe {
            dpi::set_nzea(self as *mut Self as *mut dyn NzeaDpi);
        }
        let do_wavetrace = TraceFlags::wavetrace(TRACE) && IS_DUT;
        unsafe {
            nzea_ffi::nzea_set_clock(self.sim_ptr, 0);
            nzea_ffi::nzea_eval(self.sim_ptr);
            if do_wavetrace {
                nzea_ffi::nzea_trace_dump(self.sim_ptr);
            }
            nzea_ffi::nzea_set_clock(self.sim_ptr, 1);
            nzea_ffi::nzea_eval(self.sim_ptr);
            if do_wavetrace {
                nzea_ffi::nzea_trace_dump(self.sim_ptr);
            }
        }
        Ok(())
    }

    fn state_exec(&mut self, subcmd: &StateCmd) -> Result<(), SimulatorInnerError> {
        self.state.execute(subcmd).map_err(from_state_error)?;
        Ok(())
    }
}

impl<P: SimulatorPolicy + 'static, const IS_DUT: bool> Drop for SimulatorNzea<P, IS_DUT> {
    fn drop(&mut self) {
        unsafe {
            nzea_ffi::nzea_destroy(self.sim_ptr);
            dpi::clear_nzea();
        }
    }
}

impl<P: SimulatorPolicy + 'static> SimulatorDut for SimulatorNzea<P, true> {}
