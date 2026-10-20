//! The shared MuJoCo world on host: one engine session per thread (guest apps
//! are single-threaded), created on first use and observed by every arm-family
//! device backend (arm actuators, scan rays) — the hosted counterpart of the
//! remu bus's `WorldHost`.

use std::cell::RefCell;

use remu_state_mujoco::{ARM_XML, ArmCore, ModelSource};

thread_local! {
    static WORLD: RefCell<Option<ArmCore>> = const { RefCell::new(None) };
}

/// Run `f` with the world core, loading the model and requesting the monitor
/// window on first use.
pub(crate) fn with_core<R>(f: impl FnOnce(&mut ArmCore) -> R) -> R {
    WORLD.with(|slot| {
        let mut slot = slot.borrow_mut();
        let core = slot.get_or_insert_with(|| {
            let source = ModelSource::from_env(ARM_XML);
            let mut core = ArmCore::load(&source).expect("arm: load model via libmujoco");
            core.park_mocap();
            crate::arm::request_monitor(source, core.snapshot(), core.mocap_command());
            core
        });
        f(core)
    })
}
