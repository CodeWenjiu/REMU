//! Build-time MJCF reading for app-side kinematics.
//!
//! Resolves the model declared by `[package.metadata.remu] arm-model`
//! (relative to `$MUJOCO_MENAGERIE_DIR`), picks the serial hinge chain out of
//! the body tree, and emits a `kinematics.rs` of `const` data (in terms of the
//! `kinematics` crate's types) that the app `include!`s — zero runtime
//! parsing, zero runtime cost.

remu_macro::mod_prv!(emit, extract, parse);
remu_macro::mod_pub!(build);
