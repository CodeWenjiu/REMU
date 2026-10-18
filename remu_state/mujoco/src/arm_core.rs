//! Shared simulation core: one model + one data, driven by actuator targets
//! with a published snapshot for the monitor. Embedded by the bus arm device
//! (register surface) and the native host arm backend (direct calls), so both
//! platforms run the same behavior.

use std::sync::Arc;

use crate::triple::TripleBuffer;
use crate::{ModelSource, MujocoError, Session};

/// Default microsteps per commit: 16 × 1 ms ≈ one 60 Hz frame of simulated
/// time at the built-in toy arm's timestep.
const DEFAULT_STEPS: u32 = 16;

/// Latest arm state, published by the core and consumed by the armview render
/// thread (values cross threads through a lock-free triple buffer; the mirror
/// session re-derives everything else).
#[derive(Default)]
pub struct ArmSnapshot {
    pub qpos: Vec<f64>,
    pub qvel: Vec<f64>,
}

pub struct ArmCore {
    session: Session,
    /// Actuator targets (ctrl values), one per `session.nu()`.
    targets: Vec<f32>,
    steps: u32,
    commits: u32,
    /// Shared with the monitor window (read by the render thread).
    snapshot: Arc<TripleBuffer<ArmSnapshot>>,
    /// Scratch buffers for the engine state transfers; `qpos`/`qvel` keep the
    /// last committed state, serving the register readback.
    ctrl: Vec<f64>,
    qpos: Vec<f64>,
    qvel: Vec<f64>,
}

impl ArmCore {
    /// Load the model of `source` and publish its initial state.
    pub fn load(source: &ModelSource) -> Result<Self, MujocoError> {
        let session = source.load()?;
        let mut core = Self {
            targets: vec![0.0; session.nu()],
            ctrl: vec![0.0; session.nu()],
            qpos: vec![0.0; session.nq()],
            qvel: vec![0.0; session.nv()],
            session,
            steps: DEFAULT_STEPS,
            commits: 0,
            snapshot: Arc::new(TripleBuffer::new()),
        };
        core.publish();
        Ok(core)
    }

    /// Actuator target count (ctrl channels).
    pub fn nu(&self) -> usize {
        self.targets.len()
    }

    /// Joint position count.
    pub fn nq(&self) -> usize {
        self.session.nq()
    }

    pub fn time(&self) -> f64 {
        self.session.time()
    }

    pub fn timestep(&self) -> f64 {
        self.session.timestep()
    }

    pub fn steps(&self) -> u32 {
        self.steps
    }

    pub fn commits(&self) -> u32 {
        self.commits
    }

    pub fn target(&self, channel: usize) -> f32 {
        self.targets[channel]
    }

    pub fn set_target(&mut self, channel: usize, value: f32) {
        self.targets[channel] = value;
    }

    pub fn set_steps(&mut self, steps: u32) {
        self.steps = steps.clamp(1, 100_000);
    }

    /// Apply the targets, advance the simulation, publish a snapshot.
    pub fn commit(&mut self) {
        for (c, t) in self.ctrl.iter_mut().zip(&self.targets) {
            *c = f64::from(*t);
        }
        self.session.set_ctrl(&self.ctrl).expect("arm: ctrl size");
        for _ in 0..self.steps {
            self.session.step();
        }
        self.commits = self.commits.wrapping_add(1);
        self.publish();
    }

    /// Copy the engine state into the shared snapshot and the readback cache.
    pub fn publish(&mut self) {
        self.session
            .read_qpos(&mut self.qpos)
            .expect("arm: qpos size");
        self.session
            .read_qvel(&mut self.qvel)
            .expect("arm: qvel size");
        self.snapshot.publish(|snap| {
            snap.qpos.clone_from(&self.qpos);
            snap.qvel.clone_from(&self.qvel);
        });
    }

    /// Read one joint position as of the last commit (from the cached state).
    pub fn read_qpos_f32(&self, joint: usize) -> f32 {
        self.qpos[joint] as f32
    }

    /// The snapshot channel shared with the monitor window's render thread.
    pub fn snapshot(&self) -> Arc<TripleBuffer<ArmSnapshot>> {
        Arc::clone(&self.snapshot)
    }
}
