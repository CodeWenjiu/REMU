//! `MujocoView`: the arm drawn by the engine's own `mjv`/`mjr` renderer.
//!
//! One mirror [`Session`] plus an abstract scene and an `mjrContext`, rendered
//! straight into the current default framebuffer. The caller owns the OpenGL
//! context and must keep it current on this thread for every call — `mjr`
//! uploads textures and draws during `load`, `render`, and `Drop`.

use std::ffi::CString;
use std::mem::MaybeUninit;
use std::os::raw::c_int;

use dlopen2::wrapper::Container;

use super::ModelSource;
use super::error::MujocoError;
use super::ffi::{
    self, CAT_ALL, FB_WINDOW, FONT_SHADOW, FONTSCALE_150, GRID_TOPLEFT, MjrContext, MjrRect,
    MjvCamera, MjvOption, MjvScene, MujocoApi,
};
use super::runtime;
use super::session::Session;

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
        let qpos = vec![0.0; session.nq()];
        Ok(Self {
            api,
            session,
            cam,
            opt,
            scene,
            con,
            qpos,
        })
    }

    /// Copy a controller snapshot into the mirror model (shown on next render).
    pub(crate) fn set_state(&mut self, qpos: &[f64], qvel: &[f64]) -> Result<(), MujocoError> {
        self.session.set_qpos(qpos)?;
        self.session.set_qvel(qvel)
    }

    /// Move the free camera: `action` is an `mjMOUSE_*` code, deltas
    /// normalized by the viewport size (as `mjv_moveCamera` expects).
    pub(crate) fn mouse(&mut self, action: c_int, dx: f64, dy: f64) {
        unsafe {
            self.api
                .move_camera(self.session.model_ptr(), action, dx, dy, &mut self.cam)
        };
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
}

impl Drop for MujocoView {
    fn drop(&mut self) {
        // Both calls release GL objects; the contract is a current context.
        unsafe { self.api.free_context(&mut *self.con) };
        unsafe { self.api.free_scene(&mut *self.scene) };
    }
}
