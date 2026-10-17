// Panda demo controller: drives a Franka Emika Panda (7 joint servos + 1
// gripper channel) through the arm device registers. The host loads the
// Menagerie model with
// `REMU_ARM_MODEL=$MUJOCO_MENAGERIE_DIR/franka_emika_panda/scene.xml`;
// this app exits early when the model is not a Panda.
//
// Motion: a gentle phase-shifted joint-space sweep around the model's `home`
// keyframe pose plus a slow gripper cycle. All in joint space, so the app
// carries no kinematics — the point here is the model swap and the control
// path, not the trajectory quality.

extern crate alloc;

use libm::{roundf, sinf};
use remu_hal::{
    armview_alive, commit, exit_success, read_mtime, read_nq, read_nu, read_qpos, read_timestep,
    set_steps, set_target, MTIME_TICK_HZ,
};

/// Panda actuator layout: 7 joint position servos + one gripper channel.
const JOINTS: usize = 7;
const GRIPPER: usize = 7;

/// `home` keyframe pose (joints only); sweeps stay close to it.
const HOME: [f32; JOINTS] = [0.0, 0.0, 0.0, -1.57079, 0.0, 1.57079, -0.7853];

/// Per-joint sweep amplitude (rad) and rate (rad/s) — small around the home
/// pose so the arm stays clear of the floor.
const AMP: [f32; JOINTS] = [0.6, 0.25, 0.2, 0.3, 0.2, 0.35, 0.3];
const RATE: [f32; JOINTS] = [0.35, 0.5, 0.55, 0.45, 0.6, 0.5, 0.65];

/// Gripper channel range (model: 0..255, 255 = open).
const GRIPPER_OPEN: f32 = 255.0;
const GRIPPER_MOSTLY_CLOSED: f32 = 60.0;

/// Control period (~60 Hz).
const TICK_MS: u64 = 16;

#[remu_hal::entry]
fn main() -> ! {
    remu_hal::init();
    let nu = read_nu();
    remu_hal::println!(
        "robot_arm: Menagerie Panda demo (nu={nu}, nq={})",
        read_nq()
    );

    if nu != (JOINTS + 1) as u32 {
        remu_hal::println!(
            "robot_arm: model is not a Panda; set \
             REMU_ARM_MODEL=$MUJOCO_MENAGERIE_DIR/franka_emika_panda/scene.xml (dev shell) and retry"
        );
        exit_success();
    }

    // One commit ≈ TICK_MS of simulated time (the Panda steps at 2 ms).
    let steps = roundf(TICK_MS as f32 / 1000.0 / read_timestep()) as u32;
    set_steps(steps.clamp(1, 64));

    let t0 = read_mtime();
    let mut last = 0u64;
    let mut tick = 0u32;

    loop {
        // The monitor window is the only visible output: when it goes away,
        // stop instead of controlling with nothing to show.
        if !armview_alive() {
            remu_hal::println!("robot_arm: armview monitor closed, exiting");
            exit_success();
        }

        let ms = read_mtime().wrapping_sub(t0) * 1000 / MTIME_TICK_HZ as u64;
        let t = ms as f32 / 1000.0;

        for j in 0..JOINTS {
            set_target(j, HOME[j] + AMP[j] * sinf(RATE[j] * t + j as f32));
        }
        let grip = GRIPPER_MOSTLY_CLOSED
            + (GRIPPER_OPEN - GRIPPER_MOSTLY_CLOSED) * 0.5 * (1.0 + sinf(0.3 * t));
        set_target(GRIPPER, grip);
        commit();

        if tick % 64 == 0 {
            remu_hal::println!(
                "t={:.2}s q=[{:.2} {:.2} {:.2} {:.2} {:.2} {:.2} {:.2}] grip~{:.0}",
                t,
                read_qpos(0),
                read_qpos(1),
                read_qpos(2),
                read_qpos(3),
                read_qpos(4),
                read_qpos(5),
                read_qpos(6),
                grip,
            );
        }
        tick = tick.wrapping_add(1);

        // Throttle to the control period.
        let mut now = ms;
        while now < last + TICK_MS {
            now = read_mtime().wrapping_sub(t0) * 1000 / MTIME_TICK_HZ as u64;
        }
        last = now;
    }
}
