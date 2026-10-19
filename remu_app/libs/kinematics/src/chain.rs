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
    /// Forward-kinematics spine at `q`: the grasp point, the tool orientation
    /// and the per-joint frame origins (anchors) and world rotation axes.
    pub(crate) fn spine(&self, q: &[f32; NJ]) -> Spine<NJ> {
        let mut pos = Vec3::default();
        let mut ori = Quat::IDENTITY;
        let mut anchor = [Vec3::default(); NJ];
        let mut axis = [Vec3::default(); NJ];
        for (i, j) in self.joints.iter().enumerate() {
            pos = pos + ori.rotate(j.offset);
            ori = ori * j.quat;
            anchor[i] = pos;
            axis[i] = ori.rotate(j.axis);
            ori = ori * Quat::from_axis_angle(j.axis, q[i]);
        }
        Spine {
            tip: pos + ori.rotate(self.tip_offset),
            tip_ori: ori * self.tip_quat,
            anchor,
            axis,
        }
    }

    /// World position and tool orientation of the grasp point at `q`.
    pub fn tip_frame(&self, q: &[f32; NJ]) -> (Vec3, Quat) {
        let spine = self.spine(q);
        (spine.tip, spine.tip_ori)
    }

    /// World position of the grasp point at joint configuration `q`.
    pub fn tip_pos(&self, q: &[f32; NJ]) -> Vec3 {
        self.spine(q).tip
    }

    /// The arm's link polyline at `q`: the chain root (the mounting origin),
    /// the joint anchors, and the grasp point. Used for self-filtering sensor
    /// returns against the robot's own geometry.
    pub fn links(&self, q: &[f32; NJ]) -> Links<NJ> {
        let spine = self.spine(q);
        Links {
            root: Vec3::default(),
            anchor: spine.anchor,
            tip: spine.tip,
        }
    }
}

/// The arm's link polyline at some configuration: `root -> anchor₀ -> … ->
/// anchorₙ₋₁ -> tip` (`root` is the chain mounting origin).
#[derive(Clone, Copy, Debug)]
pub struct Links<const NJ: usize> {
    root: Vec3,
    anchor: [Vec3; NJ],
    tip: Vec3,
}

impl<const NJ: usize> Links<NJ> {
    /// Distance from `p` to the closest point of the polyline. `NJ >= 1`.
    pub fn distance(&self, p: Vec3) -> f32 {
        let mut best = seg_distance(p, self.root, self.anchor[0]);
        for i in 0..NJ - 1 {
            best = best.min(seg_distance(p, self.anchor[i], self.anchor[i + 1]));
        }
        best.min(seg_distance(p, self.anchor[NJ - 1], self.tip))
    }
}

/// Distance from point `p` to the segment `[a, b]`.
fn seg_distance(p: Vec3, a: Vec3, b: Vec3) -> f32 {
    let ab = b - a;
    let aa = ab.dot(ab);
    let t = if aa > 1.0e-12 {
        ((p - a).dot(ab) / aa).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let d = p - (a + ab * t);
    crate::math::sqrt(d.dot(d))
}

pub(crate) struct Spine<const NJ: usize> {
    pub tip: Vec3,
    pub tip_ori: Quat,
    pub anchor: [Vec3; NJ],
    pub axis: [Vec3; NJ],
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
    fn links_measure_distance_to_the_polyline() {
        let links = TOY.links(&[0.0, 0.0]);
        // Straight above the middle (joint-to-joint) segment.
        let above = links.distance(Vec3::new(0.5, 0.2, 0.1));
        assert!((above - 0.2).abs() < TOL, "{above}");
        // Past the tip clamps to the endpoint.
        let beyond = links.distance(Vec3::new(1.0, 0.0, 0.1));
        assert!((beyond - 0.2).abs() < TOL, "{beyond}");
        // Below the base column clamps to the root.
        let base = links.distance(Vec3::new(0.0, 0.0, -0.05));
        assert!((base - 0.05).abs() < TOL, "{base}");
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
