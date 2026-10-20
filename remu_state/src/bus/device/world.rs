//! Shared world: the one MuJoCo model every arm-family device register
//! surface observes (arm actuators, scan rays, future sensors).
//!
//! Built by `Bus::new` when any configured device needs it (like the window
//! host), injected through [`DeviceContext`], and owned jointly by those
//! devices through `Arc` clones — so all of them see one session, one commit
//! sequence, one physical state.

use std::sync::{Arc, Mutex};

use remu_state_mujoco::{ARM_XML, ArmCore, ArmSnapshot, MocapCommand, ModelSource, TripleBuffer};

/// The engine session never actually crosses threads — every accessor runs on
/// the simulator thread (device MMIO handlers are serialized there), and the
/// mutex keeps the engine calls serialized beyond that. The handle is shared
/// as an `Arc` only so multiple register surfaces observe the same world;
/// `Session`'s own `!Send` marker keeps the engine itself out of reach.
unsafe impl Send for WorldHost {}
unsafe impl Sync for WorldHost {}

pub(crate) struct WorldHost {
    source: ModelSource,
    core: Mutex<ArmCore>,
}

impl WorldHost {
    /// Load the world model (the arm device's `REMU_ARM_MODEL`, or the built-in
    /// toy arm), park its interactive obstacles, and publish the state.
    pub(crate) fn new() -> Self {
        let source = ModelSource::from_env(ARM_XML);
        let mut core = ArmCore::load(&source).expect("world: load model via libmujoco");
        core.park_mocap();
        Self {
            source,
            core: Mutex::new(core),
        }
    }

    /// Where the model came from (used to open the monitor window).
    pub(crate) fn source(&self) -> &ModelSource {
        &self.source
    }

    /// The render-thread snapshot channel of the world.
    pub(crate) fn snapshot(&self) -> Arc<TripleBuffer<ArmSnapshot>> {
        self.core.lock().unwrap().snapshot()
    }

    /// The UI -> core obstacle command channel of the world.
    pub(crate) fn mocap(&self) -> Arc<TripleBuffer<MocapCommand>> {
        self.core.lock().unwrap().mocap_command()
    }

    /// Run `f` with the shared core; every access is serialized.
    pub(crate) fn with_core<R>(&self, f: impl FnOnce(&mut ArmCore) -> R) -> R {
        f(&mut self.core.lock().unwrap())
    }
}
