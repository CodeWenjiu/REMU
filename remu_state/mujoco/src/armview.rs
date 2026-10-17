//! Windowed front-end for [`MujocoView`]: an EGL context + window surface
//! (glutin) on the caller's thread, drawing one frame per redraw.
//!
//! `ArmView` only owns the GL stack and the view. The winit window, its events,
//! and redraw scheduling stay with the caller — the simulator's window host
//! (remu_state) or the host backend (remu_hal_host); [`ArmWindow`](crate::ArmWindow)
//! wraps this view with that event glue. Drop order matters: this type must be
//! dropped before the window it was created for.

use std::error::Error;
use std::num::NonZeroU32;
use std::os::raw::c_int;

use glutin::config::ConfigTemplateBuilder;
use glutin::context::{
    ContextApi, ContextAttributesBuilder, GlProfile, PossiblyCurrentContext, Version,
};
use glutin::display::{Display, DisplayApiPreference};
use glutin::prelude::*;
use glutin::surface::{Surface, SurfaceAttributesBuilder, SwapInterval, WindowSurface};
use winit::dpi::PhysicalSize;
use winit::event::MouseScrollDelta;
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use winit::window::Window;

use super::ModelSource;
use super::ffi;
use super::view::MujocoView;

/// Engine scroll delta per wheel click: MuJoCo's `simulate` passes 0.02.
const ZOOM_PER_CLICK: f64 = 0.02;

/// `wl_pointer.axis` units (wheel degrees) in one wheel click; winit reports
/// Wayland wheels as pixel deltas of the raw axis value.
const SCROLL_UNITS_PER_CLICK: f64 = 15.0;

pub(crate) struct ArmView {
    // Drop order: engine resources (via the `view` Option in `drop`) first,
    // then the GL objects here, then the caller drops the window.
    context: PossiblyCurrentContext,
    surface: Surface<WindowSurface>,
    /// Backs the context and surface; kept alive for the window's lifetime.
    display: Display,
    view: Option<MujocoView>,
    /// Set once if snapshots stop matching the mirror model (wrong dims).
    state_warned: bool,
    /// Physical window size, used to normalize pointer motion.
    size: PhysicalSize<u32>,
    last_cursor: Option<(f64, f64)>,
}

impl ArmView {
    /// Create the GL stack for `window` and load the mirror model from
    /// `source`. Must run on the thread that will render (the event-loop
    /// thread).
    pub(crate) fn new(window: &Window, source: &ModelSource) -> Result<Self, Box<dyn Error>> {
        let display_handle = window.display_handle()?.as_raw();
        let window_handle = window.window_handle()?.as_raw();

        let display = unsafe { Display::new(display_handle, DisplayApiPreference::Egl)? };
        let template = ConfigTemplateBuilder::new().with_alpha_size(8);
        let config = unsafe { display.find_configs(template.build())? }
            .reduce(|a, b| {
                if a.num_samples() > b.num_samples() {
                    a
                } else {
                    b
                }
            })
            .ok_or("no EGL config supports a window surface")?;
        // MuJoCo's renderer wants the compatibility profile (its glad checks
        // `GL_ARB_framebuffer_object` via the extension string, which core
        // profiles no longer report — it is core functionality there).
        let ctx_attrs = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::OpenGl(Some(Version::new(3, 3))))
            .with_profile(GlProfile::Compatibility)
            .build(Some(window_handle));
        let not_current = unsafe { display.create_context(&config, &ctx_attrs)? };
        let size = window.inner_size();
        let surf_attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
            window_handle,
            NonZeroU32::new(size.width.max(1)).unwrap(),
            NonZeroU32::new(size.height.max(1)).unwrap(),
        );
        let surface = unsafe { display.create_window_surface(&config, &surf_attrs)? };
        let context = not_current.make_current(&surface)?;
        // Best effort: some EGL implementations cannot control swapping.
        let _ =
            surface.set_swap_interval(&context, SwapInterval::Wait(NonZeroU32::new(1).unwrap()));

        let view = MujocoView::load(source)?;
        Ok(Self {
            context,
            surface,
            display,
            view: Some(view),
            state_warned: false,
            size,
            last_cursor: None,
        })
    }

    /// Copy the latest controller snapshot into the mirror (next render shows it).
    pub(crate) fn set_state(&mut self, qpos: &[f64], qvel: &[f64]) {
        if let Some(view) = self.view.as_mut() {
            match view.set_state(qpos, qvel) {
                Ok(()) => self.state_warned = false,
                Err(e) if !self.state_warned => {
                    self.state_warned = true;
                    eprintln!("armview: state rejected: {e}");
                }
                Err(_) => {}
            }
        }
    }

    /// Match the GL surface to a resized window.
    pub(crate) fn resize(&mut self, size: PhysicalSize<u32>) {
        self.size = size;
        self.surface.resize(
            &self.context,
            NonZeroU32::new(size.width.max(1)).unwrap(),
            NonZeroU32::new(size.height.max(1)).unwrap(),
        );
    }

    /// Render one frame at the window's current size and present it.
    pub(crate) fn render(&mut self, size: PhysicalSize<u32>) {
        if let Some(view) = self.view.as_mut() {
            view.render(size.width.max(1) as c_int, size.height.max(1) as c_int);
        }
        let _ = self.surface.swap_buffers(&self.context);
    }

    /// Track the cursor; while `pressed`, horizontal motion orbits and
    /// vertical motion tilts the free camera.
    pub(crate) fn cursor_moved(&mut self, x: f64, y: f64, pressed: bool) {
        let last = self.last_cursor.replace((x, y));
        if !pressed {
            return;
        }
        let Some((px, py)) = last else { return };
        let (dx, dy) = (x - px, y - py);
        // `mjv_moveCamera` takes deltas normalized by the viewport, not pixels.
        let h = self.size.height.max(1) as f64;
        if let Some(view) = self.view.as_mut() {
            if dx != 0.0 {
                view.mouse(ffi::MOUSE_ROTATE_H, dx / h, 0.0);
            }
            if dy != 0.0 {
                view.mouse(ffi::MOUSE_ROTATE_V, 0.0, dy / h);
            }
        }
    }

    /// Zoom the free camera by one scroll event's delta.
    pub(crate) fn zoom(&mut self, delta: MouseScrollDelta) {
        let clicks = match delta {
            MouseScrollDelta::LineDelta(_, y) => y as f64,
            MouseScrollDelta::PixelDelta(p) => p.y / SCROLL_UNITS_PER_CLICK,
        };
        if let Some(view) = self.view.as_mut() {
            view.mouse(ffi::MOUSE_ZOOM, 0.0, ZOOM_PER_CLICK * clicks);
        }
    }
}

impl Drop for ArmView {
    fn drop(&mut self) {
        // Free engine GL resources while the context below is still current;
        // the remaining fields drop after this body in declaration order.
        drop(self.view.take());
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};

    use winit::application::ApplicationHandler;
    use winit::dpi::LogicalSize;
    use winit::event::{ElementState, MouseButton, WindowEvent};
    use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
    use winit::platform::wayland::EventLoopBuilderExtWayland as _;
    use winit::window::{Window, WindowId};

    use crate::{ARM_XML, ModelSource, Session};

    use super::ArmView;

    /// Latest control-thread snapshot (qpos/qvel read out of a session).
    #[derive(Default)]
    struct Snapshot {
        qpos: Vec<f64>,
        qvel: Vec<f64>,
    }

    struct Demo {
        // Drop order: the GL view goes before the window it renders into.
        arm: Option<ArmView>,
        window: Option<Rc<Window>>,
        snap: Arc<Mutex<Snapshot>>,
        left: bool,
    }

    impl ApplicationHandler for Demo {
        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            if self.window.is_some() {
                return;
            }
            let window = Rc::new(
                event_loop
                    .create_window(
                        Window::default_attributes()
                            .with_title("remu armview")
                            .with_inner_size(LogicalSize::new(900.0, 700.0)),
                    )
                    .expect("create window"),
            );
            self.arm = Some(
                ArmView::new(&window, &ModelSource::Xml(ARM_XML.to_string()))
                    .expect("init armview"),
            );
            self.window = Some(window);
        }

        fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
            if self.window.as_ref().map(|w| w.id()) != Some(id) {
                return;
            }
            match event {
                WindowEvent::CloseRequested => event_loop.exit(),
                WindowEvent::Resized(size) => {
                    if let Some(arm) = self.arm.as_mut() {
                        arm.resize(size);
                    }
                }
                WindowEvent::RedrawRequested => {
                    {
                        let snap = self.snap.lock().unwrap();
                        if let Some(arm) = self.arm.as_mut() {
                            arm.set_state(&snap.qpos, &snap.qvel);
                        }
                    }
                    if let (Some(arm), Some(window)) = (self.arm.as_mut(), self.window.as_ref()) {
                        arm.render(window.inner_size());
                    }
                }
                WindowEvent::CursorMoved { position, .. } => {
                    if let Some(arm) = self.arm.as_mut() {
                        arm.cursor_moved(position.x, position.y, self.left);
                    }
                }
                WindowEvent::MouseInput {
                    state,
                    button: MouseButton::Left,
                    ..
                } => self.left = matches!(state, ElementState::Pressed),
                WindowEvent::MouseWheel { delta, .. } => {
                    if let Some(arm) = self.arm.as_mut() {
                        arm.zoom(delta);
                    }
                }
                _ => {}
            }
        }

        fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
            event_loop.set_control_flow(ControlFlow::Poll);
            if let Some(window) = self.window.as_ref() {
                window.request_redraw();
            }
        }
    }

    /// Windowed demo — the 3D successor of `mujoco_arm_demo`:
    /// `cargo test -p remu_state_mujoco armview_demo -- --ignored --nocapture`.
    /// A scripted controller thread drives the mirror in real time; drag to
    /// orbit, wheel to zoom, close the window to end.
    #[test]
    #[ignore = "opens a window; not part of the normal suite"]
    fn armview_demo() {
        let snap = Arc::new(Mutex::new(Snapshot::default()));
        let running = Arc::new(AtomicBool::new(true));

        let control = {
            let snap = Arc::clone(&snap);
            let running = Arc::clone(&running);
            std::thread::spawn(move || {
                let mut s = Session::from_xml_str(ARM_XML).expect("control model");
                let mut q = vec![0.0; s.nq()];
                let mut v = vec![0.0; s.nv()];
                let mut k = 0u64;
                while running.load(Ordering::Relaxed) {
                    let t = k as f64 * s.timestep();
                    let ctrl = [
                        0.7 * (0.6 * t).sin(),
                        0.5 * (0.9 * t).sin(),
                        0.4 * (0.5 * t).cos(),
                    ];
                    s.set_ctrl(&ctrl).unwrap();
                    s.step();
                    s.read_qpos(&mut q).unwrap();
                    s.read_qvel(&mut v).unwrap();
                    let mut g = snap.lock().unwrap();
                    g.qpos.clone_from(&q);
                    g.qvel.clone_from(&v);
                    drop(g);
                    k += 1;
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
            })
        };

        let event_loop = EventLoop::<()>::with_user_event()
            .with_any_thread(true)
            .build()
            .expect("build event loop");
        let mut demo = Demo {
            arm: None,
            window: None,
            snap,
            left: false,
        };
        event_loop.run_app(&mut demo).expect("run event loop");

        running.store(false, Ordering::Relaxed);
        let _ = control.join();
    }
}
