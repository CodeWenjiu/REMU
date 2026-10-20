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

use crate::{ArmSnapshot, MocapCommand, ModelSource, TripleBuffer};

use super::armview::ArmView;
use super::ui::ArmUi;

pub struct ArmWindow {
    /// Shared with the UI's rendering notifier, which draws the scene into
    /// the window before Slint draws the panel.
    view: Rc<RefCell<ArmView>>,
    snapshot: Arc<TripleBuffer<ArmSnapshot>>,
    /// Obstacle positions the UI publishes; the controller applies them.
    mocap: Arc<TripleBuffer<MocapCommand>>,
    /// Slint UI over the same GL context; `None` when Slint's platform is
    /// unavailable, in which case the scene is rendered directly.
    ui: Option<ArmUi>,
    /// Latest window size; updated on resize events.
    size: PhysicalSize<u32>,
    /// Left-button state, tracked by the caller's event loop (camera drag).
    left: bool,
    /// A 3D obstacle drag is in progress (the camera must stay put).
    mocap_drag: bool,
    /// Latest cursor position in physical pixels (for Slint button events,
    /// which carry no position of their own).
    cursor: (f64, f64),
}

impl ArmWindow {
    /// Create the GL stack for `window` and load the mirror model.
    /// Must run on the thread that will render (the event-loop thread).
    pub fn new(
        window: &Window,
        source: &ModelSource,
        snapshot: Arc<TripleBuffer<ArmSnapshot>>,
        mocap: Arc<TripleBuffer<MocapCommand>>,
    ) -> Result<Self, Box<dyn Error>> {
        let size = window.inner_size();
        let view = Rc::new(RefCell::new(ArmView::new(window, source)?));
        let gl = Rc::clone(&view.borrow().gl);
        let ui = match ArmUi::new(gl, Rc::clone(&view), Arc::clone(&mocap), slint_size(size)) {
            Ok(ui) => Some(ui),
            Err(e) => {
                eprintln!("armview: Slint UI unavailable ({e}); rendering the scene directly");
                None
            }
        };
        Ok(Self {
            view,
            snapshot,
            mocap,
            ui,
            size,
            left: false,
            mocap_drag: false,
            cursor: (0.0, 0.0),
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
        mocap: Arc<TripleBuffer<MocapCommand>>,
    ) -> Result<(Rc<Window>, Self), Box<dyn Error>> {
        let window = Rc::new(event_loop.create_window(Self::attributes())?);
        let view = Self::new(&window, source, snapshot, mocap)?;
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
        // A live obstacle drag owns the mocap positions; the snapshot lags it
        // by a commit and would pull the body back.
        let mocap = (!self.mocap_drag).then_some(snap.mocap.as_slice());
        self.view
            .borrow_mut()
            .set_state(&snap.qpos, &snap.qvel, mocap);

        let Some(ui) = self.ui.as_mut() else {
            self.view.borrow_mut().render(self.size);
            return;
        };
        if let Err(e) = ui.render(snap) {
            eprintln!("armview: Slint UI render failed: {e}; rendering the scene directly");
            self.ui = None;
            self.view.borrow_mut().render(self.size);
        }
    }

    /// Track the cursor. A 3D obstacle drag moves the picked mocap body;
    /// otherwise, while the left button is down, horizontal motion orbits and
    /// vertical motion tilts the free camera.
    pub fn cursor_moved(&mut self, x: f64, y: f64) {
        self.cursor = (x, y);
        if let Some(ui) = self.ui.as_ref() {
            ui.pointer_moved(x, y);
        }
        if self.mocap_drag {
            let (_, scene_h) = self.scene_size();
            if let Some(pos) = self.view.borrow_mut().drag_mocap(x, y, scene_h) {
                self.mocap.publish(|c| c.pos.clone_from(&pos));
                if let Some(ui) = self.ui.as_ref() {
                    ui.follow_mocap(&pos);
                }
            }
            return;
        }
        // Keep tracking the cursor over the panel so the camera does not jump
        // when a drag starts there, but do not rotate the camera.
        let over_panel = self
            .ui
            .as_ref()
            .is_some_and(|ui| x >= ui.scene_width() as f64);
        self.view.borrow_mut().cursor_moved(x, y, self.left && !over_panel);
    }

    /// Set the left-button state: a panel control wins, then a mocap obstacle
    /// under the cursor, otherwise the camera arms.
    pub fn set_left(&mut self, down: bool) {
        let taken = self
            .ui
            .as_ref()
            .is_some_and(|ui| ui.pointer_button(down, self.cursor.0, self.cursor.1));
        if !down {
            if self.mocap_drag {
                self.view.borrow_mut().end_mocap_drag();
                self.mocap_drag = false;
            }
            self.left = false;
            return;
        }
        if taken {
            self.left = false;
            return;
        }
        let (scene_w, scene_h) = self.scene_size();
        match self
            .view
            .borrow_mut()
            .begin_mocap_drag(self.cursor.0, self.cursor.1, scene_w, scene_h)
        {
            Some(index) => {
                self.mocap_drag = true;
                self.left = false;
                if let Some(ui) = self.ui.as_ref() {
                    ui.select_mocap(index);
                }
            }
            None => self.left = true,
        }
    }

    /// The cursor left the window: drop Slint's hover/grab state and ignore
    /// the next drag delta (it would be a jump back into the window).
    pub fn cursor_left(&mut self) {
        if let Some(ui) = self.ui.as_ref() {
            ui.pointer_exited();
        }
        if self.mocap_drag {
            self.view.borrow_mut().suspend_mocap_drag();
        }
    }

    /// Zoom the free camera by one scroll event's delta.
    pub fn wheel(&mut self, delta: MouseScrollDelta) {
        if self
            .ui
            .as_ref()
            .is_some_and(|ui| self.cursor.0 >= ui.scene_width() as f64)
        {
            return;
        }
        self.view.borrow_mut().zoom(delta);
    }

    /// The 3D viewport in physical pixels: the window minus the panel.
    fn scene_size(&self) -> (u32, u32) {
        let width = self
            .ui
            .as_ref()
            .map_or(self.size.width, |ui| ui.scene_width());
        (width.max(1), self.size.height.max(1))
    }
}

/// winit's physical size as Slint's.
fn slint_size(size: PhysicalSize<u32>) -> slint::PhysicalSize {
    slint::PhysicalSize::new(size.width, size.height)
}
