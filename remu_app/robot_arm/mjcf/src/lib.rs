//! Build-time MJCF reading for app-side kinematics.
//!
//! Resolves the model declared by `[package.metadata.remu] arm-model`
//! (relative to `$MUJOCO_MENAGERIE_DIR`), picks the serial hinge chain out of
//! the body tree, and emits a self-contained `kinematics.rs` of `const` data
//! that the app `include!`s — zero runtime parsing, zero runtime cost.

remu_macro::mod_prv!(emit, extract, parse);
remu_macro::mod_pub!(build);
