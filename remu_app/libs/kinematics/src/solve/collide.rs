//! Capsule obstacles and the segment–segment distance kernel behind the
//! solver's constraint rows.

use crate::vec::Vec3;

/// A capsule obstacle in the world frame: segment `a`–`b` swept by `radius`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Capsule {
    pub a: Vec3,
    pub b: Vec3,
    pub radius: f32,
}

/// Closest points between the segments `[p1, q1]` and `[p2, q2]` (Ericson,
/// *Real-Time Collision Detection* §5.1.9), robust to degenerate segments.
pub(crate) fn closest_points(p1: Vec3, q1: Vec3, p2: Vec3, q2: Vec3) -> (Vec3, Vec3) {
    let d1 = q1 - p1;
    let d2 = q2 - p2;
    let r = p1 - p2;
    let a = d1.dot(d1);
    let e = d2.dot(d2);
    let f = d2.dot(r);
    let (s, t) = match (a <= 1.0e-9, e <= 1.0e-9) {
        (true, true) => return (p1, p2),
        (true, false) => (0.0, (f / e).clamp(0.0, 1.0)),
        (false, true) => {
            let c = d1.dot(r);
            ((-c / a).clamp(0.0, 1.0), 0.0)
        }
        (false, false) => {
            let c = d1.dot(r);
            let b = d1.dot(d2);
            let denom = a * e - b * b;
            let mut s = if denom > 1.0e-9 {
                ((b * f - c * e) / denom).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let mut t = (b * s + f) / e;
            if t < 0.0 {
                t = 0.0;
                s = (-c / a).clamp(0.0, 1.0);
            } else if t > 1.0 {
                t = 1.0;
                s = ((b - c) / a).clamp(0.0, 1.0);
            }
            (s, t)
        }
    };
    (p1 + d1 * s, p2 + d2 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOL: f32 = 1.0e-3;

    fn seg_dist(p1: Vec3, q1: Vec3, p2: Vec3, q2: Vec3) -> f32 {
        let (a, b) = closest_points(p1, q1, p2, q2);
        let d = a - b;
        crate::math::sqrt(d.dot(d))
    }

    #[test]
    fn crossing_segments_touch() {
        let d = seg_dist(
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, -1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );
        assert!(d < TOL, "{d}");
    }

    #[test]
    fn parallel_segments_keep_their_offset() {
        let d = seg_dist(
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(-1.0, 0.5, 0.0),
            Vec3::new(1.0, 0.5, 0.0),
        );
        assert!((d - 0.5).abs() < TOL, "{d}");
    }

    #[test]
    fn degenerate_capsule_measures_point_to_segment() {
        let d = seg_dist(
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 0.7, 0.0),
            Vec3::new(0.0, 0.7, 0.0),
        );
        assert!((d - 0.7).abs() < TOL, "{d}");
    }
}
