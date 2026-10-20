//! Arm device: MMIO actuator-target registers over the shared MuJoCo world,
//! with each committed state published to the monitor window (`armview`).
//!
//! A bus master writes actuator targets (joint position setpoints; models
//! with a gripper use the trailing channel for it) and a commit register; a
//! commit applies the targets, advances the shared world by `steps`
//! microsteps, and publishes a snapshot for the render thread. `NU`/`NQ`
//! report the model's channel counts; positions and simulated time are
//! readable back through the same register window.
//!
//! The world itself belongs to [`WorldHost`] (shared with the scan device and
//! future sensors); this device is the register surface over it, and loading
//! it opens the armview monitor window.
//!
//! Bring-up shape: the register file is reachable directly for now; the
//! external-bus layer (i2c controller + target protocol) re-homes it behind a
//! bus later, without changing the model or the monitor path.

use std::sync::Arc;

use remu_state_mujoco::ArmWindow;

use crate::bus::{BusError, device::DeviceContext};

use super::WorldHost;
use super::window::{AuxRequest, WindowHost};

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
    /// The shared world; attached at bus build time.
    world: Option<Arc<WorldHost>>,
    /// The shared window host (holds the armview monitor window).
    window: Option<Arc<WindowHost>>,
}

impl Arm {
    pub(super) fn new() -> Self {
        Self {
            world: None,
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
        let world = Arc::clone(ctx.world());
        let (nu, nq) = world.with_core(|c| (c.nu(), c.nq()));
        assert!(
            nu <= MAX_TARGETS && nq <= MAX_QPOS,
            "arm: model (nu={nu}, nq={nq}) exceeds the register map"
        );
        // Loading the device opens the monitor window when a window host is
        // present (production always has one; unit tests attach world-only).
        if let Some(window) = ctx.maybe_window() {
            let source = world.source().clone();
            let snapshot = world.snapshot();
            let mocap = world.mocap();
            window.open_aux(AuxRequest::new(move |event_loop| {
                let (window, view) = ArmWindow::open(event_loop, &source, snapshot, mocap)?;
                Ok((window, Box::new(view)))
            }));
            self.window = Some(Arc::clone(window));
        }
        self.world = Some(world);
    }

    fn read_32(&mut self, offset: usize) -> Result<u32, BusError> {
        let world = self.world.as_ref().expect("arm: world not attached");
        let (nu, nq) = world.with_core(|c| (c.nu(), c.nq()));
        Ok(match offset {
            REG_NU => nu as u32,
            REG_NQ => nq as u32,
            o if (REG_TARGET..REG_TARGET + 4 * nu).contains(&o) => {
                world.with_core(|c| c.target((o - REG_TARGET) / 4).to_bits())
            }
            o if (REG_QPOS..REG_QPOS + 4 * nq).contains(&o) => {
                world.with_core(|c| c.read_qpos_f32((o - REG_QPOS) / 4).to_bits())
            }
            REG_STEPS => world.with_core(|c| c.steps()),
            REG_TIME => world.with_core(|c| (c.time() as f32).to_bits()),
            REG_COMMITS => world.with_core(|c| c.commits()),
            REG_STATUS => self.window.as_ref().map_or(0, |w| w.aux_alive() as u32),
            REG_TIMESTEP => world.with_core(|c| (c.timestep() as f32).to_bits()),
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
        let world = self.world.as_ref().expect("arm: world not attached");
        let nu = world.with_core(|c| c.nu());
        match offset {
            o if (REG_TARGET..REG_TARGET + 4 * nu).contains(&o) => {
                world.with_core(|c| c.set_target((o - REG_TARGET) / 4, f32::from_bits(value)));
            }
            REG_STEPS => world.with_core(|c| c.set_steps(value)),
            REG_COMMIT => world.with_core(|c| c.commit()),
            _ => {}
        }
        Ok(())
    }
}

impl super::window::AuxWindow for ArmWindow {
    fn redraw(&mut self) {
        ArmWindow::redraw(self);
    }

    fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        ArmWindow::resize(self, size);
    }

    fn cursor_moved(&mut self, x: f64, y: f64) {
        ArmWindow::cursor_moved(self, x, y);
    }

    fn pointer_exited(&mut self) {
        ArmWindow::cursor_left(self);
    }

    fn set_left(&mut self, down: bool) {
        ArmWindow::set_left(self, down);
    }

    fn wheel(&mut self, delta: winit::event::MouseScrollDelta) {
        ArmWindow::wheel(self, delta);
    }
}

#[cfg(test)]
mod tests {
    use crate::bus::device::DeviceAccess as _;

    use super::*;

    /// A test context over the built-in toy-arm world, no window.
    fn test_ctx() -> DeviceContext {
        let mut ctx = DeviceContext::default();
        ctx.set_world(Arc::new(WorldHost::new()));
        ctx
    }

    fn arm(ctx: &DeviceContext) -> Arm {
        let mut a = Arm::new();
        a.attach_context(ctx);
        a
    }

    #[test]
    fn commit_applies_targets_and_advances() {
        let ctx = test_ctx();
        let mut a = arm(&ctx);
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
        let snapshot = ctx.world().snapshot();
        let snap = snapshot.read();
        assert!((snap.qpos[0] as f32 - q0).abs() < 1e-9);
    }

    #[test]
    fn steps_register_controls_advance() {
        let ctx = test_ctx();
        let mut a = arm(&ctx);
        a.write_32(REG_STEPS, 64).unwrap();
        a.write_32(REG_COMMIT, 1).unwrap();
        let t = f32::from_bits(a.read_32(REG_TIME).unwrap());
        assert!((t - 0.064).abs() < 1e-6, "time = {t}");
        assert_eq!(a.read_32(REG_STEPS).unwrap(), 64);
    }

    #[test]
    fn status_reports_no_monitor_without_window() {
        // World-only context (as in unit tests): the monitor bit reads 0.
        let ctx = test_ctx();
        let mut a = arm(&ctx);
        assert_eq!(a.read_32(REG_STATUS).unwrap() & 1, 0);
    }

    #[test]
    fn counts_and_timestep_report_the_model() {
        let ctx = test_ctx();
        let mut a = arm(&ctx);
        assert_eq!(a.read_32(REG_NU).unwrap(), 3);
        assert_eq!(a.read_32(REG_NQ).unwrap(), 3);
        let ts = f32::from_bits(a.read_32(REG_TIMESTEP).unwrap());
        assert!((ts - 0.001).abs() < 1e-9, "timestep = {ts}");
    }
}
