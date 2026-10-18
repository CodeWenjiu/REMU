// Panda demo controller: drives a Franka Emika Panda (7 joint servos + 1
// gripper channel) through the arm device registers. The host loads the
// Menagerie model with
// `REMU_ARM_MODEL=$MUJOCO_MENAGERIE_DIR/franka_emika_panda/scene.xml`;
// this app exits early when the model is not a Panda.
//
// Motion: the grasp point is servoed through a slow arc swing around the base
// axis, anchored at the model's `home` tip position, with a little radius
// breathing and height bob. Each control tick runs dense damped least-squares
// IK (`kinematics`, over the model-generated joint chain) and streams the
// solved configuration; the gripper opens and closes on a slow cycle. The
// chain length, `home` pose and the expected model counts are generated from
// the MJCF at build time (see `kin`).

extern crate alloc;

remu_macro::mod_prv!(kin);

use kinematics::prelude::{Chain, Vec3};
use libm::{cosf, roundf, sinf, sqrtf};
use remu_hal::{
    MTIME_TICK_HZ, armview_alive, commit, exit_success, read_mtime, read_nq, read_nu,
    read_timestep, set_steps, set_target,
};

/// Arm chain length, generated from the model (`mjcf`); the gripper rides the
/// actuator channel after the joints.
const JOINTS: usize = kin::NJ;
const GRIPPER: usize = JOINTS;

/// `home` keyframe pose (joints only), from the model's `home` key when
/// present; the target loop is anchored at its FK tip position.
const HOME: [f32; JOINTS] = match kin::HOME_Q {
    Some(home) => home,
    None => [0.0; JOINTS],
};

/// The generated chain as const data; `ARM.reach(..)` specializes to `NJ`.
const ARM: Chain<{ kin::NJ }> = Chain {
    joints: &kin::JOINTS,
    tip_offset: kin::TIP_OFFSET,
};

/// IK iterations per control tick, and the grasp loop around the home tip:
/// a swing arc (rad) plus radius breathing / height bob (m), all slow.
const IK_ITERS: u32 = 16;
const LOOP_ARC: f32 = 1.5;
const LOOP_BREATH: f32 = 0.05;
const LOOP_BOB: f32 = 0.05;
const LOOP_RATE: f32 = 0.45;
const BREATH_RATE: f32 = 0.31;
const BOB_RATE: f32 = 0.37;

/// Gripper channel range (model: 0..255, 255 = open).
const GRIPPER_OPEN: f32 = 255.0;
const GRIPPER_MOSTLY_CLOSED: f32 = 60.0;

/// Control period (~60 Hz).
const TICK_MS: u64 = 16;

#[remu_hal::entry]
fn main() -> ! {
    remu_hal::init();
    let nu = read_nu();
    let nq = read_nq();
    remu_hal::println!("robot_arm: Menagerie Panda demo (nu={nu}, nq={nq})");

    if nu != kin::NU as u32 || nq != kin::NQ as u32 {
        remu_hal::println!(
            "robot_arm: unexpected model (nu={nu}, nq={nq}; expected nu={}, nq={}); set \
             REMU_ARM_MODEL=$MUJOCO_MENAGERIE_DIR/{} (dev shell) and retry",
            kin::NU,
            kin::NQ,
            kin::SOURCE
        );
        exit_success();
    }

    // One commit ≈ TICK_MS of simulated time (the Panda steps at 2 ms).
    let steps = roundf(TICK_MS as f32 / 1000.0 / read_timestep()) as u32;
    set_steps(steps.clamp(1, 64));

    let home_tip = ARM.tip_pos(&HOME);
    let home_r = sqrtf(home_tip[0] * home_tip[0] + home_tip[1] * home_tip[1]);
    remu_hal::println!(
        "robot_arm: home tip=[{:.3} {:.3} {:.3}] r={:.3}",
        home_tip[0],
        home_tip[1],
        home_tip[2],
        home_r
    );

    let t0 = read_mtime();
    let mut last = 0u64;
    let mut tick = 0u32;
    let mut q = HOME;

    loop {
        // The monitor window is the only visible output: when it goes away,
        // stop instead of controlling with nothing to show.
        if !armview_alive() {
            remu_hal::println!("robot_arm: armview monitor closed, exiting");
            exit_success();
        }

        let ms = read_mtime().wrapping_sub(t0) * 1000 / MTIME_TICK_HZ;
        let t = ms as f32 / 1000.0;

        // Swing back and forth instead of going full circles: joint 1 cannot
        // represent a full turn within its range, and a wrapped lap would
        // force the solver through a reconfiguration.
        let swing = LOOP_ARC * sinf(LOOP_RATE * t);
        let r = home_r + LOOP_BREATH * sinf(BREATH_RATE * t);
        let target = Vec3::new(
            r * cosf(swing),
            r * sinf(swing),
            home_tip[2] + LOOP_BOB * sinf(BOB_RATE * t),
        );
        let err = match ARM.reach(&mut q, target, IK_ITERS) {
            Ok(sol) => sol.residual,
            Err(e) => e.residual(),
        };

        for (j, &qi) in q.iter().enumerate() {
            set_target(j, qi);
        }
        let grip = GRIPPER_MOSTLY_CLOSED
            + (GRIPPER_OPEN - GRIPPER_MOSTLY_CLOSED) * 0.5 * (1.0 + sinf(0.3 * t));
        set_target(GRIPPER, grip);
        commit();

        if tick.is_multiple_of(64) {
            let tip = ARM.tip_pos(&q);
            remu_hal::println!(
                "t={:.2}s err={:.2}mm tip=[{:.3} {:.3} {:.3}]",
                t,
                err * 1000.0,
                tip[0],
                tip[1],
                tip[2]
            );
        }
        tick = tick.wrapping_add(1);

        // Throttle to the control period.
        let mut now = ms;
        while now < last + TICK_MS {
            now = read_mtime().wrapping_sub(t0) * 1000 / MTIME_TICK_HZ;
        }
        last = now;
    }
}
