//! Model-derived kinematics, generated at build time by `mjcf` from the MJCF
//! declared by `[package.metadata.remu] arm-model`.
#![allow(dead_code, unreachable_pub, clippy::approx_constant)]

include!(concat!(env!("OUT_DIR"), "/kinematics.rs"));
