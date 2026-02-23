//! Nzea simulator: minimal framework; DPI bus_read/bus_write dispatch to State via dpi module.

use remu_state::State;
use remu_types::TracerDyn;

use remu_simulator::{
    SimulatorCore, SimulatorDut, SimulatorOption, SimulatorPolicy, SimulatorPolicyOf,
};

use crate::dpi::{self, DpiBus};

/// Nzea simulator instance. Holds CPU/memory state; step logic to be implemented.
pub struct SimulatorNzea<P: SimulatorPolicy, const IS_DUT: bool> {
    state: State<P>,
    _tracer: TracerDyn,
}

impl<P: SimulatorPolicy, const IS_DUT: bool> SimulatorPolicyOf for SimulatorNzea<P, IS_DUT> {
    type Policy = P;
}

impl<P: SimulatorPolicy + 'static, const IS_DUT: bool> SimulatorCore<P> for SimulatorNzea<P, IS_DUT> {
    fn new(opt: SimulatorOption, tracer: TracerDyn) -> Self {
        crate::nzea_ffi::Nzea::init();
        Self {
            state: State::new(opt.state.clone(), tracer.clone(), IS_DUT),
            _tracer: tracer,
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
        self.set_dpi_bus_for_step();
        crate::nzea_ffi::Nzea::step();
        Self::clear_dpi_bus_for_step();
        Ok(())
    }
}

impl<P: SimulatorPolicy + 'static, const IS_DUT: bool> SimulatorNzea<P, IS_DUT> {
    /// Set the global DPI bus context to this simulator's state. Call before driving Verilator RTL.
    pub fn set_dpi_bus_for_step(&mut self) {
        unsafe {
            dpi::set_dpi_bus(self.state_mut() as *mut State<P> as *mut dyn DpiBus);
        }
    }

    /// Clear the global DPI bus context. Call after driving Verilator RTL.
    pub fn clear_dpi_bus_for_step() {
        unsafe {
            dpi::clear_dpi_bus();
        }
    }
}

impl<P: SimulatorPolicy + 'static> SimulatorDut for SimulatorNzea<P, true> {}
