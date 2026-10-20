//! Planar laser-fan sensor model: a parameterized ray fan over the shared
//! session, recast lazily — one frame per committed step, visible to both the
//! remu scan device and the host backend.

use super::arm_core::ArmCore;

/// Fan geometry and limits.
#[derive(Clone, Copy, Debug)]
pub struct ScanParams {
    /// Fan origin in the world frame (the sensor mount).
    pub origin: [f64; 3],
    /// First ray's yaw about world Z (radians).
    pub angle_min: f32,
    /// Yaw step per ray (radians).
    pub angle_incr: f32,
    /// Range limit (metres); "no return" reports this.
    pub range_max: f32,
    /// Rays per frame.
    pub n_rays: usize,
}

impl Default for ScanParams {
    /// The demo installation: a full circle around a tripod beside the arm
    /// (a model-declared `scanner` site is future work).
    fn default() -> Self {
        Self {
            origin: [-0.6, -0.6, 0.5],
            angle_min: -core::f32::consts::PI,
            angle_incr: core::f32::consts::TAU / 64.0,
            range_max: 5.0,
            n_rays: 64,
        }
    }
}

/// A sensor instance: fixed params plus the latest frame, recast lazily when
/// the observed world moved past it (frame vs. commit stays causally tied).
pub struct Scanner {
    params: ScanParams,
    ranges: Vec<f32>,
    /// Commit count of the cached frame; `None` before the first refresh.
    frame: Option<u32>,
}

impl Scanner {
    pub fn new(params: ScanParams) -> Self {
        Self {
            ranges: vec![params.range_max; params.n_rays],
            params,
            frame: None,
        }
    }

    /// Recast when `core` advanced past the cached frame; a no-op otherwise.
    pub fn refresh(&mut self, core: &mut ArmCore) {
        let commits = core.commits();
        if self.frame == Some(commits) {
            return;
        }
        let mut fan = vec![0.0; self.params.n_rays];
        core.cast_fan(
            self.params.origin,
            self.params.angle_min as f64,
            self.params.angle_incr as f64,
            self.params.range_max as f64,
            &mut fan,
        );
        for (r, f) in self.ranges.iter_mut().zip(fan) {
            *r = if f.is_finite() {
                f
            } else {
                self.params.range_max
            };
        }
        self.frame = Some(commits);
    }

    pub fn params(&self) -> &ScanParams {
        &self.params
    }

    pub fn ranges(&self) -> &[f32] {
        &self.ranges
    }

    /// Commit count of the cached frame (0 before the first refresh).
    pub fn seq(&self) -> u32 {
        self.frame.unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use crate::ModelSource;

    use super::*;

    /// A world with one vertical capsule obstacle on the fan's +x ray.
    const POLE_XML: &str = "<mujoco model=\"fan\">\
        <worldbody>\
        <geom name=\"obstacle:pole\" type=\"capsule\" fromto=\"1 -0.6 0 1 -0.6 0.5\" size=\"0.05\"/>\
        </worldbody>\
        </mujoco>";

    #[test]
    fn frame_tracks_commits_and_finds_the_pole() {
        let mut core =
            ArmCore::load(&ModelSource::Xml(POLE_XML.to_string())).expect("load mini model");
        // Scan the pole's midsection (the default plane is level with its top).
        let params = ScanParams {
            origin: [-0.6, -0.6, 0.25],
            ..ScanParams::default()
        };
        let mut scanner = Scanner::new(params);
        assert_eq!(scanner.seq(), 0);

        scanner.refresh(&mut core);
        assert_eq!(scanner.seq(), core.commits());
        // The pole sits on the fan's +x ray, 1.6 m from the tripod origin;
        // the fan must return it somewhere in the (0, range_max] band.
        assert!(
            scanner.ranges().iter().any(|r| *r < 2.0),
            "{:?}",
            scanner.ranges()
        );

        // A commit moves the world: the cached frame is stale until refresh.
        core.commit();
        assert_ne!(scanner.seq(), core.commits());
        scanner.refresh(&mut core);
        assert_eq!(scanner.seq(), core.commits());
    }
}
