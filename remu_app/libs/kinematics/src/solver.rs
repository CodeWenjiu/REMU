//! Dense damped least-squares IK over a [`Chain`]: weighted 6-DOF tasks
//! (tip position and optional tool orientation, plus a nullspace posture
//! pull), with joint-range clamping, an active set for saturated joints, and
//! classified failure diagnostics.

use crate::chain::Chain;
use crate::math;
use crate::vec::{Quat, Vec3};

/// Damping of the least-squares normal equations (metres per radian), the
/// convergence threshold (metres / radians), the per-iteration joint step cap
/// (radians) and the stagnation / limit-push threshold (radians).
const DAMPING: f32 = 0.15;
const EPS: f32 = 2.0e-4;
const MAX_STEP: f32 = 0.5;
const STEP_EPS: f32 = 1.0e-5;

/// A reach task for [`Chain::solve`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Task<const NJ: usize> {
    /// Tip position target, world frame, metres.
    pub target: Vec3,
    /// Scales the position block's rows; 0 disables the block.
    pub pos_weight: f32,
    /// Tool orientation target and its row weight.
    pub ori: Option<(Quat, f32)>,
    /// Preferred configuration and per-iteration gain; the pull is projected
    /// into the nullspace of the active task.
    pub posture: Option<([f32; NJ], f32)>,
}

impl<const NJ: usize> Task<NJ> {
    /// Unit-weighted position-only task.
    pub fn position(target: Vec3) -> Self {
        Self {
            target,
            pos_weight: 1.0,
            ori: None,
            posture: None,
        }
    }
}

/// End-effector error: tip distance (metres) and tool rotation angle (radians).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Residual {
    pub pos: f32,
    pub ori: f32,
}

/// Successful solve: iterations performed and the final task residual.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Solution {
    pub iters: u32,
    pub residual: Residual,
}

/// A solve that did not reach the task; `q` is left at the best configuration
/// found, ready for warm-started continuation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum IkError {
    /// Stagnated with the target geometrically out of reach and no joint
    /// limit blocking the way.
    Unreachable { residual: Residual },
    /// Stagnated with at least one joint pinned at its range limit while
    /// pushing outward.
    LimitLocked { residual: Residual },
    /// Iteration budget ran out while the solver was still making progress.
    Stalled { residual: Residual },
    /// The damped normal equations broke down (singular or non-finite).
    Singular { residual: Residual },
}

impl IkError {
    /// Final task residual.
    pub fn residual(self) -> Residual {
        match self {
            Self::Unreachable { residual }
            | Self::LimitLocked { residual }
            | Self::Stalled { residual }
            | Self::Singular { residual } => residual,
        }
    }
}

impl<const NJ: usize> Chain<NJ> {
    /// Position-only convenience wrapper around [`Chain::solve`].
    pub fn reach(&self, q: &mut [f32; NJ], target: Vec3, iters: u32) -> Result<Solution, IkError> {
        self.solve(q, &Task::position(target), iters)
    }

    /// Nudge `q` toward `task` (world frame) within the iteration budget,
    /// keeping joints inside their ranges; returns the iterations performed
    /// and the residual, or a classified [`IkError`] with the best-found `q`
    /// in place.
    pub fn solve(
        &self,
        q: &mut [f32; NJ],
        task: &Task<NJ>,
        iters: u32,
    ) -> Result<Solution, IkError> {
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
            let tip_ori = ori * self.tip_quat;

            let e_pos = task.target - tip;
            let e_ori = match task.ori {
                Some((target_ori, _)) => (target_ori * tip_ori.conjugate()).rotation_vector(),
                None => Vec3::ZERO,
            };
            let residual = Residual {
                pos: math::sqrt(e_pos.dot(e_pos)),
                ori: math::sqrt(e_ori.dot(e_ori)),
            };
            if !residual.pos.is_finite() || !residual.ori.is_finite() {
                return Err(IkError::Singular { residual });
            }
            if converged(task, residual) {
                return Ok(Solution {
                    iters: done,
                    residual,
                });
            }

            // Weighted 6-row error: [position; rotation vector].
            let wp = task.pos_weight;
            let wo = match task.ori {
                Some((_, w)) => w,
                None => 0.0,
            };
            let mut e6 = [0.0; 6];
            for k in 0..3 {
                e6[k] = wp * e_pos[k];
                e6[3 + k] = wo * e_ori[k];
            }

            // J Jᵀ + λ²I: a column is the tip velocity (linear; angular) per
            // joint rad, with the task weights folded in.
            let mut cols = [[0.0f32; 6]; NJ];
            for (i, a) in axis.iter().enumerate() {
                let lin = a.cross(tip - anchor[i]) * wp;
                cols[i] = [lin[0], lin[1], lin[2], wo * a[0], wo * a[1], wo * a[2]];
            }
            let all = [true; NJ];
            let mut m = gram(&cols, &all);
            let Some(y) = solve6(&m, e6) else {
                return Err(IkError::Singular { residual });
            };

            let mut dq = [0.0f32; NJ];
            let mut free = [true; NJ];
            let mut clamped = false;
            for (i, j) in self.joints.iter().enumerate() {
                let d = dot6(&cols[i], &y).clamp(-MAX_STEP, MAX_STEP);
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
                m = gram(&cols, &free);
                if let Some(y) = solve6(&m, e6) {
                    for i in 0..NJ {
                        if free[i] {
                            dq[i] = dot6(&cols[i], &y).clamp(-MAX_STEP, MAX_STEP);
                        }
                    }
                }
            }

            // Nullspace posture: project the pull against the active system.
            if let Some((pref, gain)) = task.posture {
                let mut z = [0.0f32; NJ];
                let mut z6 = [0.0f32; 6];
                for i in 0..NJ {
                    if free[i] {
                        z[i] = gain * (pref[i] - q[i]);
                        for k in 0..6 {
                            z6[k] += cols[i][k] * z[i];
                        }
                    }
                }
                if let Some(y) = solve6(&m, z6) {
                    for i in 0..NJ {
                        if free[i] {
                            let mut d = dq[i] + z[i];
                            for k in 0..6 {
                                d -= cols[i][k] * y[k];
                            }
                            dq[i] = d.clamp(-MAX_STEP, MAX_STEP);
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

        let (tip, tip_ori) = self.tip_frame(q);
        let e_pos = task.target - tip;
        let e_ori = match task.ori {
            Some((target_ori, _)) => (target_ori * tip_ori.conjugate()).rotation_vector(),
            None => Vec3::ZERO,
        };
        let residual = Residual {
            pos: math::sqrt(e_pos.dot(e_pos)),
            ori: math::sqrt(e_ori.dot(e_ori)),
        };
        if converged(task, residual) {
            Ok(Solution { iters, residual })
        } else {
            Err(IkError::Stalled { residual })
        }
    }
}

/// Whether the task's enabled blocks are within tolerance.
fn converged<const NJ: usize>(task: &Task<NJ>, r: Residual) -> bool {
    let pos_ok = task.pos_weight == 0.0 || r.pos < EPS;
    let ori_ok = match task.ori {
        Some((_, w)) => w == 0.0 || r.ori < EPS,
        None => true,
    };
    pos_ok && ori_ok
}

/// `J Jᵀ + λ²I` over the free columns.
fn gram<const NJ: usize>(cols: &[[f32; 6]; NJ], free: &[bool; NJ]) -> [[f32; 6]; 6] {
    let mut m = [[0.0; 6]; 6];
    for i in 0..NJ {
        if free[i] {
            let c = cols[i];
            for r in 0..6 {
                for k in 0..6 {
                    m[r][k] += c[r] * c[k];
                }
            }
        }
    }
    for (d, row) in m.iter_mut().enumerate() {
        row[d] += DAMPING * DAMPING;
    }
    m
}

fn dot6(a: &[f32; 6], b: &[f32; 6]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// Gaussian elimination with partial pivoting; `None` when the system breaks
/// down (singular or non-finite).
fn solve6(m: &[[f32; 6]; 6], mut b: [f32; 6]) -> Option<[f32; 6]> {
    let mut a = *m;
    for col in 0..6 {
        let mut p = col;
        for r in col + 1..6 {
            if a[r][col].abs() > a[p][col].abs() {
                p = r;
            }
        }
        if !a[p][col].is_finite() || a[p][col].abs() < 1.0e-9 {
            return None;
        }
        a.swap(col, p);
        b.swap(col, p);
        for r in col + 1..6 {
            let f = a[r][col] / a[col][col];
            for c in col..6 {
                a[r][c] -= f * a[col][c];
            }
            b[r] -= f * b[col];
        }
    }
    let mut x = [0.0; 6];
    for col in (0..6).rev() {
        let mut s = b[col];
        for c in col + 1..6 {
            s -= a[col][c] * x[c];
        }
        x[col] = s / a[col][col];
    }
    Some(x)
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
        tip_quat: Quat([0.92387956, 0.0, 0.0, -0.38268343]),
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
        assert!(sol.residual.pos < TOL, "{sol:?}");
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
            tip_quat: Quat::IDENTITY,
        };
        let mut q = [0.0];
        let err = ARM1
            .reach(&mut q, Vec3::new(0.0, 1.0, 0.0), 64)
            .unwrap_err();
        assert!(matches!(err, IkError::LimitLocked { .. }), "{err:?}");
        assert!((q[0] - 0.1).abs() < 1.0e-6, "q = {q:?}");
        assert!(err.residual().pos > 1.0, "{err:?}");
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
            tip_quat: Quat::IDENTITY,
        };
        let mut q = [0.0, 0.0];
        let err = ARM2
            .reach(&mut q, Vec3::new(10.0, 0.0, 0.1), 128)
            .unwrap_err();
        assert!(matches!(err, IkError::Unreachable { .. }), "{err:?}");
        assert!((err.residual().pos - 9.2).abs() < 0.05, "{err:?}");
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
            tip_quat: Quat::IDENTITY,
        };
        let mut q = [0.1];
        let err = ARM1
            .reach(&mut q, Vec3::new(-1.0, 0.0, 0.0), 4)
            .unwrap_err();
        assert!(matches!(err, IkError::Stalled { .. }), "{err:?}");
        assert!(err.residual().pos > 0.5, "{err:?}");
    }

    #[test]
    fn reach_stops_when_already_there() {
        let mut q = HOME;
        let tip = PANDA.tip_pos(&q);
        let sol = PANDA.reach(&mut q, tip, 32).unwrap();
        assert!(sol.residual.pos < 1.0e-6, "{sol:?}");
        assert_eq!(sol.iters, 0);
        assert_eq!(q, HOME);
    }

    #[test]
    fn solve_reaches_orientation() {
        const ONE: [Joint; 1] = [Joint {
            offset: Vec3::ZERO,
            quat: Quat::IDENTITY,
            axis: Vec3([0.0, 0.0, 1.0]),
            range: None,
        }];
        const ARM1: Chain<1> = Chain {
            joints: &ONE,
            tip_offset: Vec3([1.0, 0.0, 0.0]),
            tip_quat: Quat::IDENTITY,
        };
        // Self-consistent targets: both blocks come from the chain's own FK,
        // so backends that distort the frame geometry stay solvable.
        let (target, target_ori) = ARM1.tip_frame(&[0.7]);
        let task = Task {
            ori: Some((target_ori, 2.0)),
            ..Task::position(target)
        };
        let mut q = [0.0];
        let sol = ARM1.solve(&mut q, &task, 64).unwrap();
        assert!((q[0] - 0.7).abs() < TOL, "{q:?}");
        assert!(sol.residual.pos < TOL && sol.residual.ori < TOL, "{sol:?}");
    }

    #[test]
    fn solve_ignores_zero_weighted_orientation() {
        // An orientation target a z-joint can never match, disabled by weight
        // 0: acceptance must fall back to the position block alone.
        const ONE: [Joint; 1] = [Joint {
            offset: Vec3::ZERO,
            quat: Quat::IDENTITY,
            axis: Vec3([0.0, 0.0, 1.0]),
            range: None,
        }];
        const ARM1: Chain<1> = Chain {
            joints: &ONE,
            tip_offset: Vec3([1.0, 0.0, 0.0]),
            tip_quat: Quat::IDENTITY,
        };
        let task = Task {
            ori: Some((Quat::from_axis_angle(Vec3::new(1.0, 0.0, 0.0), 1.0), 0.0)),
            ..Task::position(Vec3::new(0.0, 1.0, 0.0))
        };
        let mut q = [0.0];
        let sol = ARM1.solve(&mut q, &task, 64).unwrap();
        assert!(sol.residual.pos < TOL, "{sol:?}");
        assert!(sol.residual.ori > 0.5, "{sol:?}");
    }

    #[test]
    fn solve_pulls_redundant_joints_toward_the_posture() {
        // Joint 2 sits exactly at the tip point, so it is pure nullspace of
        // the position task: only the posture pull can move it.
        const TWO: [Joint; 2] = [
            Joint {
                offset: Vec3::ZERO,
                quat: Quat::IDENTITY,
                axis: Vec3([0.0, 0.0, 1.0]),
                range: None,
            },
            Joint {
                offset: Vec3([1.0, 0.0, 0.0]),
                quat: Quat::IDENTITY,
                axis: Vec3([0.0, 0.0, 1.0]),
                range: None,
            },
        ];
        const ARM2: Chain<2> = Chain {
            joints: &TWO,
            tip_offset: Vec3::ZERO,
            tip_quat: Quat::IDENTITY,
        };
        let target = Vec3::new(math::cos(0.3), math::sin(0.3), 0.0);

        let mut plain = [0.0, 0.0];
        ARM2.reach(&mut plain, target, 64).unwrap();
        assert!(plain[1].abs() < 1.0e-6, "{plain:?}");

        let task = Task {
            posture: Some(([0.3, 1.0], 0.5)),
            ..Task::position(target)
        };
        let mut q = [0.0, 0.0];
        let sol = ARM2.solve(&mut q, &task, 64).unwrap();
        assert!(sol.residual.pos < TOL, "{sol:?}");
        assert!(q[1] > 0.4, "{q:?}");
    }
}
