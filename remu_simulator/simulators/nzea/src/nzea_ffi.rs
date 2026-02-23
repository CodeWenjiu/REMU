//! FFI to C++ glue (nzea_wrapper.cpp): create/destroy/drive the Verilated model.
//! DPI-C (bus_read, bus_write) are implemented in dpi.rs and use the global DPI_BUS set by the simulator.

use std::ffi::c_void;
use std::sync::{Mutex, OnceLock};

unsafe extern "C" {
    fn nzea_create() -> *mut c_void;
    fn nzea_destroy(sim: *mut c_void);
    fn nzea_set_clock(sim: *mut c_void, val: i32);
    fn nzea_set_reset(sim: *mut c_void, val: i32);
    fn nzea_eval(sim: *mut c_void);
}

/// Opaque handle to the Verilated C++ sim. Send is required to store in Mutex.
pub struct Nzea {
    sim_ptr: *mut c_void,
    pub cycles: u64,
    alive: bool,
}

unsafe impl Send for Nzea {}

/// Global singleton: the one Verilated instance, used by Nzea::step() and by DPI callbacks (via DPI_BUS set before eval).
pub static NZEA_INSTANCE: OnceLock<Mutex<Nzea>> = OnceLock::new();

impl Nzea {
    /// Create and reset the C++ sim; store in NZEA_INSTANCE. Idempotent after first successful init.
    pub fn init() -> bool {
        if NZEA_INSTANCE.get().is_some() {
            return true;
        }
        let sim_ptr = unsafe { nzea_create() };
        if sim_ptr.is_null() {
            return false;
        }
        let nzea = Nzea {
            sim_ptr,
            cycles: 0,
            alive: true,
        };
        unsafe {
            nzea_set_reset(nzea.sim_ptr, 1);
            for _ in 0..100 {
                nzea_set_clock(nzea.sim_ptr, 0);
                nzea_eval(nzea.sim_ptr);
                nzea_set_clock(nzea.sim_ptr, 1);
                nzea_eval(nzea.sim_ptr);
            }
            nzea_set_reset(nzea.sim_ptr, 0);
        }
        NZEA_INSTANCE.set(Mutex::new(nzea)).is_ok()
    }

    /// Run one clock cycle (low -> eval -> high -> eval). Call after set_dpi_bus_for_step(), then clear_dpi_bus_for_step().
    pub fn step() {
        if let Some(mux) = NZEA_INSTANCE.get() {
            let mut nzea = mux.lock().expect("nzea lock");
            if !nzea.alive {
                return;
            }
            unsafe {
                nzea_set_clock(nzea.sim_ptr, 0);
                nzea_eval(nzea.sim_ptr);
                nzea_set_clock(nzea.sim_ptr, 1);
                nzea_eval(nzea.sim_ptr);
            }
            nzea.cycles += 1;
        }
    }

    /// Drop the C++ sim; step() will no-op afterwards.
    pub fn shutdown() {
        if let Some(mux) = NZEA_INSTANCE.get() {
            let mut nzea = mux.lock().expect("nzea lock");
            if nzea.alive {
                unsafe { nzea_destroy(nzea.sim_ptr) };
                nzea.sim_ptr = std::ptr::null_mut();
                nzea.alive = false;
            }
        }
    }
}
