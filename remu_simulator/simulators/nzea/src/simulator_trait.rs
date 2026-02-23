//! Nzea simulator: DPI bus_read/bus_write dispatch via global pointer; lifecycle only at init/drop.

use std::ffi::c_void;

use remu_state::State;
use remu_types::TracerDyn;

use remu_simulator::{
    SimulatorCore, SimulatorDut, SimulatorOption, SimulatorPolicy, SimulatorPolicyOf,
};

use crate::dpi::{self, NzeaDpi};
use crate::nzea_ffi;

pub struct SimulatorNzea<P: SimulatorPolicy + 'static, const IS_DUT: bool> {
    state: State<P>,
    sim_ptr: *mut c_void,
    _tracer: TracerDyn,
    nzea_registered: std::cell::Cell<bool>,
}

impl<P: SimulatorPolicy, const IS_DUT: bool> SimulatorPolicyOf for SimulatorNzea<P, IS_DUT> {
    type Policy = P;
}

impl<P: SimulatorPolicy + 'static, const IS_DUT: bool> SimulatorCore<P> for SimulatorNzea<P, IS_DUT> {
    fn new(opt: SimulatorOption, tracer: TracerDyn) -> Self {
        let sim_ptr = unsafe { nzea_ffi::nzea_create() };
        assert!(!sim_ptr.is_null(), "nzea_create failed");

        unsafe {
            nzea_ffi::nzea_set_reset(sim_ptr, 1);
            for _ in 0..100 {
                nzea_ffi::nzea_set_clock(sim_ptr, 0);
                nzea_ffi::nzea_eval(sim_ptr);
                nzea_ffi::nzea_set_clock(sim_ptr, 1);
                nzea_ffi::nzea_eval(sim_ptr);
            }
            nzea_ffi::nzea_set_reset(sim_ptr, 0);
        }

        let state = State::new(opt.state.clone(), tracer.clone(), IS_DUT);
        Self {
            state,
            sim_ptr,
            _tracer: tracer,
            nzea_registered: std::cell::Cell::new(false),
        }
    }

    fn state(&self) -> &State<P> {
        &self.state
    }

    fn state_mut(&mut self) -> &mut State<P> {
        &mut self.state
    }

    fn step_once<const ITRACE: bool>(&mut self) -> Result<(), remu_simulator::SimulatorInnerError> {
        let _ = ITRACE;
        if !self.nzea_registered.get() {
            unsafe { dpi::set_nzea(self as *mut Self as *mut dyn NzeaDpi); }
            self.nzea_registered.set(true);
        }
        unsafe {
            nzea_ffi::nzea_set_clock(self.sim_ptr, 0);
            nzea_ffi::nzea_eval(self.sim_ptr);
            nzea_ffi::nzea_set_clock(self.sim_ptr, 1);
            nzea_ffi::nzea_eval(self.sim_ptr);
        }
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
