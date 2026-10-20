//! Shared simulation core: one model + one data, driven by actuator targets
//! with a published snapshot for the monitor. Embedded by the bus arm device
//! (register surface) and the native host arm backend (direct calls), so both
//! platforms run the same behavior.

use std::sync::Arc;

use super::flow::MocapCommand;
use super::snapshot::ArmSnapshot;
use crate::{ModelSource, MujocoError, Session, TripleBuffer};

/// Default microsteps per commit: 16 × 1 ms ≈ one 60 Hz frame of simulated
/// time at the built-in toy arm's timestep.
const DEFAULT_STEPS: u32 = 16;

/// Where parked obstacles sit: outside the scanner's 5 m range and the view.
pub(crate) const PARK_AT: [f64; 2] = [10.0, 10.0];

pub struct ArmCore {
    session: Session,
    /// Actuator targets (ctrl values), one per `session.nu()`.
    targets: Vec<f32>,
    steps: u32,
    commits: u32,
    /// Shared with the monitor window (read by the render thread).
    snapshot: Arc<TripleBuffer<ArmSnapshot>>,
    /// Obstacle positions the UI asked for (read before each commit).
    mocap_cmd: Arc<TripleBuffer<MocapCommand>>,
    /// Scratch buffers for the engine state transfers; `qpos`/`qvel`/`mocap`
    /// keep the last committed state, serving the register readback and the
    /// snapshot.
    ctrl: Vec<f64>,
    qpos: Vec<f64>,
    qvel: Vec<f64>,
    mocap: Vec<f64>,
    /// The model's initial mocap pose, kept for the arm view's "place".
    mocap_home: Vec<f64>,
}

impl ArmCore {
    /// Load the model of `source` and publish its initial state.
    pub fn load(source: &ModelSource) -> Result<Self, MujocoError> {
        let session = source.load()?;
        let mut mocap = vec![0.0; 3 * session.mocap_count()];
        session.read_mocap(&mut mocap)?;
        let mocap_home = mocap.clone();
        let mut core = Self {
            targets: vec![0.0; session.nu()],
            ctrl: vec![0.0; session.nu()],
            qpos: vec![0.0; session.nq()],
            qvel: vec![0.0; session.nv()],
            session,
            steps: DEFAULT_STEPS,
            commits: 0,
            snapshot: Arc::new(TripleBuffer::new()),
            mocap_cmd: Arc::new(TripleBuffer::new()),
            mocap,
            mocap_home,
        };
        core.publish();
        Ok(core)
    }

    /// Park every mocap obstacle at startup: the run model defines their home
    /// poses, but the demo starts with a clean scene. The arm view's "place"
    /// restores the selected one from `mocap_home`.
    pub fn park_mocap(&mut self) {
        if self.mocap.is_empty() {
            return;
        }
        for chunk in self.mocap.chunks_mut(3) {
            chunk[0] = PARK_AT[0];
            chunk[1] = PARK_AT[1];
        }
        self.session
            .set_mocap(&self.mocap)
            .expect("arm: mocap size");
        self.session.forward();
        self.publish();
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
        self.apply_mocap_command();
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
        self.session
            .read_mocap(&mut self.mocap)
            .expect("arm: mocap size");
        self.snapshot.publish(|snap| {
            snap.qpos.clone_from(&self.qpos);
            snap.qvel.clone_from(&self.qvel);
            snap.mocap.clone_from(&self.mocap);
            snap.mocap_home.clone_from(&self.mocap_home);
        });
    }

    /// Apply the newest UI obstacle positions before the next physics step.
    fn apply_mocap_command(&mut self) {
        if self.mocap.is_empty() {
            return;
        }
        let desired = self.mocap_cmd.read().pos.clone();
        if desired.len() == self.mocap.len() && desired != self.mocap {
            self.session
                .set_mocap(&desired)
                .expect("arm: mocap size");
            self.mocap = desired;
        }
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

    /// The obstacle command channel shared with the monitor window's UI.
    pub fn mocap_command(&self) -> Arc<TripleBuffer<MocapCommand>> {
        Arc::clone(&self.mocap_cmd)
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

    /// A world with one mocap capsule at (1, 0, 0..0.5).
    const MOCAP_XML: &str = "<mujoco model=\"mocap\">\
        <worldbody>\
        <body name=\"obstacle:pole\" mocap=\"true\" pos=\"1 0 0\">\
        <geom type=\"capsule\" fromto=\"0 0 0 0 0 0.5\" size=\"0.05\"/>\
        </body>\
        </worldbody>\
        </mujoco>";

    #[test]
    fn mocap_command_moves_the_obstacle_for_rays() {
        let mut core =
            ArmCore::load(&ModelSource::Xml(MOCAP_XML.to_string())).expect("load mocap model");
        let mut out = [0.0f32; 2];
        core.cast_fan(
            [0.0, 0.0, 0.25],
            0.0,
            std::f64::consts::FRAC_PI_2,
            5.0,
            &mut out,
        );
        assert!((out[0] - 0.95).abs() < 1.0e-3, "{out:?}");
        assert!(out[1].is_infinite(), "{out:?}");

        // Move the obstacle onto the +y ray; the next commit applies it.
        core.mocap_command()
            .publish(|c| c.pos = vec![0.0, 1.0, 0.0]);
        core.commit();
        core.cast_fan(
            [0.0, 0.0, 0.25],
            0.0,
            std::f64::consts::FRAC_PI_2,
            5.0,
            &mut out,
        );
        assert!(out[0].is_infinite(), "{out:?}");
        assert!((out[1] - 0.95).abs() < 1.0e-3, "{out:?}");
    }

    #[test]
    fn park_moves_obstacles_out_of_the_scan_and_keeps_home() {
        let mut core =
            ArmCore::load(&ModelSource::Xml(MOCAP_XML.to_string())).expect("load mocap model");
        core.park_mocap();
        {
            let snapshot = core.snapshot();
            let snap = snapshot.read();
            assert!((snap.mocap_home[0] - 1.0).abs() < 1.0e-9, "{:?}", snap.mocap_home);
            assert!(
                snap.mocap[0] >= PARK_AT[0] && snap.mocap[1] >= PARK_AT[1],
                "{:?}",
                snap.mocap
            );
        }
        let mut out = [0.0f32; 1];
        core.cast_fan([0.0, 0.0, 0.25], 0.0, 0.0, 5.0, &mut out);
        assert!(out[0].is_infinite(), "{out:?}");
    }
}
