//! One model + one data: RAII-managed, driven through the state API.
//!
//! The engine pointers are owned by the session and only used while both model
//! and data are alive; every state buffer is sized via `mj_stateSize` before
//! the transfer. `Session` is deliberately `!Send`/`!Sync`: MuJoCo data is
//! single-threaded, so any cross-thread integration must copy state in/out
//! (e.g. through shared snapshots).

use std::ffi::{CStr, CString};
use std::marker::PhantomData;
use std::os::raw::{c_char, c_int};
use std::path::Path;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

use dlopen2::wrapper::Container;

use super::error::MujocoError;
use super::ffi::{self, MjData, MjModel, MujocoApi};
use super::runtime;

/// Temp-file counter for [`Session::from_xml_str`].
static TEMP_SEQ: AtomicU64 = AtomicU64::new(0);

pub struct Session {
    api: &'static Container<MujocoApi>,
    model: *mut MjModel,
    data: *mut MjData,
    timestep: f64,
    nq: usize,
    nv: usize,
    nu: usize,
    _single_threaded: PhantomData<Rc<()>>,
}

impl Session {
    /// Load a model from an MJCF/XML file. The engine is dlopen'd on first use.
    pub(crate) fn from_xml_file(path: &Path) -> Result<Self, MujocoError> {
        let api = runtime::api()?;
        let file = CString::new(path.to_string_lossy().as_ref())
            .map_err(|_| MujocoError::Xml("model path contains NUL".to_string()))?;

        let mut err_buf = [0 as c_char; 1024];
        let model = unsafe {
            api.load_xml(
                file.as_ptr(),
                std::ptr::null(),
                err_buf.as_mut_ptr(),
                err_buf.len() as c_int,
            )
        };
        if model.is_null() {
            let msg = unsafe { CStr::from_ptr(err_buf.as_ptr()) }
                .to_string_lossy()
                .into_owned();
            return Err(MujocoError::Xml(msg));
        }
        let data = unsafe { api.make_data(model) };
        if data.is_null() {
            unsafe { api.delete_model(model) };
            return Err(MujocoError::Engine("mj_makeData returned null".to_string()));
        }

        let mut session = Self {
            api,
            model,
            data,
            timestep: 0.0,
            nq: 0,
            nv: 0,
            nu: 0,
            _single_threaded: PhantomData,
        };
        session.nq = session.sig_size(ffi::STATE_QPOS);
        session.nv = session.sig_size(ffi::STATE_QVEL);
        session.nu = session.sig_size(ffi::STATE_CTRL);
        session.timestep = session.probe_timestep();
        Ok(session)
    }

    /// Convenience for tests/examples: write `xml` to a temp file and load it.
    pub(crate) fn from_xml_str(xml: &str) -> Result<Self, MujocoError> {
        let seq = TEMP_SEQ.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("remu_mujoco_{}_{seq}.xml", std::process::id()));
        std::fs::write(&path, xml)
            .map_err(|e| MujocoError::Xml(format!("write temp model: {e}")))?;
        let session = Self::from_xml_file(&path);
        let _ = std::fs::remove_file(&path);
        session
    }

    /// Number of positions in `qpos`.
    pub(crate) fn nq(&self) -> usize {
        self.nq
    }

    /// Number of velocities in `qvel`.
    pub(crate) fn nv(&self) -> usize {
        self.nv
    }

    /// Number of controls in `ctrl`.
    pub(crate) fn nu(&self) -> usize {
        self.nu
    }

    /// Model timestep in seconds, probed from the engine (see `probe_timestep`).
    pub(crate) fn timestep(&self) -> f64 {
        self.timestep
    }

    /// Simulation time in seconds.
    pub(crate) fn time(&self) -> f64 {
        let mut t = [0.0];
        self.get_state(ffi::STATE_TIME, &mut t);
        t[0]
    }

    pub(crate) fn read_qpos(&self, out: &mut [f64]) -> Result<(), MujocoError> {
        self.check_len(ffi::STATE_QPOS, out.len())?;
        self.get_state(ffi::STATE_QPOS, out);
        Ok(())
    }

    pub(crate) fn set_qpos(&mut self, qpos: &[f64]) -> Result<(), MujocoError> {
        self.check_len(ffi::STATE_QPOS, qpos.len())?;
        self.set_state(ffi::STATE_QPOS, qpos);
        Ok(())
    }

    pub(crate) fn read_qvel(&self, out: &mut [f64]) -> Result<(), MujocoError> {
        self.check_len(ffi::STATE_QVEL, out.len())?;
        self.get_state(ffi::STATE_QVEL, out);
        Ok(())
    }

    pub(crate) fn set_qvel(&mut self, qvel: &[f64]) -> Result<(), MujocoError> {
        self.check_len(ffi::STATE_QVEL, qvel.len())?;
        self.set_state(ffi::STATE_QVEL, qvel);
        Ok(())
    }

    /// Raw engine handles for the renderer (`mjv_*`/`mjr_*` take them directly).
    /// Same thread as this session by construction — the pointers must not
    /// escape it.
    pub(crate) fn model_ptr(&self) -> *const MjModel {
        self.model
    }

    pub(crate) fn data_ptr(&self) -> *mut MjData {
        self.data
    }

    /// Write actuator targets. A pure input: no effect until the next [`step`](Self::step).
    pub(crate) fn set_ctrl(&mut self, ctrl: &[f64]) -> Result<(), MujocoError> {
        self.check_len(ffi::STATE_CTRL, ctrl.len())?;
        self.set_state(ffi::STATE_CTRL, ctrl);
        Ok(())
    }

    /// Advance the simulation by one timestep.
    pub(crate) fn step(&mut self) {
        unsafe { self.api.step(self.model, self.data) };
    }

    /// Recompute derived quantities from the current state without advancing time.
    pub(crate) fn forward(&mut self) {
        unsafe { self.api.forward(self.model, self.data) };
    }

    /// Reset data to the model defaults (time 0, `qpos0`).
    pub(crate) fn reset(&mut self) {
        unsafe { self.api.reset_data(self.model, self.data) };
    }

    /// Index of a joint by name.
    pub(crate) fn joint_id(&self, name: &str) -> Result<usize, MujocoError> {
        let name_c = CString::new(name)
            .map_err(|_| MujocoError::State("joint name contains NUL".to_string()))?;
        let id = unsafe {
            self.api
                .name2id(self.model, ffi::OBJ_JOINT, name_c.as_ptr())
        };
        if id < 0 {
            Err(MujocoError::State(format!("no joint named {name:?}")))
        } else {
            Ok(id as usize)
        }
    }

    /// Element count of a state signature (`mj_stateSize`).
    fn sig_size(&self, sig: c_int) -> usize {
        (unsafe { self.api.state_size(self.model, sig) }) as usize
    }

    fn get_state(&self, sig: c_int, out: &mut [f64]) {
        debug_assert_eq!(out.len(), self.sig_size(sig));
        unsafe {
            self.api
                .get_state(self.model, self.data, out.as_mut_ptr(), sig)
        };
    }

    fn set_state(&mut self, sig: c_int, values: &[f64]) {
        debug_assert_eq!(values.len(), self.sig_size(sig));
        unsafe {
            self.api
                .set_state(self.model, self.data, values.as_ptr(), sig)
        };
    }

    fn check_len(&self, sig: c_int, len: usize) -> Result<(), MujocoError> {
        let want = self.sig_size(sig);
        if want == len {
            Ok(())
        } else {
            Err(MujocoError::State(format!(
                "state signature {sig:#x}: expected {want} values, got {len}"
            )))
        }
    }

    /// Probe the model timestep: reset, step once, read `time`, reset again.
    fn probe_timestep(&mut self) -> f64 {
        self.reset();
        self.step();
        let t = self.time();
        self.reset();
        t
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        unsafe { self.api.delete_data(self.data) };
        unsafe { self.api.delete_model(self.model) };
    }
}

#[cfg(test)]
mod tests {
    use crate::ARM_XML;

    use super::*;

    fn arm() -> Session {
        Session::from_xml_str(ARM_XML).expect("load test arm")
    }

    #[test]
    fn engine_loads_and_reports_3x() {
        assert!(super::runtime::engine_version().unwrap() >= 3_000_000);
    }

    /// Loads the Menagerie Panda when the dev shell exported
    /// `MUJOCO_MENAGERIE_DIR`; verifies include + meshdir-relative resolution.
    #[test]
    #[ignore = "needs the menagerie checkout (MUJOCO_MENAGERIE_DIR)"]
    fn panda_scene_loads_from_file() {
        let dir = std::env::var("MUJOCO_MENAGERIE_DIR").expect("MUJOCO_MENAGERIE_DIR");
        let path = std::path::Path::new(&dir).join("franka_emika_panda/scene.xml");
        let s = Session::from_xml_file(&path).expect("load panda scene");
        assert_eq!((s.nu(), s.nq()), (8, 9));
    }

    /// Composed model (the obstacle-injection seam): includes the Menagerie
    /// scene by absolute path, re-declares the compiler *after* the include
    /// (the included scene's own compiler, meshdir included, is not honored),
    /// injects an obstacle geom, and must stay behaviorally identical to the
    /// direct scene load.
    #[test]
    #[ignore = "needs the menagerie checkout (MUJOCO_MENAGERIE_DIR)"]
    fn composed_model_include_merges_worldbody_and_resolves_assets() {
        const OBJ_GEOM: c_int = 5;
        let dir = std::env::var("MUJOCO_MENAGERIE_DIR").expect("MUJOCO_MENAGERIE_DIR");
        let franka = std::path::Path::new(&dir).join("franka_emika_panda");
        let xml = format!(
            "<mujoco model=\"remu probe\">\
             <include file=\"{scene}\"/>\
             <compiler angle=\"radian\" autolimits=\"true\" meshdir=\"{assets}\"/>\
             <worldbody>\
             <geom name=\"obstacle-probe\" type=\"capsule\" fromto=\"0.3 -0.5 0.4 0.3 -0.5 0.9\" size=\"0.04\" contype=\"0\" conaffinity=\"0\"/>\
             </worldbody></mujoco>",
            scene = franka.join("scene.xml").display(),
            assets = franka.join("assets").display(),
        );
        let path = std::env::temp_dir().join("remu_composed_probe.xml");
        std::fs::write(&path, xml).unwrap();
        let mut composed = Session::from_xml_file(&path).expect("load composed model");
        let _ = std::fs::remove_file(&path);
        assert_eq!((composed.nu(), composed.nq()), (8, 9));
        let name = CString::new("obstacle-probe").unwrap();
        let id = unsafe {
            composed
                .api
                .name2id(composed.model, OBJ_GEOM, name.as_ptr())
        };
        assert!(id >= 0, "injected geom missing (id {id})");

        // The same servo run on the direct scene load: any lost compiler
        // semantics (angle radian, autolimits, …) would diverge visibly.
        let mut direct = Session::from_xml_file(&franka.join("scene.xml")).expect("load scene");
        let run = |s: &mut Session| {
            s.set_ctrl(&[0.0, 0.0, 0.0, -1.5, 0.0, 1.5, -0.5, 0.0])
                .unwrap();
            for _ in 0..400 {
                s.step();
            }
            let mut q = vec![0.0; s.nq()];
            s.read_qpos(&mut q).unwrap();
            q
        };
        let a = run(&mut composed);
        let b = run(&mut direct);
        for (x, y) in a.iter().zip(&b) {
            assert!((x - y).abs() < 1.0e-6, "composed {x} vs direct {y}");
        }
    }

    #[test]
    fn state_bits_compose_and_time_is_scalar() {
        let s = arm();
        assert_eq!(s.sig_size(ffi::STATE_TIME), 1);
        assert_eq!(
            s.sig_size(ffi::STATE_QPOS | ffi::STATE_QVEL),
            s.sig_size(ffi::STATE_QPOS) + s.sig_size(ffi::STATE_QVEL)
        );
        assert_eq!(
            s.sig_size(ffi::STATE_QPOS | ffi::STATE_QVEL | ffi::STATE_CTRL),
            s.sig_size(ffi::STATE_QPOS) + s.sig_size(ffi::STATE_QVEL) + s.sig_size(ffi::STATE_CTRL)
        );
    }

    #[test]
    fn dims_and_joint_names_match_model() {
        let s = arm();
        assert_eq!((s.nq(), s.nv(), s.nu()), (3, 3, 3));
        assert_eq!(s.joint_id("j0").unwrap(), 0);
        assert_eq!(s.joint_id("j2").unwrap(), 2);
        assert!(s.joint_id("nope").is_err());
        assert!((s.timestep() - 0.001).abs() < 1e-12);
    }

    #[test]
    fn servos_reach_targets_and_time_accumulates() {
        let mut s = arm();
        let targets = [0.5, -0.4, 0.3];
        s.set_ctrl(&targets).unwrap();
        for _ in 0..500 {
            s.step();
        }
        assert!((s.time() - 0.5).abs() < 1e-9, "time = {}", s.time());
        let mut q = vec![0.0; s.nq()];
        s.read_qpos(&mut q).unwrap();
        for (got, want) in q.iter().zip(targets) {
            assert!((got - want).abs() < 0.02, "qpos {got} vs target {want}");
        }
    }

    #[test]
    fn identical_command_stream_is_deterministic() {
        let run = || {
            let mut s = arm();
            for k in 0..300 {
                let t = k as f64 * 0.001;
                s.set_ctrl(&[t.sin(), (2.0 * t).cos(), (0.5 * t).sin()])
                    .unwrap();
                s.step();
            }
            let mut q = vec![0.0; s.nq()];
            s.read_qpos(&mut q).unwrap();
            q
        };
        let a = run();
        let b = run();
        for (x, y) in a.iter().zip(&b) {
            assert_eq!(x.to_bits(), y.to_bits());
        }
    }

    #[test]
    fn qpos_roundtrip_is_exact() {
        let mut s = arm();
        let values = [0.4, -0.2, 0.1];
        s.set_qpos(&values).unwrap();
        let mut q = [0.0; 3];
        s.read_qpos(&mut q).unwrap();
        assert_eq!(q, values);
    }

    #[test]
    fn wrong_buffer_sizes_are_rejected() {
        let mut s = arm();
        assert!(s.set_ctrl(&[1.0, 2.0]).is_err());
        assert!(s.read_qpos(&mut [0.0; 2]).is_err());
    }

    /// Planar forward kinematics + ASCII render (XZ projection). The link
    /// lengths and base height mirror `TEST_ARM_XML`.
    fn render_arm(q: &[f64], t: f64, target: &[f64]) -> String {
        const W: usize = 64;
        const H: usize = 20;
        const X_MIN: f64 = -0.15;
        const X_MAX: f64 = 0.90;
        const Z_MIN: f64 = -0.60;
        const Z_MAX: f64 = 0.90;

        let to_cell = |x: f64, z: f64| {
            let c = ((x - X_MIN) / (X_MAX - X_MIN) * (W - 1) as f64).round() as isize;
            let r = ((Z_MAX - z) / (Z_MAX - Z_MIN) * (H - 1) as f64).round() as isize;
            (
                c.clamp(0, W as isize - 1) as usize,
                r.clamp(0, H as isize - 1) as usize,
            )
        };

        let mut grid = vec![vec![' '; W]; H];
        let (_, ground) = to_cell(0.0, 0.0);
        grid[ground].fill('-');

        // Chain FK: cumulative joint angles along the links.
        const LEN: [f64; 3] = [0.3, 0.25, 0.2];
        let mut points = vec![(0.0, 0.1)];
        let mut angle = 0.0;
        for (i, len) in LEN.iter().enumerate() {
            angle += q[i];
            let (px, pz) = *points.last().unwrap();
            points.push((px + len * angle.cos(), pz + len * angle.sin()));
        }

        for seg in points.windows(2) {
            for s in 0..=200 {
                let f = s as f64 / 200.0;
                let (c, r) = to_cell(
                    seg[0].0 + f * (seg[1].0 - seg[0].0),
                    seg[0].1 + f * (seg[1].1 - seg[0].1),
                );
                grid[r][c] = '#';
            }
        }
        for (x, z) in &points {
            let (c, r) = to_cell(*x, *z);
            grid[r][c] = 'O';
        }

        let mut out = format!(
            "t={t:4.2}s  target=[{:.2} {:.2} {:.2}]  qpos=[{:.2} {:.2} {:.2}]\n",
            target[0], target[1], target[2], q[0], q[1], q[2]
        );
        for row in grid {
            out.extend(row);
            out.push('\n');
        }
        out
    }

    /// Terminal demo: `cargo test -p remu_state mujoco_arm_demo -- --ignored --nocapture`.
    /// Animates in place when stdout is a TTY; otherwise prints a few frames.
    #[test]
    #[ignore = "prints a terminal animation; not part of the normal suite"]
    fn mujoco_arm_demo() {
        use std::io::{IsTerminal, Write};

        let mut s = arm();
        let tty = std::io::stdout().is_terminal();
        let mut q = vec![0.0; s.nq()];
        for k in 0..=3000 {
            let t = k as f64 * s.timestep();
            let ctrl = [0.7 * t.sin(), 0.5 * (1.3 * t).sin(), 0.4 * (0.7 * t).cos()];
            s.set_ctrl(&ctrl).unwrap();
            s.step();
            if k % 40 != 0 {
                continue;
            }
            s.read_qpos(&mut q).unwrap();
            let frame = render_arm(&q, t, &ctrl);
            if tty {
                print!("\x1b[2J\x1b[H{frame}");
                let _ = std::io::stdout().flush();
                std::thread::sleep(std::time::Duration::from_millis(33));
            } else if k % 300 == 0 {
                println!("{frame}");
            }
        }
    }
}
