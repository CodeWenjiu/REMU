//! MuJoCo runtime for the arm: engine access, the shared simulation core, the
//! engine renderer, and the winit/Slint monitor. Consumed by the bus arm device
//! (remu_state) and the native host arm backend (remu_hal_host).

// The engine/render surface is intentionally wider than what the consumers
// use today (kept whole so the FFI transcription stays reviewable).
#![allow(dead_code)]

remu_macro::mod_prv!(engine, error, render, sim, triple, window);

pub use engine::{ARM_XML, ModelSource, Session};
pub use error::MujocoError;
pub use sim::{ArmCore, ArmSnapshot, MocapCommand, ScanParams, Scanner};
pub use triple::TripleBuffer;
pub use window::ArmWindow;
