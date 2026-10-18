//! Model-derived kinematics, generated at build time by `mjcf` from the MJCF
//! declared by `[package.metadata.remu] arm-model` (or from `REMU_ARM_MODEL`
//! when set — the composed scene that carries the demo obstacles).
//!
//! `include!` textually splices `$OUT_DIR/kinematics.rs` written by `build.rs`:
//! the model is parsed on the host at build time, and only `const` data ships.
//! The allows cover the generated file: unused constants kept for later use,
//! `pub` items in a private module, and model-literal angles like `1.57079`.
#![allow(dead_code, unreachable_pub, clippy::approx_constant)]

include!(concat!(env!("OUT_DIR"), "/kinematics.rs"));
