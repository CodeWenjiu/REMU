// Robot-arm controller: a 3-DOF planar arm driven through the arm device
// registers. The app solves inverse kinematics toward a scripted end-effector
// trajectory and writes joint targets; the device simulates and the armview
// monitor window renders. The app never renders anything itself.

extern crate alloc;

use libm::{cosf, sinf};
use remu_hal::{
    MTIME_TICK_HZ, armview_alive, commit, exit_success, read_mtime, read_qpos, set_target,
};

/// Link lengths (m) and base height; mirrors the arm MJCF used by the device.
const LEN: [f32; 3] = [0.3, 0.25, 0.2];
const BASE_Z: f32 = 0.1;

/// Control period (~60 Hz).
const TICK_MS: u64 = 16;
/// One normalized transpose-Jacobian IK step per tick; the position servos
/// smooth the rest.
const IK_GAIN: f32 = 0.4;
const IK_DAMPING: f32 = 0.08;

#[remu_hal::entry]
fn main() -> ! {
    remu_hal::init();
    remu_hal::println!("robot_arm: 3-DOF planar IK controller (circle trajectory)");

    let t0 = read_mtime();
    let mut last = 0u64;
    let mut q = [0f32; 3];
    let mut tick = 0u32;

    loop {
        // The monitor window is the only visible output: when it goes away
        // (closed, or never opened), there is nothing left to show — stop.
        if !armview_alive() {
            remu_hal::println!("robot_arm: armview monitor closed, exiting");
            exit_success();
        }

        let ms = read_mtime().wrapping_sub(t0) * 1000 / MTIME_TICK_HZ as u64;
        let t = ms as f32 / 1000.0;

        // End-effector target: a circle in the XZ plane.
        let target = (0.42 + 0.14 * cosf(1.2 * t), 0.40 + 0.14 * sinf(1.2 * t));

        // One damped transpose-Jacobian IK step.
        let points = fk(&q);
        let (ex, ez) = points[3];
        let (err_x, err_z) = (target.0 - ex, target.1 - ez);
        let mut norm = IK_DAMPING;
        let mut dots = [0f32; 3];
        for i in 0..3 {
            let (px, pz) = points[i];
            let (jx, jz) = (ez - pz, -(ex - px));
            dots[i] = err_x * jx + err_z * jz;
            norm += jx * jx + jz * jz;
        }
        for i in 0..3 {
            q[i] = (q[i] + IK_GAIN * dots[i] / norm).clamp(-1.4, 1.4);
            set_target(i, q[i]);
        }
        commit();

        // Periodic status from the device readback (what the servo actually
        // did; `ee` is the commanded end effector).
        if tick % 64 == 0 {
            let (ex, ez) = fk(&q)[3];
            remu_hal::println!(
                "t={:.2}s q=[{:.2} {:.2} {:.2}] ee=({:.2}, {:.2})",
                t,
                read_qpos(0),
                read_qpos(1),
                read_qpos(2),
                ex,
                ez
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

/// Forward kinematics: cumulative joint angles along the chain (XZ plane,
/// MuJoCo y-axis hinge sense: positive rotation takes +x toward -z).
fn fk(q: &[f32; 3]) -> [(f32, f32); 4] {
    let mut points = [(0f32, BASE_Z); 4];
    let mut angle = 0f32;
    for i in 0..3 {
        angle += q[i];
        let (px, pz) = points[i];
        points[i + 1] = (px + LEN[i] * cosf(angle), pz - LEN[i] * sinf(angle));
    }
    points
}
