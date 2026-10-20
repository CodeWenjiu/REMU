//! The arm model source: the built-in toy arm, or an MJCF file selected by
//! the `REMU_ARM_MODEL` environment variable.

use std::path::PathBuf;

use crate::MujocoError;
use super::session::Session;

/// MJCF of the 3-DOF planar arm on a floor stand: the default model when
/// `REMU_ARM_MODEL` is unset. No gravity, position servos (steady state ==
/// target); `armature` (motor reflected inertia) keeps the light links stable
/// under the explicit-Euler integrator at kp=100.
///
/// The scene is non-colliding visual dressing for the monitor. The arm sweeps
/// a full disk (radius 0.77 around its base in the y=0 plane, down to z ~
/// -0.67), so nothing may sit under the sweep: the floor lies below every
/// reachable pose and the mount column stands clear of the y=0 slab at y=-0.3,
/// with a bracket reaching to the base. The massless `tip` body at the end of
/// `link2` is the end-effector point the armview trail follows. The mocap
/// `obstacle:pole` is the interactive target the armview panel drags; it is
/// off the sweep plane and non-colliding, so it only affects ray casting.
pub const ARM_XML: &str = r#"
<mujoco model="arm3dof">
  <option timestep="0.001" gravity="0 0 0"/>
  <visual>
    <headlight ambient="0.4 0.4 0.4" diffuse="0.7 0.7 0.7"/>
  </visual>
  <asset>
    <texture name="tex-floor" type="2d" builtin="checker" rgb1="0.2 0.21 0.23" rgb2="0.27 0.28 0.31" width="512" height="512"/>
    <material name="mat-floor" texture="tex-floor" texrepeat="6 6" texuniform="true" reflectance="0.08"/>
    <material name="mat-stand" rgba="0.35 0.35 0.38 1"/>
  </asset>
  <default>
    <joint type="hinge" axis="0 1 0" damping="0.1" armature="0.01"/>
    <geom type="capsule" size="0.02" density="700"/>
    <position kp="100" kv="3"/>
  </default>
  <worldbody>
    <light directional="true" pos="0.6 1 2.5" diffuse="0.45 0.45 0.45" specular="0.5 0.5 0.5"/>
    <light directional="true" pos="-0.8 -1.5 1.5" diffuse="0.2 0.2 0.2"/>
    <geom name="floor" type="plane" pos="0 0 -1.0" size="0 0 0.25" material="mat-floor" contype="0" conaffinity="0"/>
    <geom name="stand" type="cylinder" pos="0 -0.3 -0.45" size="0.06 0.55" material="mat-stand" contype="0" conaffinity="0"/>
    <geom name="bracket" type="capsule" fromto="0 -0.3 0.1 0 -0.03 0.1" size="0.03" material="mat-stand" contype="0" conaffinity="0"/>
    <body name="obstacle:pole" mocap="true" pos="0.35 0.45 0">
      <geom name="obstacle:pole" type="capsule" fromto="0 0 0 0 0 0.9" size="0.035" rgba="0.75 0.25 0.2 1" contype="0" conaffinity="0"/>
    </body>
    <body name="link0" pos="0 0 0.1">
      <joint name="j0"/>
      <geom fromto="0 0 0 0.3 0 0"/>
      <body name="link1" pos="0.3 0 0">
        <joint name="j1"/>
        <geom fromto="0 0 0 0.25 0 0"/>
        <body name="link2" pos="0.25 0 0">
          <joint name="j2"/>
          <geom fromto="0 0 0 0.2 0 0"/>
          <body name="tip" pos="0.2 0 0"/>
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

/// Where the arm's MJCF model comes from.
#[derive(Clone)]
pub enum ModelSource {
    /// Inline MJCF (the built-in toy arm).
    Xml(String),
    /// An MJCF file on disk; relative asset paths resolve against its directory.
    File(PathBuf),
}

impl ModelSource {
    /// `REMU_ARM_MODEL` (an MJCF path) when set, else the built-in model.
    pub fn from_env(builtin: &str) -> Self {
        match std::env::var("REMU_ARM_MODEL") {
            Ok(path) if !path.is_empty() => Self::File(PathBuf::from(path)),
            _ => Self::Xml(builtin.to_string()),
        }
    }

    pub fn load(&self) -> Result<Session, MujocoError> {
        match self {
            Self::Xml(xml) => Session::from_xml_str(xml),
            Self::File(path) => Session::from_xml_file(path),
        }
    }

    /// The built-in toy arm lacks `<visual><global>` camera hints, so the
    /// viewer frames it manually; file models bring their own.
    pub fn is_builtin(&self) -> bool {
        matches!(self, Self::Xml(_))
    }
}
