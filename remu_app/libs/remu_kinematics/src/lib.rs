//! Portable forward and inverse kinematics for serial arms.
//!
//! `no_std` by default (`std` is opt-in). The floating-point backend is a
//! compile-time choice — exactly one of `math-std`, `math-libm` (default),
//! or `math-micromath` must be enabled.

#![cfg_attr(not(feature = "std"), no_std)]

remu_macro::mod_prv!(math);

#[cfg(not(any(
    feature = "math-std",
    feature = "math-libm",
    feature = "math-micromath"
)))]
compile_error!("remu_kinematics: enable a math backend (math-std, math-libm, or math-micromath)");

#[cfg(any(
    all(feature = "math-std", feature = "math-libm"),
    all(feature = "math-std", feature = "math-micromath"),
    all(feature = "math-libm", feature = "math-micromath")
))]
compile_error!("remu_kinematics: enable exactly one math backend");
