//! Kinematics over the model-generated joint chain: 3D forward kinematics
//! plus dense damped least-squares IK, monomorphized on the chain length with
//! `Chain<const NJ>` so the solver loops specialize against `const` data.

use libm::{cosf, sinf, sqrtf};

use crate::kin::{self, Joint};

/// The generated chain as const data; `ARM.reach(..)` specializes to `NJ`.
pub(crate) const ARM: Chain<{ kin::NJ }> = Chain {
    joints: &kin::JOINTS,
    tip_offset: kin::TIP_OFFSET,
};

/// Damping of the least-squares normal equations (metres per radian) and the
/// convergence threshold / per-iteration joint step cap (metres, radians).
const DAMPING: f32 = 0.15;
const EPS: f32 = 2.0e-4;
const MAX_STEP: f32 = 0.5;

pub(crate) struct Chain<const NJ: usize> {
    joints: &'static [Joint; NJ],
    tip_offset: [f32; 3],
}

impl<const NJ: usize> Chain<NJ> {
    /// World position of the grasp point at joint configuration `q`.
    pub(crate) fn tip_pos(&self, q: &[f32; NJ]) -> [f32; 3] {
        let mut pos = [0.0; 3];
        let mut ori = IDENTITY;
        for (i, j) in self.joints.iter().enumerate() {
            pos = add(pos, qrot(ori, j.offset));
            ori = qmul(ori, j.quat);
            ori = qmul(ori, spin(j.axis, q[i]));
        }
        add(pos, qrot(ori, self.tip_offset))
    }

    /// Nudge `q` toward `target` (world frame, metres), keeping joints inside
    /// their ranges; returns the residual tip distance after the last step.
    pub(crate) fn reach(&self, q: &mut [f32; NJ], target: [f32; 3], iters: u32) -> f32 {
        for _ in 0..iters {
            let mut pos = [0.0; 3];
            let mut ori = IDENTITY;
            let mut anchor = [[0.0; 3]; NJ];
            let mut axis = [[0.0; 3]; NJ];
            for (i, j) in self.joints.iter().enumerate() {
                pos = add(pos, qrot(ori, j.offset));
                ori = qmul(ori, j.quat);
                anchor[i] = pos;
                axis[i] = qrot(ori, j.axis);
                ori = qmul(ori, spin(j.axis, q[i]));
            }
            let tip = add(pos, qrot(ori, self.tip_offset));
            let err = sub(target, tip);
            if dot3(err, err) < EPS * EPS {
                break;
            }

            // J Jᵀ + λ²I: each column of J is the tip velocity per joint rad.
            let mut cols = [[0.0; 3]; NJ];
            let mut m = [[0.0; 3]; 3];
            for (i, &a) in axis.iter().enumerate() {
                let c = cross(a, sub(tip, anchor[i]));
                cols[i] = c;
                for (r, &cv) in c.iter().enumerate() {
                    for (k, &kw) in c.iter().enumerate() {
                        m[r][k] += cv * kw;
                    }
                }
            }
            for (d, row) in m.iter_mut().enumerate() {
                row[d] += DAMPING * DAMPING;
            }
            let Some(y) = solve3(&m, err) else {
                break;
            };

            for (i, j) in self.joints.iter().enumerate() {
                let step = dot3(cols[i], y).clamp(-MAX_STEP, MAX_STEP);
                let mut qi = q[i] + step;
                if let Some([lo, hi]) = j.range {
                    qi = qi.clamp(lo, hi);
                }
                q[i] = qi;
            }
        }

        let err = sub(target, self.tip_pos(q));
        sqrtf(dot3(err, err))
    }
}

const IDENTITY: [f32; 4] = [1.0, 0.0, 0.0, 0.0];

fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// Hamilton product `a ∘ b`.
fn qmul(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [
        a[0] * b[0] - a[1] * b[1] - a[2] * b[2] - a[3] * b[3],
        a[0] * b[1] + a[1] * b[0] + a[2] * b[3] - a[3] * b[2],
        a[0] * b[2] - a[1] * b[3] + a[2] * b[0] + a[3] * b[1],
        a[0] * b[3] + a[1] * b[2] - a[2] * b[1] + a[3] * b[0],
    ]
}

fn scale(a: [f32; 3], s: f32) -> [f32; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}

fn qrot(q: [f32; 4], v: [f32; 3]) -> [f32; 3] {
    let u = [q[1], q[2], q[3]];
    let t = scale(cross(u, v), 2.0);
    add(add(v, scale(t, q[0])), cross(u, t))
}

/// Quaternion for a rotation of `angle` about the unit `axis`.
fn spin(axis: [f32; 3], angle: f32) -> [f32; 4] {
    let h = 0.5 * angle;
    let s = sinf(h);
    [cosf(h), axis[0] * s, axis[1] * s, axis[2] * s]
}

/// Cramer's rule on a 3×3 system; `None` when the matrix is singular.
fn solve3(m: &[[f32; 3]; 3], b: [f32; 3]) -> Option<[f32; 3]> {
    let [m00, m01, m02] = m[0];
    let [m10, m11, m12] = m[1];
    let [m20, m21, m22] = m[2];
    let det = m00 * (m11 * m22 - m12 * m21) - m01 * (m10 * m22 - m12 * m20)
        + m02 * (m10 * m21 - m11 * m20);
    if det.abs() < 1.0e-9 {
        return None;
    }
    let x = (b[0] * (m11 * m22 - m12 * m21) - m01 * (b[1] * m22 - m12 * b[2])
        + m02 * (b[1] * m21 - m11 * b[2]))
        / det;
    let y = (m00 * (b[1] * m22 - m12 * b[2]) - b[0] * (m10 * m22 - m12 * m20)
        + m02 * (m10 * b[2] - b[1] * m20))
        / det;
    let z = (m00 * (m11 * b[2] - b[1] * m21) - m01 * (m10 * b[2] - b[1] * m20)
        + b[0] * (m10 * m21 - m11 * m20))
        / det;
    Some([x, y, z])
}
