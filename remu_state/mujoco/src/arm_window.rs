//! Monitor-window glue over [`ArmView`]: event handling and per-frame
//! rendering from the published snapshot. Shared by the simulator's window
//! host (remu_state) and the native host backend (remu_hal_host); the winit
//! window itself stays owned by the caller.

use std::error::Error;
use std::sync::{Arc, Mutex};

use winit::dpi::PhysicalSize;
use winit::event::MouseScrollDelta;
use winit::window::Window;

use crate::{ArmSnapshot, ModelSource};

use super::armview::ArmView;

pub struct ArmWindow {
    view: ArmView,
    snapshot: Arc<Mutex<ArmSnapshot>>,
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
        snapshot: Arc<Mutex<ArmSnapshot>>,
    ) -> Result<Self, Box<dyn Error>> {
        let size = window.inner_size();
        let view = ArmView::new(window, source)?;
        Ok(Self {
            view,
            snapshot,
            size,
            left: false,
        })
    }

    /// Match the GL surface to a resized window.
    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.size = size;
        self.view.resize(size);
    }

    /// Render one frame from the latest snapshot.
    pub fn redraw(&mut self) {
        let snap = self.snapshot.lock().unwrap();
        self.view.set_state(&snap.qpos, &snap.qvel);
        drop(snap);
        self.view.render(self.size);
    }

    /// Track the cursor; while the left button is down, horizontal motion
    /// orbits and vertical motion tilts the free camera.
    pub fn cursor_moved(&mut self, x: f64, y: f64) {
        self.view.cursor_moved(x, y, self.left);
    }

    /// Set the left-button state (camera drag).
    pub fn set_left(&mut self, down: bool) {
        self.left = down;
    }

    /// Zoom the free camera by one scroll event's delta.
    pub fn wheel(&mut self, delta: MouseScrollDelta) {
        self.view.zoom(delta);
    }
}
