//! Dense damped least-squares IK over a [`Chain`].
//!
//! The loop is generic over a [`SolvePolicy`]: [`Basic`] folds the task rows
//! (weighted position, optional orientation, optional nullspace posture) into
//! the `JᵀJ + λ²I` normal equations and always honors joint limits through
//! the active set; [`Avoiding`] additionally folds capsule-obstacle clearance
//! rows from the `avoid` layer over the `collide` distance kernel. Failures
//! are classified; `q` is left at the best configuration found.
//!
//! Obstacle avoidance is a soft constraint at the velocity level: every arm
//! spine segment closer than `Task::clearance` to an obstacle adds one
//! weighted row, so each step balances the task pull against pushing the gap
//! back out (equilibrium penetration scales with `1/avoid_weight²`). It is
//! local (gradient) avoidance: an obstacle wall between pose and target ends
//! in [`IkError::Unreachable`] with the best `gap`, not in a way around it.

remu_macro::mod_prv!(avoid, collide, flow, linalg);

pub use collide::Capsule;
pub use flow::{Avoiding, Basic, SolvePolicy};

use crate::chain::Chain;
use crate::math;
use crate::vec::{Quat, Vec3};

use self::linalg::{chol_solve, cholesky, dot6};

/// Damping of the least-squares normal equations (metres per radian), the
/// convergence threshold (metres / radians), the per-iteration joint step cap
/// (radians) and the stagnation / limit-push threshold (radians).
const DAMPING: f32 = 0.15;
const EPS: f32 = 2.0e-4;
const MAX_STEP: f32 = 0.5;
const STEP_EPS: f32 = 1.0e-5;

/// A reach task for [`Chain::solve`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Task<'a, const NJ: usize> {
    /// Tip position target, world frame, metres.
    pub target: Vec3,
    /// Scales the position block's rows; 0 disables the block.
    pub pos_weight: f32,
    /// Tool orientation target and its row weight.
    pub ori: Option<(Quat, f32)>,
    /// Preferred configuration and per-iteration gain; the pull is projected
    /// into the nullspace of the active task.
    pub posture: Option<([f32; NJ], f32)>,
    /// Capsule obstacles held at arm's length, world frame.
    pub obstacles: &'a [Capsule],
    /// Minimum clearance kept between the arm spine and each obstacle
    /// surface, metres.
    pub clearance: f32,
    /// Scales the obstacle constraint rows.
    pub avoid_weight: f32,
}

impl<'a, const NJ: usize> Task<'a, NJ> {
    /// Unit-weighted position-only task without obstacles.
    pub fn position(target: Vec3) -> Self {
        Self {
            target,
            pos_weight: 1.0,
            ori: None,
            posture: None,
            obstacles: &[],
            clearance: 0.02,
            avoid_weight: 1.0,
        }
    }
}

/// End-effector error: tip distance (metres), tool rotation angle (radians)
/// and the minimum obstacle surface gap (metres; infinite when the task has
/// no obstacles).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Residual {
    pub pos: f32,
    pub ori: f32,
    pub gap: f32,
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
    /// Stagnated with the target out of reach (geometry or obstacles) and no
    /// joint limit blocking the way.
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
    /// Position-only convenience wrapper around `solve::<Basic>`.
    pub fn reach(&self, q: &mut [f32; NJ], target: Vec3, iters: u32) -> Result<Solution, IkError> {
        self.solve::<Basic>(q, &Task::position(target), iters)
    }

    /// Nudge `q` toward `task` (world frame) within the iteration budget,
    /// keeping joints inside their ranges; the policy `P` selects the active
    /// layers (see [`SolvePolicy`]). Returns the iterations performed and the
    /// residual, or a classified [`IkError`] with the best-found `q` in
    /// place.
    pub fn solve<P: SolvePolicy>(
        &self,
        q: &mut [f32; NJ],
        task: &Task<'_, NJ>,
        iters: u32,
    ) -> Result<Solution, IkError> {
        for done in 0..iters {
            let spine = self.spine(q);
            let e_pos = task.target - spine.tip;
            let e_ori = match task.ori {
                Some((target_ori, _)) => (target_ori * spine.tip_ori.conjugate()).rotation_vector(),
                None => Vec3::ZERO,
            };

            // Weighted 6-row task error and seam columns; the obstacle scan
            // folds its constraint rows into the same system.
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
            let mut cols = [[0.0f32; 6]; NJ];
            for (i, a) in spine.axis.iter().enumerate() {
                let lin = a.cross(spine.tip - spine.anchor[i]) * wp;
                cols[i] = [lin[0], lin[1], lin[2], wo * a[0], wo * a[1], wo * a[2]];
            }
            let all = [true; NJ];
            let (mut a, mut b) = task_system(&cols, &e6, &all);
            let mut gap = f32::INFINITY;
            if P::OBSTACLES {
                gap = avoid::fold(
                    &spine,
                    &all,
                    task.obstacles,
                    task.clearance,
                    task.avoid_weight,
                    &mut a,
                    &mut b,
                );
            }

            let residual = Residual {
                pos: math::sqrt(e_pos.dot(e_pos)),
                ori: math::sqrt(e_ori.dot(e_ori)),
                gap,
            };
            if !residual.pos.is_finite() || !residual.ori.is_finite() || residual.gap.is_nan() {
                return Err(IkError::Singular { residual });
            }
            if converged(task, residual) {
                return Ok(Solution {
                    iters: done,
                    residual,
                });
            }

            let Some(mut l) = cholesky(a) else {
                return Err(IkError::Singular { residual });
            };
            let mut dq = chol_solve(&l, b);
            for d in dq.iter_mut() {
                *d = d.clamp(-MAX_STEP, MAX_STEP);
            }

            let mut free = [true; NJ];
            let mut clamped = false;
            for (i, j) in self.joints.iter().enumerate() {
                if let Some([lo, hi]) = j.range {
                    let cand = q[i] + dq[i];
                    free[i] = cand >= lo && cand <= hi;
                    clamped |= !free[i];
                }
            }

            // Active set: re-solve with the saturated joints pinned, so they
            // stop dragging the free ones.
            if clamped {
                let (mut a2, mut b2) = task_system(&cols, &e6, &free);
                if P::OBSTACLES {
                    avoid::fold(
                        &spine,
                        &free,
                        task.obstacles,
                        task.clearance,
                        task.avoid_weight,
                        &mut a2,
                        &mut b2,
                    );
                }
                if let Some(l2) = cholesky(a2) {
                    let d2 = chol_solve(&l2, b2);
                    for i in 0..NJ {
                        if free[i] {
                            dq[i] = d2[i].clamp(-MAX_STEP, MAX_STEP);
                        }
                    }
                    l = l2;
                }
            }

            // Nullspace posture: `λ²(JᵀJ + λ²I)⁻¹z`, the damped projection of
            // the pull against the active system.
            if let Some((pref, gain)) = task.posture {
                let mut rhs = [0.0f32; NJ];
                for i in 0..NJ {
                    if free[i] {
                        rhs[i] = gain * (pref[i] - q[i]) * DAMPING * DAMPING;
                    }
                }
                let post = chol_solve(&l, rhs);
                for i in 0..NJ {
                    if free[i] {
                        dq[i] = (dq[i] + post[i]).clamp(-MAX_STEP, MAX_STEP);
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

        let spine = self.spine(q);
        let e_pos = task.target - spine.tip;
        let e_ori = match task.ori {
            Some((target_ori, _)) => (target_ori * spine.tip_ori.conjugate()).rotation_vector(),
            None => Vec3::ZERO,
        };
        let all = [true; NJ];
        let mut scratch_a = [[0.0; NJ]; NJ];
        let mut scratch_b = [0.0; NJ];
        let mut gap = f32::INFINITY;
        if P::OBSTACLES {
            gap = avoid::fold(
                &spine,
                &all,
                task.obstacles,
                task.clearance,
                task.avoid_weight,
                &mut scratch_a,
                &mut scratch_b,
            );
        }
        let residual = Residual {
            pos: math::sqrt(e_pos.dot(e_pos)),
            ori: math::sqrt(e_ori.dot(e_ori)),
            gap,
        };
        if converged(task, residual) {
            Ok(Solution { iters, residual })
        } else {
            Err(IkError::Stalled { residual })
        }
    }
}

/// Whether the task's enabled blocks are within tolerance.
fn converged<const NJ: usize>(task: &Task<'_, NJ>, r: Residual) -> bool {
    let pos_ok = task.pos_weight == 0.0 || r.pos < EPS;
    let ori_ok = match task.ori {
        Some((_, w)) => w == 0.0 || r.ori < EPS,
        None => true,
    };
    pos_ok && ori_ok
}

/// `JᵀJ + λ²I` over the free joints, and its right-hand side `Jᵀe`; each
/// 6-row `cols[i]` is folded in as pairwise dots.
fn task_system<const NJ: usize>(
    cols: &[[f32; 6]; NJ],
    e6: &[f32; 6],
    free: &[bool; NJ],
) -> ([[f32; NJ]; NJ], [f32; NJ]) {
    let mut a = [[0.0; NJ]; NJ];
    let mut b = [0.0; NJ];
    for i in 0..NJ {
        if free[i] {
            b[i] = dot6(&cols[i], e6);
        }
        for j in 0..NJ {
            if free[i] && free[j] {
                a[i][j] += dot6(&cols[i], &cols[j]);
            }
        }
    }
    for (d, row) in a.iter_mut().enumerate() {
        row[d] += DAMPING * DAMPING;
    }
    (a, b)
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
    #[rustfmt::skip]
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
        let sol = ARM1.solve::<Basic>(&mut q, &task, 64).unwrap();
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
        let sol = ARM1.solve::<Basic>(&mut q, &task, 64).unwrap();
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
        let sol = ARM2.solve::<Basic>(&mut q, &task, 64).unwrap();
        assert!(sol.residual.pos < TOL, "{sol:?}");
        assert!(q[1] > 0.4, "{q:?}");
    }

    #[test]
    fn solve_detours_around_an_obstacle() {
        // A range-free planar arm sweeping from +x to 0.6 rad: the direct
        // arc passes through a capsule standing at 0.3 rad, so the elbow must
        // bend to route the tip around it.
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
        const OBS: [Capsule; 1] = [Capsule {
            a: Vec3([0.688, 0.213, 0.0]),
            b: Vec3([0.688, 0.213, 0.3]),
            radius: 0.04,
        }];
        let task = Task {
            obstacles: &OBS,
            clearance: 0.03,
            avoid_weight: 1.5,
            ..Task::position(Vec3::new(0.7 * math::cos(0.6), 0.7 * math::sin(0.6), 0.1))
        };
        let mut q = [0.0, 0.0];
        let sol = ARM2.solve::<Avoiding>(&mut q, &task, 128).unwrap();
        assert!(sol.residual.pos < TOL, "{sol:?}");
        assert!(sol.residual.gap > 0.02, "{sol:?}");
    }
}
