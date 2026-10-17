//! Arm device: MMIO joint-target registers driving a MuJoCo model, with each
//! committed state published to the monitor window (`armview`).
//!
//! A bus master writes three target angles and a commit register; a commit
//! applies the targets, advances the simulation by `steps` microsteps, and
//! publishes a snapshot for the render thread. Joint positions and simulated
//! time are readable back through the same register window.
//!
//! Bring-up shape: the register file is reachable directly for now; the
//! external-bus layer (i2c controller + target protocol) re-homes it behind a
//! bus later, without changing the model or the monitor path.

use std::sync::{Arc, Mutex};

use crate::bus::{BusError, device::DeviceContext};

use super::mujoco::Session;
use super::window::WindowHost;

/// MJCF of the 3-DOF planar arm, no gravity, position servos (steady state ==
/// target). `armature` (motor reflected inertia) keeps the light links stable
/// under the explicit-Euler integrator at kp=100. Shared by the device, the
/// session tests, and the armview demo.
pub(crate) const ARM_XML: &str = r#"
<mujoco model="arm3dof">
  <option timestep="0.001" gravity="0 0 0"/>
  <visual>
    <headlight ambient="0.4 0.4 0.4" diffuse="0.7 0.7 0.7"/>
  </visual>
  <default>
    <joint type="hinge" axis="0 1 0" damping="0.1" armature="0.01"/>
    <geom type="capsule" size="0.02" density="700"/>
    <position kp="100" kv="3"/>
  </default>
  <worldbody>
    <body name="link0" pos="0 0 0.1">
      <joint name="j0"/>
      <geom fromto="0 0 0 0.3 0 0"/>
      <body name="link1" pos="0.3 0 0">
        <joint name="j1"/>
        <geom fromto="0 0 0 0.25 0 0"/>
        <body name="link2" pos="0.25 0 0">
          <joint name="j2"/>
          <geom fromto="0 0 0 0.2 0 0"/>
        </body>
      </body>
    </body>
  </worldbody>
  <actuator>
    <position name="a0" joint="j0" ctrlrange="-1.5 1.5"/>
    <position name="a1" joint="j1" ctrlrange="-1.5 1.5"/>
    <position name="a2" joint="j2" ctrlrange="-1.5 1.5"/>
  </actuator>
</mujoco>
"#;

/// Joint count of [`ARM_XML`].
const ARM_JOINTS: usize = 3;

// Register offsets (32-bit registers; f32 values travel as their bit pattern).
const REG_TARGET: usize = 0x00; // r/w: joint targets (radians), stride 4
const REG_COMMIT: usize = 0x0C; // w: apply targets + advance + publish
const REG_STEPS: usize = 0x10; // r/w: microsteps per commit (default 16)
const REG_QPOS: usize = 0x14; // r: joint positions (radians), stride 4
const REG_TIME: usize = 0x20; // r: simulated time (seconds)
const REG_COMMITS: usize = 0x24; // r: completed commits (wrapping u32)
const REG_STATUS: usize = 0x28; // r: bit 0 = armview monitor window alive
const ARM_REG_SIZE: usize = 0x2C;

/// Default microsteps per commit: 16 × 1 ms ≈ one 60 Hz frame of simulated
/// time.
const DEFAULT_STEPS: u32 = 16;

/// Latest arm state, published by the device and consumed by the armview
/// render thread (state crosses threads only as copied snapshots; the mirror
/// session re-derives everything else).
#[derive(Default)]
pub(crate) struct ArmSnapshot {
    pub(crate) qpos: Vec<f64>,
    pub(crate) qvel: Vec<f64>,
}

pub(super) struct Arm {
    session: Session,
    targets: [f32; ARM_JOINTS],
    steps: u32,
    commits: u32,
    /// The shared window host (holds the armview monitor window).
    window: Option<Arc<WindowHost>>,
    /// Shared with the monitor window (read by the render thread).
    snapshot: Arc<Mutex<ArmSnapshot>>,
    /// Scratch buffers for reading state out of the engine.
    qpos: Vec<f64>,
    qvel: Vec<f64>,
}

// The engine session is single-threaded by design, but a bus device must
// satisfy `Send + Sync`. Device access is serialized (every accessor takes
// `&mut self` and runs on the simulator thread that built the session), and
// state crosses to the render thread only as snapshots (see `session.rs`),
// so these impls just satisfy the boxed-device contract.
unsafe impl Send for Arm {}
unsafe impl Sync for Arm {}

impl Arm {
    pub(super) fn new() -> Self {
        let session = Session::from_xml_str(ARM_XML).expect("arm: load model via libmujoco");
        let qpos = vec![0.0; session.nq()];
        let qvel = vec![0.0; session.nv()];
        let mut arm = Self {
            session,
            targets: [0.0; ARM_JOINTS],
            steps: DEFAULT_STEPS,
            commits: 0,
            window: None,
            snapshot: Arc::new(Mutex::new(ArmSnapshot::default())),
            qpos,
            qvel,
        };
        arm.publish();
        arm
    }

    /// Apply the targets, advance the simulation, publish a snapshot.
    fn commit(&mut self) {
        let ctrl = self.targets.map(f64::from);
        self.session.set_ctrl(&ctrl).expect("arm: ctrl size");
        for _ in 0..self.steps {
            self.session.step();
        }
        self.commits = self.commits.wrapping_add(1);
        self.publish();
    }

    /// Copy the engine state into the shared snapshot.
    fn publish(&mut self) {
        self.session
            .read_qpos(&mut self.qpos)
            .expect("arm: qpos size");
        self.session
            .read_qvel(&mut self.qvel)
            .expect("arm: qvel size");
        let mut snap = self.snapshot.lock().unwrap();
        snap.qpos.clone_from(&self.qpos);
        snap.qvel.clone_from(&self.qvel);
    }

    /// Read one joint position back out of the engine.
    fn qpos_f32(&mut self, joint: usize) -> f32 {
        self.session
            .read_qpos(&mut self.qpos)
            .expect("arm: qpos size");
        self.qpos[joint] as f32
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
        window.open_arm_view(ARM_XML, Arc::clone(&self.snapshot));
        self.window = Some(Arc::clone(window));
    }

    fn read_32(&mut self, offset: usize) -> Result<u32, BusError> {
        Ok(match offset {
            o if (REG_TARGET..REG_TARGET + 4 * ARM_JOINTS).contains(&o) => {
                self.targets[(o - REG_TARGET) / 4].to_bits()
            }
            o if (REG_QPOS..REG_QPOS + 4 * ARM_JOINTS).contains(&o) => {
                self.qpos_f32((o - REG_QPOS) / 4).to_bits()
            }
            REG_STEPS => self.steps,
            REG_TIME => (self.session.time() as f32).to_bits(),
            REG_COMMITS => self.commits,
            REG_STATUS => self.window.as_ref().map_or(0, |w| w.armview_alive() as u32),
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
            o if (REG_TARGET..REG_TARGET + 4 * ARM_JOINTS).contains(&o) => {
                self.targets[(o - REG_TARGET) / 4] = f32::from_bits(value);
            }
            REG_STEPS => self.steps = value.clamp(1, 100_000),
            REG_COMMIT => self.commit(),
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
        let snap = a.snapshot.lock().unwrap();
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
}
