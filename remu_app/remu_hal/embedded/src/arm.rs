//! Arm device (MMIO): joint targets, commit, and state readback.
//!
//! The device drives a MuJoCo model; each commit advances the simulation and
//! publishes a frame to the armview monitor window. Register layout matches
//! `remu_state`'s arm device (`bus/device/arm.rs`).

use crate::addresses::ARM_BASE;
use core::ptr::{read_volatile, write_volatile};

/// Number of joints of the arm model.
pub const ARM_JOINTS: usize = 3;

// Register offsets (32-bit registers; f32 values travel as their bit pattern).
const REG_TARGET: usize = 0x00; // r/w: joint targets (radians), stride 4
const REG_COMMIT: usize = 0x0C; // w: apply targets + advance + publish
const REG_STEPS: usize = 0x10; // r/w: microsteps per commit (default 16)
const REG_QPOS: usize = 0x14; // r: joint positions (radians), stride 4
const REG_TIME: usize = 0x20; // r: simulated time (seconds)
const REG_COMMITS: usize = 0x24; // r: completed commits (wrapping u32)
const REG_STATUS: usize = 0x28; // r: bit 0 = armview monitor window alive

/// Set one joint target (radians); applied by the next [`commit`].
#[inline]
pub fn set_target(joint: usize, value: f32) {
    if joint < ARM_JOINTS {
        unsafe {
            write_volatile(
                (ARM_BASE + REG_TARGET + joint * 4) as *mut u32,
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

/// Set the physics microsteps per commit (clamped to ≥ 1; default 16 ≈ one
/// 60 Hz frame at a 1 ms timestep).
#[inline]
pub fn set_steps(steps: u32) {
    unsafe { write_volatile((ARM_BASE + REG_STEPS) as *mut u32, steps) }
}

/// Read a joint position (radians) as of the last commit.
#[inline]
pub fn read_qpos(joint: usize) -> f32 {
    if joint < ARM_JOINTS {
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

/// Whether the armview monitor window is currently open.
///
/// Returns `false` once the window is closed (or if it could not be opened),
/// so demo controllers can stop and exit instead of controlling with no
/// visible monitor — the arm-device counterpart of `display_alive`.
#[inline]
pub fn armview_alive() -> bool {
    unsafe { read_volatile((ARM_BASE + REG_STATUS) as *const u32) & 1 != 0 }
}
