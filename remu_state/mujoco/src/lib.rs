//! MuJoCo runtime for the arm: the dlopen'd engine FFI, state-API sessions,
//! the armview OpenGL renderer + monitor-window glue, and the shared
//! simulation core. Consumed by the bus arm device (remu_state) and the
//! native host arm backend (remu_hal_host).

// The engine/render surface is intentionally wider than what the consumers
// use today (kept whole so the FFI transcription stays reviewable).
#![allow(dead_code)]

remu_macro::mod_prv!(
    arm_window, armview, arm_core, error, ffi, model, runtime, session, triple, view
);

pub use arm_core::{ArmCore, ArmSnapshot};
pub use arm_window::ArmWindow;
pub use error::MujocoError;
pub use model::{ARM_XML, ModelSource};
pub use session::Session;
pub use triple::TripleBuffer;
