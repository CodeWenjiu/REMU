//! Arm device on host: the MuJoCo simulation + armview monitor running
//! natively, without the remu bus.
//!
//! Same behavior surface as the bus device (remu_state): actuator targets,
//! commit, and state readback, with the monitor window opened on first use
//! and closed-window detection via [`armview_alive`]. The model comes from
//! the same `REMU_ARM_MODEL` environment variable (defaults to the built-in
//! toy arm), so host and remu runs show the same thing.

use std::cell::RefCell;

use remu_state_mujoco::{ARM_XML, ArmCore, ModelSource};

/// Register-map capacities of the remu device; kept so app-side bounds work
/// identically on both platforms.
pub const ARM_TARGETS_MAX: usize = 8;
/// Register-map capacity of the position block.
pub const ARM_QPOS_MAX: usize = 10;

thread_local! {
    /// The native arm simulation, created on first use (thread-confined like
    /// the engine itself; guest apps are single-threaded).
    static ARM: RefCell<Option<ArmCore>> = const { RefCell::new(None) };
}

/// Run `f` with the arm core, loading the model and requesting the monitor
/// window on first use.
fn with_arm<R>(f: impl FnOnce(&mut ArmCore) -> R) -> R {
    ARM.with(|slot| {
        let mut slot = slot.borrow_mut();
        let core = slot.get_or_insert_with(|| {
            let source = ModelSource::from_env(ARM_XML);
            let core = ArmCore::load(&source).expect("arm: load model via libmujoco");
            crate::window::request_arm_view(source, core.snapshot());
            core
        });
        f(core)
    })
}

/// Actuator target count of the loaded model (joints, plus one gripper
/// channel for models that have one).
pub fn read_nu() -> u32 {
    with_arm(|c| c.nu() as u32)
}

/// Joint position count of the loaded model.
pub fn read_nq() -> u32 {
    with_arm(|c| c.nq() as u32)
}

/// Set one actuator target; applied by the next [`commit`]. For joint servos
/// the value is the joint angle (radians); a gripper channel uses the model's
/// own range (e.g. 0..255, 255 = open, for a Panda).
pub fn set_target(channel: usize, value: f32) {
    with_arm(|c| {
        if channel < c.nu() {
            c.set_target(channel, value);
        }
    });
}

/// Apply the targets and advance the simulation, publishing a new frame to
/// the monitor window.
pub fn commit() {
    with_arm(|c| c.commit());
}

/// Set the physics microsteps per commit (clamped to ≥ 1; the default of 16
/// uses as much simulated time as possible per 60 Hz frame at a 1 ms
/// timestep — read [`read_timestep`] to scale for other models).
pub fn set_steps(steps: u32) {
    with_arm(|c| c.set_steps(steps));
}

/// Read a joint position (radians) as of the last commit.
pub fn read_qpos(joint: usize) -> f32 {
    with_arm(|c| {
        if joint < c.nq() {
            c.read_qpos_f32(joint)
        } else {
            0.0
        }
    })
}

/// Simulated time (seconds) as of the last commit.
pub fn read_time() -> f32 {
    with_arm(|c| c.time() as f32)
}

/// Number of completed commits (wrapping u32).
pub fn read_commits() -> u32 {
    with_arm(|c| c.commits())
}

/// Model timestep (seconds); pick `steps` so one commit covers ~16 ms.
pub fn read_timestep() -> f32 {
    with_arm(|c| c.timestep() as f32)
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
    with_arm(|_| ());
    crate::window::armview_alive()
}
