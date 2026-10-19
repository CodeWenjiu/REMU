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

    /// Cast one planar fan of rays from `origin` (world frame): ray `i` runs
    /// along yaw `angle_min + i * angle_incr` about world Z. `out` receives the
    /// nearest surface distance per ray, `f32::INFINITY` for no hit within
    /// `range_max`; rays are cast against the model as of its committed state.
    pub fn cast_fan(
        &mut self,
        origin: [f64; 3],
        angle_min: f64,
        angle_incr: f64,
        range_max: f64,
        out: &mut [f32],
    ) {
        let mut dirs = Vec::with_capacity(3 * out.len());
        for i in 0..out.len() {
            let a = angle_min + angle_incr * i as f64;
            dirs.extend_from_slice(&[a.cos(), a.sin(), 0.0]);
        }
        let mut dists = vec![0.0; out.len()];
        self.session
            .multi_ray(origin, &dirs, &mut dists, range_max)
            .expect("arm: multi_ray sizes");
        for (o, d) in out.iter_mut().zip(dists) {
            *o = if d >= 0.0 && d < range_max {
                d as f32
            } else {
                f32::INFINITY
            };
        }
    }

    /// The snapshot channel shared with the monitor window's render thread.
    pub fn snapshot(&self) -> Arc<TripleBuffer<ArmSnapshot>> {
        Arc::clone(&self.snapshot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A world with one vertical capsule obstacle at (1, 0, 0..0.5).
    const POLE_XML: &str = "<mujoco model=\"fan\">\
        <worldbody>\
        <geom name=\"obstacle:pole\" type=\"capsule\" fromto=\"1 0 0 1 0 0.5\" size=\"0.05\"/>\
        </worldbody>\
        </mujoco>";

    #[test]
    fn fan_hits_the_pole_and_misses_elsewhere() {
        let mut core =
            ArmCore::load(&ModelSource::Xml(POLE_XML.to_string())).expect("load mini model");
        let mut out = [0.0f32; 4];
        core.cast_fan(
            [0.0, 0.0, 0.25],
            0.0,
            std::f64::consts::FRAC_PI_2,
            5.0,
            &mut out,
        );
        // Ray 0 heads +x into the pole side (1 - radius); the rest see nothing.
        assert!((out[0] - 0.95).abs() < 1.0e-3, "{out:?}");
        assert!(out[1..].iter().all(|r| r.is_infinite()), "{out:?}");
    }
}
