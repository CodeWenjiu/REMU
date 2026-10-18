//! Serial-chain geometry: [`Joint`] frames and forward kinematics
//! (quaternion propagation), monomorphized on the joint count.

use crate::vec::{Quat, Vec3};

/// One revolute joint of a serial chain.
///
/// The chain convention (matching `mjcf`'s generated frame data):
/// `frame_i = frame_(i-1) * translate(offset) * rotate(quat) * rotate(axis, q_i)`.
#[derive(Clone, Copy, Debug)]
pub struct Joint {
    /// Joint frame origin in the previous joint frame.
    pub offset: Vec3,
    /// Fixed rotation from the previous joint frame to this joint's frame.
    pub quat: Quat,
    /// Rotation axis in this joint's frame.
    pub axis: Vec3,
    /// Joint limits, when the model declares a range.
    pub range: Option<[f32; 2]>,
}

/// A serial chain of revolute joints ending in a tool point. `NJ` is a `const`
/// parameter so the loops specialize against `const` data.
pub struct Chain<const NJ: usize> {
    pub joints: &'static [Joint; NJ],
    /// Grasp point offset from the last joint frame.
    pub tip_offset: Vec3,
    /// Tool rotation from the last joint frame.
    pub tip_quat: Quat,
}

impl<const NJ: usize> Chain<NJ> {
    /// World position and tool orientation of the grasp point at `q`.
    pub fn tip_frame(&self, q: &[f32; NJ]) -> (Vec3, Quat) {
        let mut pos = Vec3::ZERO;
        let mut ori = Quat::IDENTITY;
        for (i, j) in self.joints.iter().enumerate() {
            pos = pos + ori.rotate(j.offset);
            ori = ori * j.quat;
            ori = ori * Quat::from_axis_angle(j.axis, q[i]);
        }
        (pos + ori.rotate(self.tip_offset), ori * self.tip_quat)
    }

    /// World position of the grasp point at joint configuration `q`.
    pub fn tip_pos(&self, q: &[f32; NJ]) -> Vec3 {
        self.tip_frame(q).0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Backend-agnostic position tolerance: micromath trig caps accuracy at
    /// ~0.002 per call.
    const TOL: f32 = 5.0e-3;

    const TOY2: [Joint; 2] = [
        Joint {
            offset: Vec3([0.0, 0.0, 0.1]),
            quat: Quat::IDENTITY,
            axis: Vec3([0.0, 0.0, 1.0]),
            range: None,
        },
        Joint {
            offset: Vec3([0.5, 0.0, 0.0]),
            quat: Quat::IDENTITY,
            axis: Vec3([0.0, 0.0, 1.0]),
            range: None,
        },
    ];
    const TOY: Chain<2> = Chain {
        joints: &TOY2,
        tip_offset: Vec3([0.3, 0.0, 0.0]),
        tip_quat: Quat::IDENTITY,
    };

    #[test]
    fn zero_pose_extends_along_x() {
        let p = TOY.tip_pos(&[0.0, 0.0]);
        assert!(
            (p[0] - 0.8).abs() < TOL && p[1].abs() < TOL && (p[2] - 0.1).abs() < TOL,
            "{p:?}"
        );
    }

    #[test]
    fn rotations_compose_about_z() {
        // Expected values are the f64 closed form rounded to f32:
        // p = Rz(0.3)·(0.5, 0, 0) + Rz(0.3 - 0.4)·(0.3, 0, 0) + (0, 0, 0.1).
        let p = TOY.tip_pos(&[0.3, -0.4]);
        assert!((p[0] - 0.776_169_5).abs() < TOL, "{p:?}");
        assert!((p[1] - 0.117_810_1).abs() < TOL, "{p:?}");
        assert!((p[2] - 0.1).abs() < TOL, "{p:?}");
    }

    #[test]
    fn tip_frame_carries_tool_rotation() {
        let (_, tip_ori) = TOY.tip_frame(&[0.3, -0.4]);
        let rv = tip_ori.rotation_vector();
        assert!(
            rv[0].abs() < TOL && rv[1].abs() < TOL && (rv[2] + 0.1).abs() < TOL,
            "{rv:?}"
        );
    }
}
