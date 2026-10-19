//! Scan device on host: the planar laser fan served from the shared world
//! (`world`), without the remu bus. Mirrors the embedded register surface
//! (`remu_hal_embedded/src/scan.rs`) item for item.

use std::cell::RefCell;

use remu_state_mujoco::{ScanParams, Scanner};

use crate::world;

/// Register-map capacity of the range block; read [`read_n_rays`] for the
/// frame's actual ray count.
pub const SCAN_RAYS_MAX: usize = 64;

thread_local! {
    static SCAN: RefCell<Scanner> = RefCell::new(Scanner::new(ScanParams::default()));
}

fn with_scanner<R>(f: impl FnOnce(&mut Scanner) -> R) -> R {
    SCAN.with(|s| f(&mut s.borrow_mut()))
}

/// Rays per frame.
pub fn read_n_rays() -> u32 {
    with_scanner(|s| s.params().n_rays as u32)
}

/// First ray's yaw in radians.
pub fn read_angle_min() -> f32 {
    with_scanner(|s| s.params().angle_min)
}

/// Yaw step per ray in radians.
pub fn read_angle_incr() -> f32 {
    with_scanner(|s| s.params().angle_incr)
}

/// Upper range bound in metres; "no return" rays report this.
pub fn read_range_max() -> f32 {
    with_scanner(|s| s.params().range_max)
}

/// Fan origin in the world frame, per axis (metres).
pub fn read_origin_x() -> f32 {
    with_scanner(|s| s.params().origin[0] as f32)
}

pub fn read_origin_y() -> f32 {
    with_scanner(|s| s.params().origin[1] as f32)
}

pub fn read_origin_z() -> f32 {
    with_scanner(|s| s.params().origin[2] as f32)
}

/// Frame sequence number (= commit count of the observed world state).
pub fn read_scan_seq() -> u32 {
    with_scanner(|s| {
        world::with_core(|core| s.refresh(core));
        s.seq()
    })
}

/// Surface distance (metres) of ray `i` in the latest frame.
pub fn read_range(i: usize) -> f32 {
    with_scanner(|s| {
        world::with_core(|core| s.refresh(core));
        s.ranges().get(i).copied().unwrap_or(0.0)
    })
}
