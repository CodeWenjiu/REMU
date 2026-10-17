//! Arm device (MMIO): actuator targets, commit, and state readback.
//!
//! The device drives a MuJoCo model (built-in toy arm, or a real model when
//! the host ran with `REMU_ARM_MODEL`); each commit advances the simulation
//! and publishes a frame to the armview monitor window. Register layout
//! matches `remu_state`'s arm device (`bus/device/arm.rs`). `NU`/`NQ` report
//! the loaded model's target and joint counts.

use crate::addresses::ARM_BASE;
use core::ptr::{read_volatile, write_volatile};

/// Register-map capacity of the target block; the model may use fewer
/// channels (read [`read_nu`]).
pub const ARM_TARGETS_MAX: usize = 8;
/// Register-map capacity of the position block.
pub const ARM_QPOS_MAX: usize = 10;

// Register offsets (32-bit registers; f32 values travel as their bit pattern).
const REG_NU: usize = 0x00; // r: actuator target count (ctrl channels)
const REG_NQ: usize = 0x04; // r: joint position count
const REG_TARGET: usize = 0x08; // r/w: actuator targets, stride 4
const REG_COMMIT: usize = 0x28; // w: apply targets + advance + publish
const REG_STEPS: usize = 0x2C; // r/w: microsteps per commit (default 16)
const REG_QPOS: usize = 0x30; // r: joint positions (radians), stride 4
const REG_TIME: usize = 0x58; // r: simulated time (seconds)
const REG_COMMITS: usize = 0x5C; // r: completed commits (wrapping u32)
const REG_STATUS: usize = 0x60; // r: bit 0 = armview monitor window alive
const REG_TIMESTEP: usize = 0x64; // r: model timestep (seconds)

/// Actuator target count of the loaded model (joints, plus one gripper
/// channel for models that have one).
#[inline]
pub fn read_nu() -> u32 {
    unsafe { read_volatile((ARM_BASE + REG_NU) as *const u32) }
}

/// Joint position count of the loaded model.
#[inline]
pub fn read_nq() -> u32 {
    unsafe { read_volatile((ARM_BASE + REG_NQ) as *const u32) }
}

/// Set one actuator target; applied by the next [`commit`]. For joint servos
/// the value is the joint angle (radians); a gripper channel uses the model's
/// own range (e.g. 0..255, 255 = open, for a Panda).
#[inline]
pub fn set_target(channel: usize, value: f32) {
    if channel < ARM_TARGETS_MAX {
        unsafe {
            write_volatile(
                (ARM_BASE + REG_TARGET + channel * 4) as *mut u32,
                value.to_bits(),
            )
        }
    }
}

/// Apply the targets and advance the simulation by `steps` microsteps,
/// publishing a new frame to the monitor window.
#[inline]
pub fn commit() {
    unsafe { write_volatile((ARM_BASE + REG_COMMIT) as *mut u32, 1) }
}

/// Set the physics microsteps per commit (clamped to ≥ 1; the default of 16
/// uses as much simulated time as possible per 60 Hz frame at a 1 ms
/// timestep — read [`read_timestep`] to scale for other models).
#[inline]
pub fn set_steps(steps: u32) {
    unsafe { write_volatile((ARM_BASE + REG_STEPS) as *mut u32, steps) }
}

/// Read a joint position (radians) as of the last commit.
#[inline]
pub fn read_qpos(joint: usize) -> f32 {
    if joint < ARM_QPOS_MAX {
        f32::from_bits(unsafe { read_volatile((ARM_BASE + REG_QPOS + joint * 4) as *const u32) })
    } else {
        0.0
    }
}

/// Simulated time (seconds) as of the last commit.
#[inline]
pub fn read_time() -> f32 {
    f32::from_bits(unsafe { read_volatile((ARM_BASE + REG_TIME) as *const u32) })
}

/// Number of completed commits (wrapping u32).
#[inline]
pub fn read_commits() -> u32 {
    unsafe { read_volatile((ARM_BASE + REG_COMMITS) as *const u32) }
}

/// Model timestep (seconds); pick `steps` so one commit covers ~16 ms.
#[inline]
pub fn read_timestep() -> f32 {
    f32::from_bits(unsafe { read_volatile((ARM_BASE + REG_TIMESTEP) as *const u32) })
}

/// Whether the armview monitor window is currently open.
///
/// Returns `false` once the window is closed (or if it could not be opened),
/// so demo controllers can stop and exit instead of controlling with no
/// visible monitor — the arm-device counterpart of `display_alive`.
#[inline]
pub fn armview_alive() -> bool {
    unsafe { read_volatile((ARM_BASE + REG_STATUS) as *const u32) & 1 != 0 }
}
