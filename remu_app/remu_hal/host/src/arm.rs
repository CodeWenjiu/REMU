//! Arm device on hosted targets: unavailable.
//!
//! The arm device only exists on the remu bus (the hosted HAL has no MMIO).
//! These stubs keep guest-only apps compiling for host targets; calling one is
//! a configuration error and fails loudly (`just run-app` refuses host runs of
//! guest-only apps via `[package.metadata.remu] isas`).

/// Number of joints of the arm model.
pub const ARM_JOINTS: usize = 3;

const UNAVAILABLE: &str =
    "remu_hal: the arm device only exists on the remu bus (run with --platform remu)";

pub fn set_target(_joint: usize, _value: f32) {
    panic!("{UNAVAILABLE}")
}

pub fn commit() {
    panic!("{UNAVAILABLE}")
}

pub fn set_steps(_steps: u32) {
    panic!("{UNAVAILABLE}")
}

pub fn read_qpos(_joint: usize) -> f32 {
    panic!("{UNAVAILABLE}")
}

pub fn read_time() -> f32 {
    panic!("{UNAVAILABLE}")
}

pub fn read_commits() -> u32 {
    panic!("{UNAVAILABLE}")
}

pub fn armview_alive() -> bool {
    panic!("{UNAVAILABLE}")
}
