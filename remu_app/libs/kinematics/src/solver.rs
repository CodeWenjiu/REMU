//! Dense damped least-squares IK over a [`Chain`]: `J·Jᵀ + λ²I` normal
//! equations solved per iteration with joint-range clamping and classified
//! failure diagnostics.

use crate::chain::Chain;
use crate::math;
use crate::vec::{Quat, Vec3};

/// Damping of the least-squares normal equations (metres per radian), the
/// convergence threshold (metres), the per-iteration joint step cap (radians)
/// and the stagnation / limit-push threshold (radians).
const DAMPING: f32 = 0.15;
const EPS: f32 = 2.0e-4;
const MAX_STEP: f32 = 0.5;
const STEP_EPS: f32 = 1.0e-5;

/// Successful solve: iterations performed and the residual tip distance
/// (metres).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Solution {
    pub iters: u32,
    pub residual: f32,
}

/// A solve that did not reach the target; `q` is left at the best
/// configuration found, ready for warm-started continuation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum IkError {
    /// Stagnated with the target geometrically out of reach and no joint
    /// limit blocking the way.
    Unreachable { residual: f32 },
    /// Stagnated with at least one joint pinned at its range limit while
    /// pushing outward.
    LimitLocked { residual: f32 },
    /// Iteration budget ran out while the solver was still making progress.
    Stalled { residual: f32 },
    /// The damped normal equations broke down (singular or non-finite).
    Singular { residual: f32 },
}

impl IkError {
    /// Final tip distance to the target, in metres.
    pub fn residual(self) -> f32 {
        match self {
            Self::Unreachable { residual }
            | Self::LimitLocked { residual }
            | Self::Stalled { residual }
            | Self::Singular { residual } => residual,
        }
    }
}

impl<const NJ: usize> Chain<NJ> {
    /// Nudge `q` toward `target` (world frame, metres), keeping joints inside
    /// their ranges; returns the iterations performed and the residual tip
    /// distance, or a classified [`IkError`] with the best-found `q` in place.
    pub fn reach(&self, q: &mut [f32; NJ], target: Vec3, iters: u32) -> Result<Solution, IkError> {
        for done in 0..iters {
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
            let err_sq = err.dot(err);
            if !err_sq.is_finite() {
                return Err(IkError::Singular { residual: f32::NAN });
            }
            let residual = math::sqrt(err_sq);
            if residual < EPS {
                return Ok(Solution {
                    iters: done,
                    residual,
                });
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
                return Err(IkError::Singular { residual });
            };

            let mut dq = [0.0f32; NJ];
            let mut free = [true; NJ];
            let mut clamped = false;
            for (i, j) in self.joints.iter().enumerate() {
                let d = cols[i].dot(y).clamp(-MAX_STEP, MAX_STEP);
                dq[i] = d;
                if let Some([lo, hi]) = j.range {
                    let cand = q[i] + d;
                    free[i] = cand >= lo && cand <= hi;
                    clamped |= !free[i];
                }
            }

            // Active set: re-solve with the saturated joints pinned, so they
            // stop dragging the free ones.
            if clamped {
                let mut m = [[0.0; 3]; 3];
                for i in 0..NJ {
                    if free[i] {
                        let c = cols[i];
                        for r in 0..3 {
                            for k in 0..3 {
                                m[r][k] += c[r] * c[k];
                            }
                        }
                    }
                }
                for (d, row) in m.iter_mut().enumerate() {
                    row[d] += DAMPING * DAMPING;
                }
                if let Some(y) = solve3(&m, err) {
                    for i in 0..NJ {
                        if free[i] {
                            dq[i] = cols[i].dot(y).clamp(-MAX_STEP, MAX_STEP);
                        }
                    }
                }
            }

            let mut blocked = false;
            let mut moved_sq = 0.0;
            for (i, j) in self.joints.iter().enumerate() {
                let cand = q[i] + dq[i];
                let qi = match j.range {
                    Some([_, hi]) if cand > hi => {
                        blocked |= dq[i] > STEP_EPS;
                        hi
                    }
                    Some([lo, _]) if cand < lo => {
                        blocked |= dq[i] < -STEP_EPS;
                        lo
                    }
                    _ => cand,
                };
                moved_sq += (qi - q[i]) * (qi - q[i]);
                q[i] = qi;
            }

            if moved_sq < STEP_EPS * STEP_EPS {
                return Err(if blocked {
                    IkError::LimitLocked { residual }
                } else {
                    IkError::Unreachable { residual }
                });
            }
        }

        let err = target - self.tip_pos(q);
        let residual = math::sqrt(err.dot(err));
        if residual < EPS {
            Ok(Solution { iters, residual })
        } else {
            Err(IkError::Stalled { residual })
        }
    }
}

/// Cramer's rule on a 3×3 system; `None` when the matrix is singular.
fn solve3(m: &[[f32; 3]; 3], b: Vec3) -> Option<Vec3> {
    let [m00, m01, m02] = m[0];
    let [m10, m11, m12] = m[1];
    let [m20, m21, m22] = m[2];
    let det = m00 * (m11 * m22 - m12 * m21) - m01 * (m10 * m22 - m12 * m20)
        + m02 * (m10 * m21 - m11 * m20);
    if !det.is_finite() || det.abs() < 1.0e-9 {
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
        let sol = PANDA.reach(&mut q, target, 64).unwrap();
        assert!(sol.residual < TOL, "{sol:?}");
        let tip = PANDA.tip_pos(&q);
        for (t, p) in target.0.iter().zip(tip.0) {
            assert!((t - p).abs() < TOL, "{tip:?}");
        }
    }

    #[test]
    fn reach_respects_joint_ranges() {
        // One joint with a range far short of the 90° the target needs: the
        // solver must park at the limit and report it as locked, not
        // unreachable.
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
        let err = ARM1
            .reach(&mut q, Vec3::new(0.0, 1.0, 0.0), 64)
            .unwrap_err();
        assert!(matches!(err, IkError::LimitLocked { .. }), "{err:?}");
        assert!((q[0] - 0.1).abs() < 1.0e-6, "q = {q:?}");
        assert!(err.residual() > 1.0, "{err:?}");
    }

    #[test]
    fn reach_reports_unreachable_beyond_the_workspace() {
        // A range-free planar arm: the solver stalls at full x extension,
        // 9.2 m short of the target, with no limit to blame.
        const FLAT: [Joint; 2] = [
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
        const ARM2: Chain<2> = Chain {
            joints: &FLAT,
            tip_offset: Vec3([0.3, 0.0, 0.0]),
        };
        let mut q = [0.0, 0.0];
        let err = ARM2
            .reach(&mut q, Vec3::new(10.0, 0.0, 0.1), 128)
            .unwrap_err();
        assert!(matches!(err, IkError::Unreachable { .. }), "{err:?}");
        assert!((err.residual() - 9.2).abs() < 0.05, "{err:?}");
    }

    #[test]
    fn reach_reports_stalled_when_the_budget_runs_out() {
        // One joint asked to turn ~180°: starting off the antipodal dead
        // point keeps the damped steps productive, but four iterations are
        // far from enough.
        const ONE: [Joint; 1] = [Joint {
            offset: Vec3::ZERO,
            quat: Quat::IDENTITY,
            axis: Vec3([0.0, 0.0, 1.0]),
            range: None,
        }];
        const ARM1: Chain<1> = Chain {
            joints: &ONE,
            tip_offset: Vec3([1.0, 0.0, 0.0]),
        };
        let mut q = [0.1];
        let err = ARM1
            .reach(&mut q, Vec3::new(-1.0, 0.0, 0.0), 4)
            .unwrap_err();
        assert!(matches!(err, IkError::Stalled { .. }), "{err:?}");
        assert!(err.residual() > 0.5, "{err:?}");
    }

    #[test]
    fn reach_stops_when_already_there() {
        let mut q = HOME;
        let tip = PANDA.tip_pos(&q);
        let sol = PANDA.reach(&mut q, tip, 32).unwrap();
        assert!(sol.residual < 1.0e-6, "{sol:?}");
        assert_eq!(sol.iters, 0);
        assert_eq!(q, HOME);
    }
}
