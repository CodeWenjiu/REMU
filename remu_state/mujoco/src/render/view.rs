//! `MujocoView`: the arm drawn by the engine's own `mjv`/`mjr` renderer.
//!
//! One mirror [`Session`] plus an abstract scene and an `mjrContext`, rendered
//! straight into the current default framebuffer. The caller owns the OpenGL
//! context and must keep it current on this thread for every call — `mjr`
//! uploads textures and draws during `load`, `render`, and `Drop`.

use std::ffi::{CStr, CString};
use std::mem::MaybeUninit;
use std::os::raw::c_int;

use dlopen2::wrapper::Container;

use super::trail::Trail;
use crate::MujocoError;
use crate::engine::ffi::{
    self, CAT_ALL, FB_WINDOW, FONT_SHADOW, FONTSCALE_150, GRID_TOPLEFT, MjrContext, MjrRect,
    MjvCamera, MjvOption, MjvPerturb, MjvScene, MujocoApi, OBJ_BODY, PERT_TRANSLATE,
};
use crate::engine::runtime;
use crate::engine::{ModelSource, Session};

/// Geom capacity of the render scene.
const MAX_GEOM: c_int = 1000;


pub(crate) struct MujocoView {
    api: &'static Container<MujocoApi>,
    session: Session,
    cam: MjvCamera,
    opt: MjvOption,
    scene: Box<MjvScene>,
    con: Box<MjrContext>,
    /// Scratch buffer for the overlay's qpos readout.
    qpos: Vec<f64>,
    /// End-effector motion trail.
    trail: Trail,
    /// Picked body + drag reference while a mocap obstacle is dragged.
    pert: MjvPerturb,
    /// Last cursor position during a mocap drag (`None` when not dragging).
    drag_last: Option<(f64, f64)>,
    /// Whether `scene` has a camera frustum yet (picking needs it).
    scene_ready: bool,
}

impl MujocoView {
    /// Load `source` as a mirror model and initialize the renderer.
    /// Requires a current GL context on this thread.
    pub(crate) fn load(source: &ModelSource) -> Result<Self, MujocoError> {
        let api = runtime::api()?;
        let mut session = source.load()?;
        let model = session.model_ptr();

        let mut cam = unsafe { MaybeUninit::<MjvCamera>::zeroed().assume_init() };
        unsafe { api.default_free_camera(model, &mut cam) };
        if source.is_builtin() {
            // The built-in toy arm has no `<visual><global>` hints: frame its
            // planar reach volume, seen from the front and slightly above.
            cam.lookat = [0.3, 0.0, 0.15];
            cam.distance = 2.2;
            cam.azimuth = -70.0;
            cam.elevation = -15.0;
        }
        let mut opt = unsafe { MaybeUninit::<MjvOption>::zeroed().assume_init() };
        unsafe { api.default_option(&mut opt) };

        let mut scene = Box::new(unsafe { MaybeUninit::<MjvScene>::zeroed().assume_init() });
        unsafe { api.make_scene(model, &mut *scene, MAX_GEOM) };
        scene.flags[ffi::RND_SKYBOX] = 1;
        scene.flags[ffi::RND_SHADOW] = 1;
        scene.flags[ffi::RND_REFLECTION] = 1;

        let mut con = Box::new(unsafe { MaybeUninit::<MjrContext>::zeroed().assume_init() });
        unsafe { api.make_context(model, &mut *con, FONTSCALE_150) };
        unsafe { api.set_buffer(FB_WINDOW, &mut *con) };

        // Derived state for the first frame before any snapshot arrives.
        session.forward();
        let trail = Trail::new(api, session.model_ptr());
        let qpos = vec![0.0; session.nq()];
        Ok(Self {
            api,
            session,
            cam,
            opt,
            scene,
            con,
            qpos,
            trail,
            pert: MjvPerturb::default(),
            drag_last: None,
            scene_ready: false,
        })
    }

    /// Copy a controller snapshot into the mirror model (shown on next render).
    /// `mocap` is skipped while the UI is dragging an obstacle: the monitor
    /// owns the live position then, and the snapshot would pull it back.
    pub(crate) fn set_state(
        &mut self,
        qpos: &[f64],
        qvel: &[f64],
        mocap: Option<&[f64]>,
    ) -> Result<(), MujocoError> {
        self.session.set_qpos(qpos)?;
        self.session.set_qvel(qvel)?;
        if let Some(mocap) = mocap {
            self.session.set_mocap(mocap)?;
        }
        Ok(())
    }

    /// Move the free camera: `action` is an `mjMOUSE_*` code, deltas
    /// normalized by the viewport size (as `mjv_moveCamera` expects).
    pub(crate) fn mouse(&mut self, action: c_int, dx: f64, dy: f64) {
        unsafe {
            self.api
                .move_camera(self.session.model_ptr(), action, dx, dy, &mut self.cam)
        };
    }

    /// Pick a mocap obstacle under the cursor (`x`, `y` in window pixels,
    /// top-left origin). Returns the mocap index whose drag started.
    pub(crate) fn begin_mocap_drag(
        &mut self,
        x: f64,
        y: f64,
        scene_w: u32,
        scene_h: u32,
    ) -> Option<usize> {
        let w = scene_w.max(1) as f64;
        let h = scene_h.max(1) as f64;
        if !self.scene_ready || !(0.0..w).contains(&x) || !(0.0..h).contains(&y) {
            return None;
        }
        let relx = x / w;
        let rely = 1.0 - y / h;
        let mut selpnt = [0.0; 3];
        let (mut geomid, mut flexid, mut skinid) = (-1, -1, -1);
        let body = unsafe {
            self.api.select(
                self.session.model_ptr(),
                self.session.data_ptr(),
                &self.opt,
                w / h,
                relx,
                rely,
                &*self.scene,
                selpnt.as_mut_ptr(),
                &mut geomid,
                &mut flexid,
                &mut skinid,
            )
        };
        if body <= 0 || !self.is_mocap_obstacle(body) {
            return None;
        }
        self.pert = MjvPerturb {
            select: body,
            ..Default::default()
        };
        unsafe {
            self.api.init_perturb(
                self.session.model_ptr(),
                self.session.data_ptr(),
                &*self.scene,
                &mut self.pert,
            );
        }
        self.pert.active = PERT_TRANSLATE;

        // `mjv_initPerturb` copies the body's inertial-frame pose into
        // `refpos`; its x/y are the mocap body's x/y, so they identify the
        // picked obstacle (the z may be offset by the geom's inertial pose).
        let n = self.session.mocap_count();
        let mut pos = vec![0.0; 3 * n];
        if self.session.read_mocap(&mut pos).is_err() {
            self.end_mocap_drag();
            return None;
        }
        let index = pos.as_chunks::<3>().0.iter().position(|p| {
            (p[0] - self.pert.refpos[0]).abs() < 1.0e-6
                && (p[1] - self.pert.refpos[1]).abs() < 1.0e-6
        });
        if index.is_none() {
            self.end_mocap_drag();
            return None;
        }
        self.drag_last = Some((x, y));
        index
    }

    /// Move the picked obstacle with the cursor; returns the new mocap
    /// positions (flat `x y z`) to publish to the controller.
    pub(crate) fn drag_mocap(&mut self, x: f64, y: f64, scene_h: u32) -> Option<Vec<f64>> {
        let last = self.drag_last.replace((x, y));
        let (last_x, last_y) = last?;
        let h = scene_h.max(1) as f64;
        let (dx, dy) = ((x - last_x) / h, (y - last_y) / h);
        if dx == 0.0 && dy == 0.0 {
            return None;
        }
        let n = self.session.mocap_count();
        if n == 0 {
            return None;
        }
        let mut before = vec![0.0; 3 * n];
        self.session.read_mocap(&mut before).ok()?;

        unsafe {
            self.api.move_perturb(
                self.session.model_ptr(),
                self.session.data_ptr(),
                ffi::MOUSE_MOVE_H,
                dx,
                dy,
                &*self.scene,
                &mut self.pert,
            );
            self.api.apply_perturb_pose(
                self.session.model_ptr(),
                self.session.data_ptr(),
                &self.pert,
                0,
            );
        }

        let mut after = vec![0.0; 3 * n];
        self.session.read_mocap(&mut after).ok()?;
        // Camera-horizontal motion also has a vertical component; keep each
        // moved obstacle on the plane it started on.
        for i in 0..n {
            let c = 3 * i;
            if before[c..c + 3] != after[c..c + 3] {
                // `refpos` is the inertial frame; shift it by the same z we
                // remove from the body's X-frame position.
                self.pert.refpos[2] += before[c + 2] - after[c + 2];
                after[c + 2] = before[c + 2];
            }
        }
        self.session.set_mocap(&after).ok()?;
        Some(after)
    }

    /// End a mocap drag (the button was released).
    pub(crate) fn end_mocap_drag(&mut self) {
        self.pert.active = 0;
        self.pert.select = 0;
        self.drag_last = None;
    }

    /// Drop the next move delta (the pointer left the window mid-drag).
    pub(crate) fn suspend_mocap_drag(&mut self) {
        self.drag_last = None;
    }

    /// Whether `body` is one of the model's named mocap obstacles.
    fn is_mocap_obstacle(&self, body: c_int) -> bool {
        let name = unsafe {
            self.api
                .id2name(self.session.model_ptr(), OBJ_BODY, body)
        };
        if name.is_null() {
            return false;
        }
        // Safety: the model owns the name for its lifetime.
        unsafe { CStr::from_ptr(name) }
            .to_str()
            .is_ok_and(|name| name.starts_with("obstacle:"))
    }

    /// Render one frame into the current default framebuffer.
    pub(crate) fn render(&mut self, width: c_int, height: c_int) {
        // qpos/qvel may have changed: refresh derived state, rebuild the
        // abstract scene from it, draw, then stamp the overlay.
        self.session.forward();
        let rect = MjrRect {
            left: 0,
            bottom: 0,
            width,
            height,
        };
        unsafe {
            self.api.update_scene(
                self.session.model_ptr(),
                self.session.data_ptr(),
                &self.opt,
                std::ptr::null(),
                &mut self.cam,
                CAT_ALL,
                &mut *self.scene,
            );
        }
        self.scene_ready = true;
        self.trail.follow(self.api, self.session.data_ptr());
        self.trail.draw(self.api, &mut self.scene);
        unsafe {
            self.api.render(rect, &mut *self.scene, &*self.con);
        }
        self.overlay(rect);
    }

    fn overlay(&mut self, rect: MjrRect) {
        let _ = self.session.read_qpos(&mut self.qpos);
        let q = self
            .qpos
            .iter()
            .map(|v| format!("{v:5.2}"))
            .collect::<Vec<_>>()
            .join(" ");
        let line1 = CString::new("remu armview - drag: orbit  wheel: zoom").unwrap();
        let line2 = CString::new(format!("q=[{q}]")).unwrap();
        unsafe {
            self.api.overlay(
                FONT_SHADOW,
                GRID_TOPLEFT,
                rect,
                line1.as_ptr(),
                line2.as_ptr(),
                &*self.con,
            )
        };
    }

    /// Render the current scene into the offscreen buffer and save it as a
    /// BMP — used by the windowed demo to inspect trail pixels.
    #[cfg(test)]
    pub(crate) fn save_offscreen_bmp(
        &mut self,
        path: &std::path::Path,
        width: c_int,
        height: c_int,
    ) {
        let Some(shot) = super::shot::load() else {
            eprintln!("armview: cannot dlopen the engine for the offscreen shot");
            return;
        };
        let rect = MjrRect {
            left: 0,
            bottom: 0,
            width,
            height,
        };
        let mut rgb = vec![0u8; width as usize * height as usize * 3];
        unsafe {
            shot.resize_offscreen(width, height, &mut *self.con);
            self.api.set_buffer(ffi::FB_OFFSCREEN, &mut *self.con);
            self.api.render(rect, &mut *self.scene, &*self.con);
            shot.read_pixels(rgb.as_mut_ptr(), std::ptr::null_mut(), rect, &*self.con);
            self.api.set_buffer(FB_WINDOW, &mut *self.con);
        }
        match super::shot::write_bmp(path, width as usize, height as usize, &rgb) {
            Ok(()) => eprintln!("armview: offscreen shot saved to {}", path.display()),
            Err(e) => eprintln!("armview: offscreen shot failed: {e}"),
        }
    }
}

impl Drop for MujocoView {
    fn drop(&mut self) {
        // Both calls release GL objects; the contract is a current context.
        unsafe { self.api.free_context(&mut *self.con) };
        unsafe { self.api.free_scene(&mut *self.scene) };
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::Session;

    fn make_scene(api: &MujocoApi, session: &Session, maxgeom: c_int) -> Box<MjvScene> {
        let mut scene = Box::new(unsafe { MaybeUninit::<MjvScene>::zeroed().assume_init() });
        unsafe { api.make_scene(session.model_ptr(), &mut *scene, maxgeom) };
        scene
    }

    #[test]
    fn geom_struct_matches_pinned_layout() {
        assert_eq!(std::mem::size_of::<ffi::MjvGeom>(), 252);
    }


    #[test]
    fn select_and_perturb_move_a_mocap_obstacle() {
        const XML: &str = "<mujoco model=\"pick\"><worldbody>\
            <body name=\"obstacle:pole\" mocap=\"true\" pos=\"1 0 0\">\
            <geom name=\"obstacle:pole\" type=\"capsule\" fromto=\"0 0 0 0 0 0.8\" size=\"0.05\"/>\
            </body></worldbody></mujoco>";
        let api = runtime::api().unwrap();
        let mut session = Session::from_xml_str(XML).unwrap();
        session.forward();

        let mut cam = unsafe { MaybeUninit::<MjvCamera>::zeroed().assume_init() };
        unsafe { api.default_free_camera(session.model_ptr(), &mut cam) };
        cam.lookat = [1.0, 0.0, 0.4];
        cam.distance = 2.0;
        cam.azimuth = 90.0;
        cam.elevation = -10.0;
        let mut opt = unsafe { MaybeUninit::<MjvOption>::zeroed().assume_init() };
        unsafe { api.default_option(&mut opt) };
        let mut scene = make_scene(api, &session, MAX_GEOM);
        unsafe {
            api.update_scene(
                session.model_ptr(),
                session.data_ptr(),
                &opt,
                std::ptr::null(),
                &mut cam,
                CAT_ALL,
                &mut *scene,
            )
        };

        let mut selpnt = [0.0; 3];
        let (mut geomid, mut flexid, mut skinid) = (-1, -1, -1);
        let body = unsafe {
            api.select(
                session.model_ptr(),
                session.data_ptr(),
                &opt,
                1.0,
                0.5,
                0.5,
                &*scene,
                selpnt.as_mut_ptr(),
                &mut geomid,
                &mut flexid,
                &mut skinid,
            )
        };
        assert!(body > 0, "body = {body}");
        let name = unsafe { CStr::from_ptr(api.id2name(session.model_ptr(), OBJ_BODY, body)) };
        assert_eq!(name.to_str().unwrap(), "obstacle:pole");

        let mut pert = MjvPerturb {
            select: body,
            ..Default::default()
        };
        unsafe {
            api.init_perturb(
                session.model_ptr(),
                session.data_ptr(),
                &*scene,
                &mut pert,
            )
        };
        pert.active = PERT_TRANSLATE;
        let mut before = [0.0; 3];
        session.read_mocap(&mut before).unwrap();
        assert!(
            (pert.refpos[0] - before[0]).abs() < 1.0e-9
                && (pert.refpos[1] - before[1]).abs() < 1.0e-9,
            "refpos {:?} does not identify mocap {before:?}",
            pert.refpos
        );
        unsafe {
            api.move_perturb(
                session.model_ptr(),
                session.data_ptr(),
                ffi::MOUSE_MOVE_H,
                0.2,
                0.0,
                &*scene,
                &mut pert,
            );
            api.apply_perturb_pose(session.model_ptr(), session.data_ptr(), &pert, 0);
        }
        let mut after = [0.0; 3];
        session.read_mocap(&mut after).unwrap();
        let moved: f64 = before
            .iter()
            .zip(after)
            .map(|(a, b)| (a - b) * (a - b))
            .sum();
        assert!(moved > 1.0e-4, "{before:?} -> {after:?}");

        unsafe { api.free_scene(&mut *scene) };
    }
}
