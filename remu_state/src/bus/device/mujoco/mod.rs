//! MuJoCo engine wrapper for the device layer: a dlopen2 FFI table, the lazy
//! loader, a state-API session, the armview renderer, and its windowed
//! backend. Consumed by the arm device and the shared window host.

// The engine/render surface is intentionally wider than what the device and
// monitor use today (kept whole so the FFI transcription stays reviewable).
#![allow(dead_code)]

remu_macro::mod_prv!(armview, error, ffi, runtime, session, view);

pub(crate) use armview::ArmView;
pub(crate) use session::Session;
