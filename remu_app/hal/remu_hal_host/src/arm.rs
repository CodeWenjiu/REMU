//! Arm device on host: the actuator register surface over the shared world
//! (`world`), without the remu bus.
//!
//! Same behavior surface as the bus device (remu_state): actuator targets,
//! commit, and state readback, with the monitor window opened when the world
//! loads and closed-window detection via [`armview_alive`]. The model comes
//! from the same `REMU_ARM_MODEL` environment variable (defaults to the
//! built-in toy arm), so host and remu runs show the same thing.

use std::sync::Arc;

use remu_state_mujoco::{ArmSnapshot, ArmWindow, ModelSource, TripleBuffer};

use crate::window::{AuxRequest, AuxWindow, request_aux};
use crate::world;

/// Register-map capacities of the remu device; kept so app-side bounds work
/// identically on both platforms.
pub const ARM_TARGETS_MAX: usize = 8;
/// Register-map capacity of the position block.
pub const ARM_QPOS_MAX: usize = 10;

/// Open the arm monitor window on the shared event loop, publishing into
/// `snapshot`.
pub(crate) fn request_monitor(source: ModelSource, snapshot: Arc<TripleBuffer<ArmSnapshot>>) {
    request_aux(AuxRequest::new(move |event_loop| {
        let (window, view) = ArmWindow::open(event_loop, &source, snapshot)?;
        Ok((window, Box::new(view)))
    }));
}

impl AuxWindow for ArmWindow {
    fn redraw(&mut self) {
        ArmWindow::redraw(self);
    }

    fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        ArmWindow::resize(self, size);
    }

    fn cursor_moved(&mut self, x: f64, y: f64) {
        ArmWindow::cursor_moved(self, x, y);
    }

    fn set_left(&mut self, down: bool) {
        ArmWindow::set_left(self, down);
    }

    fn wheel(&mut self, delta: winit::event::MouseScrollDelta) {
        ArmWindow::wheel(self, delta);
    }
}

/// Actuator target count of the loaded model (joints, plus one gripper
/// channel for models that have one).
pub fn read_nu() -> u32 {
    world::with_core(|c| c.nu() as u32)
}

/// Joint position count of the loaded model.
pub fn read_nq() -> u32 {
    world::with_core(|c| c.nq() as u32)
}

/// Set one actuator target; applied by the next [`commit`]. For joint servos
/// the value is the joint angle (radians); a gripper channel uses the model's
/// own range (e.g. 0..255, 255 = open, for a Panda).
pub fn set_target(channel: usize, value: f32) {
    world::with_core(|c| {
        if channel < c.nu() {
            c.set_target(channel, value);
        }
    });
}

/// Apply the targets and advance the simulation, publishing a new frame to
/// the monitor window.
pub fn commit() {
    world::with_core(|c| c.commit());
}

/// Set the physics microsteps per commit (clamped to ≥ 1; the default of 16
/// uses as much simulated time as possible per 60 Hz frame at a 1 ms
/// timestep — read [`read_timestep`] to scale for other models).
pub fn set_steps(steps: u32) {
    world::with_core(|c| c.set_steps(steps));
}

/// Read a joint position (radians) as of the last commit.
pub fn read_qpos(joint: usize) -> f32 {
    world::with_core(|c| {
        if joint < c.nq() {
            c.read_qpos_f32(joint)
        } else {
            0.0
        }
    })
}

/// Simulated time (seconds) as of the last commit.
pub fn read_time() -> f32 {
    world::with_core(|c| c.time() as f32)
}

/// Number of completed commits (wrapping u32).
pub fn read_commits() -> u32 {
    world::with_core(|c| c.commits())
}

/// Model timestep (seconds); pick `steps` so one commit covers ~16 ms.
pub fn read_timestep() -> f32 {
    world::with_core(|c| c.timestep() as f32)
}

/// Whether the armview monitor window is alive.
///
/// Returns `false` once the window is closed (or if it could not be opened),
/// so demo controllers can stop and exit instead of controlling with no
/// visible monitor — the arm-device counterpart of `display_alive`. Pending
/// (the window is still being set up) counts as alive, so an app polling at
/// startup cannot exit before the window had a chance to open.
pub fn armview_alive() -> bool {
    // Ensure the simulation is up and the window requested before asking.
    world::with_core(|_| ());
    crate::window::aux_alive()
}
