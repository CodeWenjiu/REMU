//! Dense damped least-squares IK over a [`Chain`]: `J·Jᵀ + λ²I` normal
//! equations solved per iteration, with joint-range clamping.

use crate::chain::Chain;
use crate::math;
use crate::vec::{Quat, Vec3};

/// Damping of the least-squares normal equations (metres per radian) and the
/// convergence threshold / per-iteration joint step cap (metres, radians).
const DAMPING: f32 = 0.15;
const EPS: f32 = 2.0e-4;
const MAX_STEP: f32 = 0.5;

impl<const NJ: usize> Chain<NJ> {
    /// Nudge `q` toward `target` (world frame, metres), keeping joints inside
    /// their ranges; returns the residual tip distance after the last step.
    pub fn reach(&self, q: &mut [f32; NJ], target: Vec3, iters: u32) -> f32 {
        for _ in 0..iters {
            let mut pos = Vec3::ZERO;
            let mut ori = Quat::IDENTITY;
            let mut anchor = [Vec3::ZERO; NJ];
            let mut axis = [Vec3::ZERO; NJ];
            for (i, j) in self.joints.iter().enumerate() {
                pos = pos + ori.rotate(j.offset);
                ori = ori * j.quat;
                anchor[i] = pos;
                axis[i] = ori.rotate(j.axis);
                ori = ori * Quat::from_axis_angle(j.axis, q[i]);
            }
            let tip = pos + ori.rotate(self.tip_offset);
            let err = target - tip;
            if err.dot(err) < EPS * EPS {
                break;
            }

            // J Jᵀ + λ²I: each column of J is the tip velocity per joint rad.
            let mut cols = [Vec3::ZERO; NJ];
            let mut m = [[0.0; 3]; 3];
            for (i, &a) in axis.iter().enumerate() {
                let c = a.cross(tip - anchor[i]);
                cols[i] = c;
                for r in 0..3 {
                    for k in 0..3 {
                        m[r][k] += c[r] * c[k];
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
                let step = cols[i].dot(y).clamp(-MAX_STEP, MAX_STEP);
                let mut qi = q[i] + step;
                if let Some([lo, hi]) = j.range {
                    qi = qi.clamp(lo, hi);
                }
                q[i] = qi;
            }
        }

        let err = target - self.tip_pos(q);
        math::sqrt(err.dot(err))
    }
}

/// Cramer's rule on a 3×3 system; `None` when the matrix is singular.
fn solve3(m: &[[f32; 3]; 3], b: Vec3) -> Option<Vec3> {
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
    Some(Vec3::new(x, y, z))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::Joint;

    /// Backend-agnostic tolerance (see `math.rs`): micromath trig caps
    /// accuracy at ~0.002 per call, so chained results need headroom.
    const TOL: f32 = 5.0e-3;

    /// Copy of the Panda chain from robot_arm's generated `kinematics.rs`
    /// (test fixture; the app-side data stays the source of truth).
    const PANDA_JOINTS: [Joint; 7] = [
        Joint { offset: Vec3([0.0, 0.0, 0.333]), quat: Quat([1.0, 0.0, 0.0, 0.0]), axis: Vec3([0.0, 0.0, 1.0]), range: Some([-2.8973, 2.8973]) },
        Joint { offset: Vec3([0.0, 0.0, 0.0]), quat: Quat([0.70710677, -0.70710677, 0.0, 0.0]), axis: Vec3([0.0, 0.0, 1.0]), range: Some([-1.7628, 1.7628]) },
        Joint { offset: Vec3([0.0, -0.316, 0.0]), quat: Quat([0.70710677, 0.70710677, 0.0, 0.0]), axis: Vec3([0.0, 0.0, 1.0]), range: Some([-2.8973, 2.8973]) },
        Joint { offset: Vec3([0.0825, 0.0, 0.0]), quat: Quat([0.70710677, 0.70710677, 0.0, 0.0]), axis: Vec3([0.0, 0.0, 1.0]), range: Some([-3.0718, -0.0698]) },
        Joint { offset: Vec3([-0.0825, 0.384, 0.0]), quat: Quat([0.70710677, -0.70710677, 0.0, 0.0]), axis: Vec3([0.0, 0.0, 1.0]), range: Some([-2.8973, 2.8973]) },
        Joint { offset: Vec3([0.0, 0.0, 0.0]), quat: Quat([0.70710677, 0.70710677, 0.0, 0.0]), axis: Vec3([0.0, 0.0, 1.0]), range: Some([-0.0175, 3.7525]) },
        Joint { offset: Vec3([0.088, 0.0, 0.0]), quat: Quat([0.70710677, 0.70710677, 0.0, 0.0]), axis: Vec3([0.0, 0.0, 1.0]), range: Some([-2.8973, 2.8973]) },
    ];
    const PANDA: Chain<7> = Chain {
        joints: &PANDA_JOINTS,
        tip_offset: Vec3([0.0, 0.0, 0.107]),
    };
    #[allow(clippy::approx_constant)]
    const HOME: [f32; 7] = [0.0, 0.0, 0.0, -1.57079, 0.0, 1.57079, -0.7853];

    #[test]
    fn home_tip_matches_the_documented_convention() {
        // Independent f64 reference computed from the frame convention
        // documented in mjcf's generated header:
        // (0.5544994337511387, 2.6e-08, 0.6245024522842164).
        let p = PANDA.tip_pos(&HOME);
        assert!((p[0] - 0.554_499_4).abs() < TOL, "{p:?}");
        assert!(p[1].abs() < TOL, "{p:?}");
        assert!((p[2] - 0.624_502_5).abs() < TOL, "{p:?}");
    }

    #[test]
    fn reach_converges_on_a_nearby_target() {
        let mut q = HOME;
        let home = PANDA.tip_pos(&q);
        let target = Vec3::new(home[0] + 0.02, home[1] + 0.03, home[2] - 0.01);
        let res = PANDA.reach(&mut q, target, 64);
        assert!(res < TOL, "residual {res}");
        let tip = PANDA.tip_pos(&q);
        for (t, p) in target.0.iter().zip(tip.0) {
            assert!((t - p).abs() < TOL, "{tip:?}");
        }
    }

    #[test]
    fn reach_respects_joint_ranges() {
        // One joint with a range far short of the 90° the target needs: the
        // solver must park at the limit and report the residual instead.
        const ONE: [Joint; 1] = [Joint {
            offset: Vec3::ZERO,
            quat: Quat::IDENTITY,
            axis: Vec3([0.0, 0.0, 1.0]),
            range: Some([0.0, 0.1]),
        }];
        const ARM1: Chain<1> = Chain {
            joints: &ONE,
            tip_offset: Vec3([1.0, 0.0, 0.0]),
        };
        let mut q = [0.0];
        let res = ARM1.reach(&mut q, Vec3::new(0.0, 1.0, 0.0), 64);
        assert!((q[0] - 0.1).abs() < 1.0e-6, "q = {q:?}");
        assert!(res > 1.0, "residual {res}");
    }

    #[test]
    fn reach_stops_when_already_there() {
        let mut q = HOME;
        let tip = PANDA.tip_pos(&q);
        let res = PANDA.reach(&mut q, tip, 32);
        assert!(res < 1.0e-6, "residual {res}");
        assert_eq!(q, HOME);
    }
}
