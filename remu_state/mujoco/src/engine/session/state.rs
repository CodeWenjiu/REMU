//! State-signature access: qpos/qvel/ctrl/mocap plus size checks.

use std::os::raw::c_int;

use super::Session;
use super::super::ffi;
use crate::MujocoError;

impl Session {
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
    /// Write actuator targets. A pure input: no effect until the next [`step`](Self::step).
    pub(crate) fn set_ctrl(&mut self, ctrl: &[f64]) -> Result<(), MujocoError> {
        self.check_len(ffi::STATE_CTRL, ctrl.len())?;
        self.set_state(ffi::STATE_CTRL, ctrl);
        Ok(())
    }

    /// Number of mocap bodies in the model.
    pub(crate) fn mocap_count(&self) -> usize {
        self.sig_size(ffi::STATE_MOCAP_POS) / 3
    }

    /// Read the mocap body positions (`3 * nmocap` values).
    pub(crate) fn read_mocap(&self, out: &mut [f64]) -> Result<(), MujocoError> {
        self.check_len(ffi::STATE_MOCAP_POS, out.len())?;
        self.get_state(ffi::STATE_MOCAP_POS, out);
        Ok(())
    }

    /// Write the mocap body positions; kinematics picks them up on the next
    /// `forward`/`step`.
    pub(crate) fn set_mocap(&mut self, pos: &[f64]) -> Result<(), MujocoError> {
        self.check_len(ffi::STATE_MOCAP_POS, pos.len())?;
        self.set_state(ffi::STATE_MOCAP_POS, pos);
        Ok(())
    }
    /// Element count of a state signature (`mj_stateSize`).
    pub(super) fn sig_size(&self, sig: c_int) -> usize {
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
    pub(super) fn probe_timestep(&mut self) -> f64 {
        self.reset();
        self.step();
        let t = self.time();
        self.reset();
        t
    }
}
