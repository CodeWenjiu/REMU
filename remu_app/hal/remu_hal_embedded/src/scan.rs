//! Scan device (MMIO): planar laser-fan ranges in LaserScan shape.
//!
//! Register layout matches `remu_state`'s scan device (`bus/device/scan.rs`).
//! The device exposes one frame per commit of the shared MuJoCo world; the
//! frame refreshes lazily on the first read after the world advanced, and
//! rays that see nothing report [`read_range_max`]. The fan geometry (origin,
//! sweep, range limit) is reported through the same window, so polar returns
//! can be lifted into the world frame without duplicated constants.

use crate::addresses::SCAN_BASE;
use core::ptr::read_volatile;

/// Register-map capacity of the range block; read [`read_n_rays`] for the
/// frame's actual ray count.
pub const SCAN_RAYS_MAX: usize = 64;

// Register offsets (32-bit registers; f32 values travel as their bit pattern).
const REG_N_RAYS: usize = 0x00; // r: rays per frame
const REG_ANGLE_MIN: usize = 0x04; // r: first ray's yaw (f32, radians)
const REG_ANGLE_INCR: usize = 0x08; // r: yaw step per ray (f32, radians)
const REG_RANGE_MAX: usize = 0x0C; // r: metres; "no return" reports this
const REG_SEQ: usize = 0x10; // r: frame sequence (= world commit count)
const REG_ORIGIN_X: usize = 0x14; // r: fan origin (world frame, f32, metres)
const REG_ORIGIN_Y: usize = 0x18;
const REG_ORIGIN_Z: usize = 0x1C;
const REG_RANGES: usize = 0x20; // r: surface distances, stride 4

#[inline]
fn read_32(offset: usize) -> u32 {
    unsafe { read_volatile((SCAN_BASE + offset) as *const u32) }
}

/// Rays per frame.
#[inline]
pub fn read_n_rays() -> u32 {
    read_32(REG_N_RAYS)
}

/// First ray's yaw in radians.
#[inline]
pub fn read_angle_min() -> f32 {
    f32::from_bits(read_32(REG_ANGLE_MIN))
}

/// Yaw step per ray in radians.
#[inline]
pub fn read_angle_incr() -> f32 {
    f32::from_bits(read_32(REG_ANGLE_INCR))
}

/// Upper range bound in metres; "no return" rays report this.
#[inline]
pub fn read_range_max() -> f32 {
    f32::from_bits(read_32(REG_RANGE_MAX))
}

/// Frame sequence number (= commit count of the observed world state).
#[inline]
pub fn read_scan_seq() -> u32 {
    read_32(REG_SEQ)
}

/// Fan origin in the world frame, per axis (metres).
#[inline]
pub fn read_origin_x() -> f32 {
    f32::from_bits(read_32(REG_ORIGIN_X))
}

#[inline]
pub fn read_origin_y() -> f32 {
    f32::from_bits(read_32(REG_ORIGIN_Y))
}

#[inline]
pub fn read_origin_z() -> f32 {
    f32::from_bits(read_32(REG_ORIGIN_Z))
}

/// Surface distance (metres) of ray `i` in the latest frame.
#[inline]
pub fn read_range(i: usize) -> f32 {
    if i < SCAN_RAYS_MAX {
        f32::from_bits(read_32(REG_RANGES + 4 * i))
    } else {
        0.0
    }
}
