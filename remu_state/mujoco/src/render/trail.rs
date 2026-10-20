//! End-effector motion trail: fading connectors appended to the scene.

use std::collections::VecDeque;
use std::ffi::CString;
use std::os::raw::c_int;
use std::time::{Duration, Instant};

use crate::engine::ffi::{self, GEOM_LINE, MjvScene, MujocoApi, SAMEFRAME_NONE};

/// End-effector trail: samples within this window of time, thinned by
/// `TRAIL_MIN_STEP` and capped at `TRAIL_CAP` (≈2.5 s at 60 fps).
const TRAIL_TTL: Duration = Duration::from_millis(2500);
const TRAIL_CAP: usize = 160;
const TRAIL_MIN_STEP: f64 = 1.0e-4;

/// Trail look: warm orange lines fading with age, `TRAIL_WIDTH_PX` wide.
const TRAIL_RGB: [f32; 3] = [1.0, 0.45, 0.12];
const TRAIL_ALPHA: f32 = 0.75;
const TRAIL_WIDTH_PX: f64 = 6.0;

/// Recent tip world positions, appended to the scene as fading line
/// connectors (user geoms, `mjv_initGeom`/`mjv_connector` family).
pub(super) struct Trail {
    body: Option<c_int>,
    points: VecDeque<(Instant, [f64; 3])>,
}

impl Trail {
    /// Track the model's end-effector body: menagerie arms name it `hand`,
    /// the built-in toy arm `tip`; without either, no trail is drawn.
    pub(super) fn new(api: &MujocoApi, model: *const ffi::MjModel) -> Self {
        let find = |name: &str| {
            let Ok(name_c) = CString::new(name) else {
                return -1;
            };
            unsafe { api.name2id(model, ffi::OBJ_BODY, name_c.as_ptr()) }
        };
        let body = ["hand", "tip"].into_iter().map(find).find(|&id| id >= 0);
        if body.is_none() {
            eprintln!("armview: no `hand`/`tip` body in the model; trail disabled");
        }
        Self {
            body,
            points: VecDeque::new(),
        }
    }

    pub(super) fn follow(&mut self, api: &MujocoApi, data: *mut ffi::MjData) {
        self.follow_at(api, data, Instant::now());
    }

    fn follow_at(&mut self, api: &MujocoApi, data: *mut ffi::MjData, now: Instant) {
        let Some(body) = self.body else {
            return;
        };
        let local = [0.0f64; 3];
        let mut tip = [0.0f64; 3];
        unsafe {
            api.local2_global(
                data,
                tip.as_mut_ptr(),
                std::ptr::null_mut(),
                local.as_ptr(),
                std::ptr::null(),
                body,
                SAMEFRAME_NONE,
            )
        };
        let moved = match self.points.back() {
            Some((_, last)) => {
                let d = [tip[0] - last[0], tip[1] - last[1], tip[2] - last[2]];
                d[0] * d[0] + d[1] * d[1] + d[2] * d[2] > TRAIL_MIN_STEP * TRAIL_MIN_STEP
            }
            None => true,
        };
        if moved {
            self.points.push_back((now, tip));
        }
        while let Some((t, _)) = self.points.front() {
            if now.duration_since(*t) > TRAIL_TTL {
                self.points.pop_front();
            } else {
                break;
            }
        }
        while self.points.len() > TRAIL_CAP {
            self.points.pop_front();
        }
    }

    /// Append one fading line per consecutive pair, newest first, within
    /// the scene's remaining geom capacity.
    pub(super) fn draw(&mut self, api: &MujocoApi, scene: &mut MjvScene) {
        let points = self.points.make_contiguous();
        let n = points.len();
        if n < 2 {
            return;
        }
        let budget = (scene.maxgeom - scene.ngeom).max(0) as usize;
        for k in 0..(n - 1).min(budget) {
            let idx = n - 1 - k;
            let fresh = idx as f32 / (n - 1) as f32;
            let rgba = [
                TRAIL_RGB[0],
                TRAIL_RGB[1],
                TRAIL_RGB[2],
                0.1 + TRAIL_ALPHA * fresh * fresh,
            ];
            let from = points[idx - 1].1;
            let to = points[idx].1;
            unsafe {
                let slot = scene.ngeom as usize;
                // Keep the distance-order buffer consistent with the appended
                // entries, whatever the renderer does with it.
                *scene.geomorder.add(slot) = scene.ngeom;
                let geom = &mut *scene.geoms.add(slot);
                api.init_geom(
                    geom,
                    GEOM_LINE,
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    rgba.as_ptr(),
                );
                api.connector(geom, GEOM_LINE, TRAIL_WIDTH_PX, from.as_ptr(), to.as_ptr());
                geom.transparent = 1;
            }
            scene.ngeom += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::mem::MaybeUninit;

    use super::*;
    use crate::engine::runtime;
    use crate::{ARM_XML, ModelSource, Session};

    fn make_scene(api: &MujocoApi, session: &Session, maxgeom: c_int) -> Box<MjvScene> {
        let mut scene = Box::new(unsafe { MaybeUninit::<MjvScene>::zeroed().assume_init() });
        unsafe { api.make_scene(session.model_ptr(), &mut *scene, maxgeom) };
        scene
    }

    #[test]
    fn tip_body_resolved_and_tracked() {
        let api = runtime::api().unwrap();
        let mut session = Session::from_xml_str(ARM_XML).unwrap();
        let mut trail = Trail::new(api, session.model_ptr());
        assert!(trail.body.is_some(), "built-in arm declares a `tip` body");

        session.forward();
        let t0 = Instant::now();
        trail.follow_at(api, session.data_ptr(), t0);
        assert_eq!(trail.points.len(), 1);
        let tip = trail.points.back().unwrap().1;
        // Zero pose: straight out along +x from link0 at (0, 0, 0.1).
        assert!(
            (tip[0] - 0.75).abs() < 1e-9 && tip[1].abs() < 1e-9 && (tip[2] - 0.1).abs() < 1e-9,
            "{tip:?}"
        );

        // Rested samples are thinned out at the same position.
        trail.follow_at(api, session.data_ptr(), t0 + Duration::from_millis(16));
        assert_eq!(trail.points.len(), 1);

        // Joint 0 (about +y) swings the tip toward -z.
        session.set_qpos(&[0.3, 0.0, 0.0]).unwrap();
        session.forward();
        trail.follow_at(api, session.data_ptr(), t0 + Duration::from_millis(32));
        assert_eq!(trail.points.len(), 2);
        let tip = trail.points.back().unwrap().1;
        let (s, c) = (0.3f64.sin(), 0.3f64.cos());
        assert!(
            (tip[0] - 0.75 * c).abs() < 1e-9 && (tip[2] - (0.1 - 0.75 * s)).abs() < 1e-9,
            "{tip:?}"
        );

        // Samples older than the window leave even while the arm rests.
        trail.follow_at(
            api,
            session.data_ptr(),
            t0 + TRAIL_TTL + Duration::from_millis(1),
        );
        assert_eq!(trail.points.len(), 1);
    }

    #[test]
    fn trail_connectors_fade_and_shape() {
        let api = runtime::api().unwrap();
        let session = Session::from_xml_str(ARM_XML).unwrap();
        let mut scene = make_scene(api, &session, 64);
        let mut trail = Trail {
            body: None,
            points: VecDeque::new(),
        };
        let t0 = Instant::now();
        for (i, x) in [0.0f64, 0.1, 0.2].into_iter().enumerate() {
            trail
                .points
                .push_back((t0 + Duration::from_millis(i as u64 * 16), [x, 0.0, 0.0]));
        }
        trail.draw(api, &mut scene);
        assert_eq!(scene.ngeom, 2);

        // Newest pair is appended first.
        let g0 = unsafe { &*scene.geoms };
        let g1 = unsafe { &*scene.geoms.add(1) };
        assert_eq!(g0.type_, GEOM_LINE);
        assert_eq!((g0.transparent, g1.transparent), (1, 1));
        assert!(g1.rgba[3] < g0.rgba[3], "{} < {}", g1.rgba[3], g0.rgba[3]);
        // Newest connector: starts at `from`, local z along +x, pixel width.
        assert!(
            (g0.pos[0] - 0.1).abs() < 1e-6 && g0.pos[1].abs() < 1e-6 && g0.pos[2].abs() < 1e-6,
            "{:?}",
            g0.pos
        );
        assert!(
            (g0.size[0] as f64 - TRAIL_WIDTH_PX).abs() < 1e-6,
            "{:?}",
            g0.size
        );
        assert!(
            (g0.size[1] as f64 - TRAIL_WIDTH_PX).abs() < 1e-6,
            "{:?}",
            g0.size
        );
        // Connector convention: width in size[0]/size[1], length in size[2].
        assert!((g0.size[2] - 0.1).abs() < 1e-6, "{:?}", g0.size);
        assert!(
            (g0.mat[2] - 1.0).abs() < 1e-6 && g0.mat[5].abs() < 1e-6 && g0.mat[8].abs() < 1e-6,
            "{:?}",
            g0.mat
        );
        // Lines are not centered around the pair: the older one starts at 0.
        assert!(g1.pos[0].abs() < 1e-6, "{:?}", g1.pos);

        // A full scene takes no more geoms.
        scene.ngeom = scene.maxgeom;
        trail.draw(api, &mut scene);
        assert_eq!(scene.ngeom, scene.maxgeom);
    }

    #[test]
    #[ignore = "needs the menagerie checkout (MUJOCO_MENAGERIE_DIR)"]
    fn panda_trail_body_is_the_hand() {
        let dir = std::env::var("MUJOCO_MENAGERIE_DIR").expect("MUJOCO_MENAGERIE_DIR");
        let path = std::path::Path::new(&dir).join("franka_emika_panda/scene.xml");
        let session = ModelSource::File(path).load().unwrap();
        let api = runtime::api().unwrap();
        let trail = Trail::new(api, session.model_ptr());
        assert!(trail.body.is_some());
    }
}
