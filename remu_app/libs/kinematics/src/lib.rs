//! Portable forward and inverse kinematics for serial arms: a const-generic
//! [`Chain`](prelude::Chain) core (forward kinematics over quaternion joint
//! frames; dense damped least-squares IK for weighted position/orientation
//! tasks with joint limits, capsule-obstacle clearance and classified
//! failures) plus a feature-selected floating point backend.
//!
//! `no_std` by default (`std` is opt-in). Exactly one of `math-std`,
//! `math-libm` (default), or `math-micromath` must be enabled.

#![cfg_attr(not(feature = "std"), no_std)]

remu_macro::mod_prv!(chain, math, solve, vec);
remu_macro::mod_pub!(prelude);

#[cfg(not(any(
    feature = "math-std",
    feature = "math-libm",
    feature = "math-micromath"
)))]
compile_error!("kinematics: enable a math backend (math-std, math-libm, or math-micromath)");

#[cfg(any(
    all(feature = "math-std", feature = "math-libm"),
    all(feature = "math-std", feature = "math-micromath"),
    all(feature = "math-libm", feature = "math-micromath")
))]
compile_error!("kinematics: enable exactly one math backend");
