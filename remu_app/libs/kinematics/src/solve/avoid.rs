//! The obstacle-avoidance layer: fold capsule clearance constraints into the
//! solver's normal equations.

use crate::chain::Spine;
use crate::math;
use crate::solve::collide::{Capsule, closest_points};

/// Accumulate the weighted obstacle constraint rows for the free joints into
/// `a`/`b` and return the minimum surface gap over all links × obstacles.
pub(crate) fn fold<const NJ: usize>(
    spine: &Spine<NJ>,
    free: &[bool; NJ],
    obstacles: &[Capsule],
    clearance: f32,
    weight: f32,
    a: &mut [[f32; NJ]; NJ],
    b: &mut [f32; NJ],
) -> f32 {
    let mut gap = f32::INFINITY;
    for obs in obstacles {
        for i in 0..NJ {
            let end = if i + 1 < NJ {
                spine.anchor[i + 1]
            } else {
                spine.tip
            };
            let (p, o) = closest_points(spine.anchor[i], end, obs.a, obs.b);
            let d = p - o;
            let dist = math::sqrt(d.dot(d));
            let g = dist - obs.radius;
            if g < gap {
                gap = g;
            }
            if g >= clearance || dist < 1.0e-6 {
                continue;
            }
            // Error and row are both scaled by the weight (a squared share in
            // the objective); the row is ∂gap/∂q restricted to the joints
            // upstream of the closest link point.
            let e = (clearance - g) * weight;
            let n_hat = d * (1.0 / dist);
            let mut row = [0.0f32; NJ];
            for (j, axis) in spine.axis.iter().enumerate().take(i + 1) {
                row[j] = weight * n_hat.dot(axis.cross(p - spine.anchor[j]));
            }
            for r in 0..NJ {
                if !free[r] {
                    continue;
                }
                b[r] += e * row[r];
                for c in 0..NJ {
                    if free[c] {
                        a[r][c] += row[r] * row[c];
                    }
                }
            }
        }
    }
    gap
}
