//! Monitor-window glue over [`ArmView`]: event handling and per-frame
//! rendering from the published snapshot. Shared by the simulator's window
//! host (remu_state) and the native host backend (remu_hal_host); the winit
//! window itself stays owned by the caller.

use std::cell::RefCell;
use std::error::Error;
use std::rc::Rc;
use std::sync::Arc;

use winit::dpi::PhysicalSize;
use winit::event::MouseScrollDelta;
use winit::window::Window;

use crate::{ArmSnapshot, ModelSource, TripleBuffer};

use super::armview::ArmView;
use super::ui::ArmUi;

pub struct ArmWindow {
    /// Shared with the UI's rendering notifier, which draws the scene into
    /// the window before Slint draws the panel.
    view: Rc<RefCell<ArmView>>,
    snapshot: Arc<TripleBuffer<ArmSnapshot>>,
    /// Slint UI over the same GL context; `None` when Slint's platform is
    /// unavailable, in which case the scene is rendered directly.
    ui: Option<ArmUi>,
    /// Latest window size; updated on resize events.
    size: PhysicalSize<u32>,
    /// Left-button state, tracked by the caller's event loop (camera drag).
    left: bool,
}

impl ArmWindow {
    /// Create the GL stack for `window` and load the mirror model.
    /// Must run on the thread that will render (the event-loop thread).
    pub fn new(
        window: &Window,
        source: &ModelSource,
        snapshot: Arc<TripleBuffer<ArmSnapshot>>,
    ) -> Result<Self, Box<dyn Error>> {
        let size = window.inner_size();
        let view = Rc::new(RefCell::new(ArmView::new(window, source)?));
        let gl = Rc::clone(&view.borrow().gl);
        let ui = match ArmUi::new(gl, Rc::clone(&view), slint_size(size)) {
            Ok(ui) => Some(ui),
            Err(e) => {
                eprintln!("armview: Slint UI unavailable ({e}); rendering the scene directly");
                None
            }
        };
        Ok(Self {
            view,
            snapshot,
            ui,
            size,
            left: false,
        })
    }

    /// Window attributes for the monitor: the panel plus a fixed 4:3 3D
    /// viewport. Tiling compositors honour `min == max`; without it they would
    /// stretch the window and the viewport would lose its ratio.
    pub fn attributes() -> winit::window::WindowAttributes {
        let size = super::ui::arm_view_window_size();
        winit::window::WindowAttributes::default()
            .with_title("remu armview")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .with_max_inner_size(size)
    }

    /// Create the monitor window and its view on the loop thread.
    pub fn open(
        event_loop: &winit::event_loop::ActiveEventLoop,
        source: &ModelSource,
        snapshot: Arc<TripleBuffer<ArmSnapshot>>,
    ) -> Result<(Rc<Window>, Self), Box<dyn Error>> {
        let window = Rc::new(event_loop.create_window(Self::attributes())?);
        let view = Self::new(&window, source, snapshot)?;
        Ok((window, view))
    }

    /// Match the GL surface to a resized window.
    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.size = size;
        self.view.borrow_mut().resize(size);
        if let Some(ui) = self.ui.as_mut() {
            ui.resize(slint_size(size));
        }
    }

    /// Render one frame from the latest snapshot.
    pub fn redraw(&mut self) {
        let snap = self.snapshot.read();
        self.view.borrow_mut().set_state(&snap.qpos, &snap.qvel);

        let Some(ui) = self.ui.as_mut() else {
            self.view.borrow_mut().render(self.size);
            return;
        };
        if let Err(e) = ui.render() {
            eprintln!("armview: Slint UI render failed: {e}; rendering the scene directly");
            self.ui = None;
            self.view.borrow_mut().render(self.size);
        }
    }

    /// Track the cursor; while the left button is down, horizontal motion
    /// orbits and vertical motion tilts the free camera.
    pub fn cursor_moved(&mut self, x: f64, y: f64) {
        // Keep tracking the cursor over the panel so the camera does not jump
        // when a drag starts there, but do not rotate the camera.
        let over_panel = self
            .ui
            .as_ref()
            .is_some_and(|ui| x >= ui.scene_width() as f64);
        self.view.borrow_mut().cursor_moved(x, y, self.left && !over_panel);
    }

    /// Set the left-button state (camera drag).
    pub fn set_left(&mut self, down: bool) {
        self.left = down;
    }

    /// Zoom the free camera by one scroll event's delta.
    pub fn wheel(&mut self, delta: MouseScrollDelta) {
        self.view.borrow_mut().zoom(delta);
    }
}

/// winit's physical size as Slint's.
fn slint_size(size: PhysicalSize<u32>) -> slint::PhysicalSize {
    slint::PhysicalSize::new(size.width, size.height)
}
