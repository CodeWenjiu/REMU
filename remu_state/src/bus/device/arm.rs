//! Arm device: MMIO actuator-target registers driving a MuJoCo model, with
//! each committed state published to the monitor window (`armview`).
//!
//! A bus master writes actuator targets (joint position setpoints; models
//! with a gripper use the trailing channel for it) and a commit register; a
//! commit applies the targets, advances the simulation by `steps` microsteps,
//! and publishes a snapshot for the render thread. `NU`/`NQ` report the
//! model's channel counts; positions and simulated time are readable back
//! through the same register window.
//!
//! By default the built-in 3-DOF toy arm loads; setting `REMU_ARM_MODEL` (an
//! MJCF path, e.g. the Menagerie Panda scene) loads a real model instead.
//! The simulation core lives in `remu_state_mujoco` (shared with the native
//! host backend); this device is the register surface over it.
//!
//! Bring-up shape: the register file is reachable directly for now; the
//! external-bus layer (i2c controller + target protocol) re-homes it behind a
//! bus later, without changing the model or the monitor path.

use std::sync::Arc;

use crate::bus::{BusError, device::DeviceContext};

use remu_state_mujoco::{ARM_XML, ArmCore, ModelSource};

use super::window::WindowHost;

/// Register-map capacity of the target block; a model may use fewer channels
/// (the `NU` register reports the truth).
const MAX_TARGETS: usize = 8;
/// Register-map capacity of the position block.
const MAX_QPOS: usize = 10;

// Register offsets (32-bit registers; f32 values travel as their bit pattern).
const REG_NU: usize = 0x00; // r: actuator targets (ctrl channels)
const REG_NQ: usize = 0x04; // r: joint positions
const REG_TARGET: usize = 0x08; // r/w: actuator targets, stride 4
const REG_COMMIT: usize = 0x28; // w: apply targets + advance + publish
const REG_STEPS: usize = 0x2C; // r/w: microsteps per commit (default 16)
const REG_QPOS: usize = 0x30; // r: joint positions (radians), stride 4
const REG_TIME: usize = 0x58; // r: simulated time (seconds)
const REG_COMMITS: usize = 0x5C; // r: completed commits (wrapping u32)
const REG_STATUS: usize = 0x60; // r: bit 0 = armview monitor window alive
const REG_TIMESTEP: usize = 0x64; // r: model timestep (seconds)
const ARM_REG_SIZE: usize = 0x68;

pub(super) struct Arm {
    core: ArmCore,
    /// Where the model came from; reused to open the monitor window.
    source: ModelSource,
    /// The shared window host (holds the armview monitor window).
    window: Option<Arc<WindowHost>>,
}

// The engine session is single-threaded by design, but a bus device must
// satisfy `Send + Sync`. Device access is serialized (every accessor takes
// `&mut self` and runs on the simulator thread that built the session), and
// state crosses to the render thread only as snapshots (see `remu_state_mujoco`),
// so these impls just satisfy the boxed-device contract.
unsafe impl Send for Arm {}
unsafe impl Sync for Arm {}

impl Arm {
    pub(super) fn new() -> Self {
        let source = ModelSource::from_env(ARM_XML);
        let core = ArmCore::load(&source).expect("arm: load model via libmujoco");
        assert!(
            core.nu() <= MAX_TARGETS && core.nq() <= MAX_QPOS,
            "arm: model (nu={}, nq={}) exceeds the register map",
            core.nu(),
            core.nq()
        );
        Self {
            core,
            source,
            window: None,
        }
    }
}

impl super::DeviceAccess for Arm {
    fn name(&self) -> &str {
        "arm"
    }

    fn size(&self) -> usize {
        ARM_REG_SIZE
    }

    fn attach_context(&mut self, ctx: &DeviceContext) {
        // Loading the device opens the monitor window (same trigger shape as
        // the display device).
        let window = ctx.window();
        window.open_arm_view(self.source.clone(), self.core.snapshot());
        self.window = Some(Arc::clone(window));
    }

    fn read_32(&mut self, offset: usize) -> Result<u32, BusError> {
        Ok(match offset {
            REG_NU => self.core.nu() as u32,
            REG_NQ => self.core.nq() as u32,
            o if (REG_TARGET..REG_TARGET + 4 * self.core.nu()).contains(&o) => {
                self.core.target((o - REG_TARGET) / 4).to_bits()
            }
            o if (REG_QPOS..REG_QPOS + 4 * self.core.nq()).contains(&o) => {
                self.core.read_qpos_f32((o - REG_QPOS) / 4).to_bits()
            }
            REG_STEPS => self.core.steps(),
            REG_TIME => (self.core.time() as f32).to_bits(),
            REG_COMMITS => self.core.commits(),
            REG_STATUS => self.window.as_ref().map_or(0, |w| w.armview_alive() as u32),
            REG_TIMESTEP => (self.core.timestep() as f32).to_bits(),
            REG_COMMIT => 0, // write-only
            _ => {
                return Err(BusError::UnsupportedAccessWidth(
                    32,
                    std::backtrace::Backtrace::capture(),
                ));
            }
        })
    }

    fn write_32(&mut self, offset: usize, value: u32) -> Result<(), BusError> {
        match offset {
            o if (REG_TARGET..REG_TARGET + 4 * self.core.nu()).contains(&o) => {
                self.core
                    .set_target((o - REG_TARGET) / 4, f32::from_bits(value));
            }
            REG_STEPS => self.core.set_steps(value),
            REG_COMMIT => self.core.commit(),
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::bus::device::DeviceAccess as _;

    use super::*;

    #[test]
    fn commit_applies_targets_and_advances() {
        let mut a = Arm::new();
        for (i, v) in [0.5f32, -0.4, 0.3].iter().enumerate() {
            a.write_32(REG_TARGET + 4 * i, v.to_bits()).unwrap();
        }
        assert_eq!(a.read_32(REG_COMMITS).unwrap(), 0);
        a.write_32(REG_COMMIT, 1).unwrap();
        assert_eq!(a.read_32(REG_COMMITS).unwrap(), 1);

        // 16 steps × 1 ms.
        let t = f32::from_bits(a.read_32(REG_TIME).unwrap());
        assert!((t - 0.016).abs() < 1e-6, "time = {t}");

        // The servos moved toward the targets and stayed in range.
        for i in 0..3 {
            let q = f32::from_bits(a.read_32(REG_QPOS + 4 * i).unwrap());
            assert!(q.abs() > 0.0, "joint {i} did not move");
            assert!(q.abs() <= 1.5, "joint {i} out of range: {q}");
        }

        // The published snapshot mirrors the register readback.
        let q0 = f32::from_bits(a.read_32(REG_QPOS).unwrap());
        let snapshot = a.core.snapshot();
        let snap = snapshot.read();
        assert!((snap.qpos[0] as f32 - q0).abs() < 1e-9);
    }

    #[test]
    fn steps_register_controls_advance() {
        let mut a = Arm::new();
        a.write_32(REG_STEPS, 64).unwrap();
        a.write_32(REG_COMMIT, 1).unwrap();
        let t = f32::from_bits(a.read_32(REG_TIME).unwrap());
        assert!((t - 0.064).abs() < 1e-6, "time = {t}");
        assert_eq!(a.read_32(REG_STEPS).unwrap(), 64);
    }

    #[test]
    fn status_reports_no_monitor_without_context() {
        // No window host is attached in a unit test: the monitor bit reads 0.
        let mut a = Arm::new();
        assert_eq!(a.read_32(REG_STATUS).unwrap() & 1, 0);
    }

    #[test]
    fn counts_and_timestep_report_the_model() {
        let mut a = Arm::new();
        assert_eq!(a.read_32(REG_NU).unwrap(), 3);
        assert_eq!(a.read_32(REG_NQ).unwrap(), 3);
        let ts = f32::from_bits(a.read_32(REG_TIMESTEP).unwrap());
        assert!((ts - 0.001).abs() < 1e-9, "timestep = {ts}");
    }
}
