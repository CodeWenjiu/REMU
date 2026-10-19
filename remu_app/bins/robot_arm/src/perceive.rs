//! Perception: scan frames become solver obstacles.
//!
//! Reads a laser fan, drops self returns (proximity to the arm's own link
//! polyline from forward kinematics — the self filter a real controller runs
//! against its own geometry), and reports the rest as vertical columns: a 2D
//! return means "something vertical stands here", the standard 2.5D reading
//! of a planar scanner.
//!
//! Columns are remembered while their view stays blocked: the arm occludes
//! its own sensors near an obstacle (the sight line grazes the base), and
//! erasing obstacles at first occlusion is how controllers drive through
//! things. A column is cleared only when its own ray returns empty space;
//! a self return clears nothing. This is occupancy-grid style memory, not
//! cheating: it only ever holds what was actually seen.
//!
//! Device I/O lives with the caller (`main` reads the scan registers into a
//! [`Fan`]), so all of the logic below runs — and is tested — without a
//! device.

use kinematics::prelude::{Capsule, Links, Vec3};

/// Most rays a fan may carry (register-map capacity of the scan device).
const MAX_RAYS: usize = 64;
/// Obstacles kept at once; the fan sees the demo pole with one or two rays.
const MAX_OBSTACLES: usize = 16;
/// Safety envelope around a return: covers the ray spacing (half the arc at
/// 1.4 m is ~0.07 m).
const HIT_RADIUS: f32 = 0.08;
/// Returns closer than this to the arm's own link polyline are self hits.
/// A generous envelope: real link surfaces (elbow covers, the hand) sit well
/// off the joint-frame chords; returns that drift into the envelope during a
/// close approach are bridged by the column memory.
const SELF_RADIUS: f32 = 0.22;
/// Columns run from the floor to this height (above the demo arm's reach).
const COLUMN_TOP: f32 = 1.2;
/// Returns within this radius of a remembered column refresh it in place.
const MERGE_RADIUS: f32 = 0.12;
/// Backstop age for a column whose ray has not resolved either way (~10 s at
/// 60 Hz ticks). Seeing a no-return on the column's own ray clears it right
/// away; a self return keeps it (the view is blocked, not empty).
const COLUMN_TTL: u32 = 600;

const NO_CAPSULE: Capsule = Capsule {
    a: Vec3::new(0.0, 0.0, 0.0),
    b: Vec3::new(0.0, 0.0, 0.0),
    radius: 0.0,
};

/// One scan frame as read from a planar laser fan, in the world frame.
#[derive(Clone, Copy)]
pub(crate) struct Fan<'a> {
    /// Fan origin (the sensor mount), world frame, metres.
    pub(crate) origin: Vec3,
    /// First ray's yaw about world Z (radians).
    pub(crate) angle_min: f32,
    /// Yaw step per ray (radians).
    pub(crate) angle_incr: f32,
    /// Range bound; a return at or beyond this is "no return".
    pub(crate) range_max: f32,
    /// Surface distance per ray (metres).
    pub(crate) ranges: &'a [f32],
}

/// The app-side perception state: remembered obstacle columns plus the last
/// scan's self-hit count.
pub(crate) struct Perception {
    caps: [Capsule; MAX_OBSTACLES],
    /// Tick of the last sighting; the live prefix is `0..count`.
    seen: [u32; MAX_OBSTACLES],
    /// Ray each column was last returned by (its clearance authority).
    ray: [usize; MAX_OBSTACLES],
    count: usize,
    tick: u32,
    self_hits: u32,
}

impl Perception {
    pub(crate) const fn new() -> Self {
        Self {
            caps: [NO_CAPSULE; MAX_OBSTACLES],
            seen: [0; MAX_OBSTACLES],
            ray: [0; MAX_OBSTACLES],
            count: 0,
            tick: 0,
            self_hits: 0,
        }
    }

    /// Fold one scan frame into the obstacle set, against the arm's current
    /// link polyline.
    pub(crate) fn update<const NJ: usize>(&mut self, fan: &Fan<'_>, links: &Links<NJ>) {
        self.tick = self.tick.wrapping_add(1);

        // Classify every ray: 0 = no return, 1 = self, 2 = obstacle return.
        let mut class = [0u8; MAX_RAYS];
        self.self_hits = 0;
        let n = fan.ranges.len().min(MAX_RAYS);
        for (i, c) in class.iter_mut().enumerate().take(n) {
            let r = fan.ranges[i];
            if r >= fan.range_max {
                continue; // no return within the sensor's band
            }
            let a = fan.angle_min + fan.angle_incr * i as f32;
            let hit = fan.origin + Vec3::new(r * libm::cosf(a), r * libm::sinf(a), 0.0);
            if links.distance(hit) < SELF_RADIUS {
                *c = 1;
                self.self_hits += 1;
                continue;
            }
            *c = 2;
            self.insert(i, hit);
        }
        self.expire(&class);
    }

    /// Live columns for the solver.
    pub(crate) fn obstacles(&self) -> &[Capsule] {
        &self.caps[..self.count]
    }

    /// Self returns in the last scan cycle (telemetry).
    pub(crate) fn self_hits(&self) -> u32 {
        self.self_hits
    }

    /// Drop a column once its own ray has looked into empty space (a no-return
    /// means nothing is there anymore); self returns never clear — the view is
    /// blocked, not empty, and forgetting an occluded obstacle is how a
    /// controller drives through it. The TTL is the backstop for rays that
    /// never resolve either way.
    fn expire(&mut self, class: &[u8; MAX_RAYS]) {
        let mut kept = 0;
        for i in 0..self.count {
            let empty = class[self.ray[i]] == 0;
            let fresh = self.tick.wrapping_sub(self.seen[i]) < COLUMN_TTL;
            if !empty && fresh {
                self.caps[kept] = self.caps[i];
                self.seen[kept] = self.seen[i];
                self.ray[kept] = self.ray[i];
                kept += 1;
            }
        }
        self.count = kept;
    }

    /// Refresh a nearby column or append a new one (dropped at capacity).
    fn insert(&mut self, ray: usize, hit: Vec3) {
        for i in 0..self.count {
            let dx = self.caps[i].a[0] - hit[0];
            let dy = self.caps[i].a[1] - hit[1];
            if dx * dx + dy * dy < MERGE_RADIUS * MERGE_RADIUS {
                self.caps[i].a = Vec3::new(hit[0], hit[1], 0.0);
                self.caps[i].b = Vec3::new(hit[0], hit[1], COLUMN_TOP);
                self.seen[i] = self.tick;
                self.ray[i] = ray;
                return;
            }
        }
        if self.count < MAX_OBSTACLES {
            self.caps[self.count] = Capsule {
                a: Vec3::new(hit[0], hit[1], 0.0),
                b: Vec3::new(hit[0], hit[1], COLUMN_TOP),
                radius: HIT_RADIUS,
            };
            self.seen[self.count] = self.tick;
            self.ray[self.count] = ray;
            self.count += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use kinematics::prelude::{Chain, Joint, Quat};

    use super::*;

    /// One joint at the origin, reaching 1 m along +x: the link polyline is
    /// the segment `(0,0,0)..(1,0,0)`.
    const JOINT1: [Joint; 1] = [Joint {
        offset: Vec3::new(0.0, 0.0, 0.0),
        quat: Quat::IDENTITY,
        axis: Vec3([0.0, 0.0, 1.0]),
        range: None,
    }];
    const ARM: Chain<1> = Chain {
        joints: &JOINT1,
        tip_offset: Vec3([1.0, 0.0, 0.0]),
        tip_quat: Quat::IDENTITY,
    };

    /// Fan at the world origin: ray 0 heads +x (down the arm), ray 1 heads +y.
    fn fan(ranges: &[f32]) -> Fan<'_> {
        Fan {
            origin: Vec3::new(0.0, 0.0, 0.0),
            angle_min: 0.0,
            angle_incr: core::f32::consts::FRAC_PI_2,
            range_max: 3.0,
            ranges,
        }
    }

    #[test]
    fn returns_on_the_arm_are_self_hits() {
        let mut p = Perception::new();
        // Ray 0 lands on the arm (0.5, 0, 0); ray 1 sees nothing.
        p.update(&fan(&[0.5, 3.0]), &ARM.links(&[0.0]));
        assert_eq!(p.self_hits(), 1);
        assert!(p.obstacles().is_empty(), "{:?}", p.obstacles());
    }

    #[test]
    fn off_arm_returns_become_columns() {
        let mut p = Perception::new();
        // Ray 1 lands 0.5 m off the arm's axis, on the +y ray.
        p.update(&fan(&[3.0, 0.5]), &ARM.links(&[0.0]));
        assert_eq!(p.self_hits(), 0);
        let caps = p.obstacles();
        assert_eq!(caps.len(), 1, "{caps:?}");
        assert!((caps[0].a[0] - 0.0).abs() < 1.0e-3 && (caps[0].a[1] - 0.5).abs() < 1.0e-3);
        assert!((caps[0].b[2] - COLUMN_TOP).abs() < 1.0e-6);
    }

    #[test]
    fn a_nearby_return_refreshes_instead_of_adding() {
        let mut p = Perception::new();
        p.update(&fan(&[3.0, 0.5]), &ARM.links(&[0.0]));
        p.update(&fan(&[3.0, 0.52]), &ARM.links(&[0.0]));
        assert_eq!(p.obstacles().len(), 1, "close returns merged");
        assert!((p.obstacles()[0].a[1] - 0.52).abs() < 1.0e-3);
    }

    #[test]
    fn occlusion_keeps_a_column_and_an_empty_ray_clears_it() {
        let mut p = Perception::new();
        let links = ARM.links(&[0.0]);

        // Sighted: a column on ray 1.
        p.update(&fan(&[3.0, 0.5]), &links);
        assert_eq!(p.obstacles().len(), 1);

        // The arm swings in and blocks ray 1: a self return, not empty space.
        // The column must survive (forgetting an occluded obstacle is how a
        // controller drives through it).
        p.update(&fan(&[3.0, 0.1]), &links);
        assert_eq!(p.self_hits(), 1);
        assert_eq!(p.obstacles().len(), 1, "column kept while occluded");

        // The ray now looks into empty space: the column is gone for good.
        p.update(&fan(&[3.0, 3.0]), &links);
        assert!(p.obstacles().is_empty(), "{:?}", p.obstacles());
    }
}
