//! Scan device (MMIO): a planar laser-fan sensor over the shared world.
//!
//! Exposes the latest committed step's fan of ranges in LaserScan shape: one
//! frame per commit, refreshed lazily on the first read after the world
//! advanced (deterministic frame-vs-commit coupling; nothing is cast while the
//! app does not read). The fan geometry (origin, angular sweep, range limit)
//! is reported through the same register window — the sensor's mounting pose
//! is part of its frame, so the app can lift polar returns into the world
//! frame without duplicating installation constants.
//!
//! Rays hit everything in the model, including the arm itself (a real scanner
//! also sees its robot; dropping self returns is the sensing layer's job).
//!
//! Register layout matches the embedded HAL (`remu_hal_embedded/src/scan.rs`).

use std::sync::Arc;

use remu_state_mujoco::{ScanParams, Scanner};

use crate::bus::{BusError, device::DeviceContext};

use super::WorldHost;

/// Register-map capacity of the range block; a sensor may report fewer rays.
const MAX_RAYS: usize = 64;

// Register offsets (32-bit registers; f32 values travel as their bit pattern).
const REG_N_RAYS: usize = 0x00; // r: rays per frame
const REG_ANGLE_MIN: usize = 0x04; // r: first ray's yaw (radians)
const REG_ANGLE_INCR: usize = 0x08; // r: yaw step per ray (radians)
const REG_RANGE_MAX: usize = 0x0C; // r: metres; "no return" reports this
const REG_SEQ: usize = 0x10; // r: frame sequence (= commit count of the state)
const REG_ORIGIN_X: usize = 0x14; // r: fan origin (world frame, metres)
const REG_ORIGIN_Y: usize = 0x18;
const REG_ORIGIN_Z: usize = 0x1C;
const REG_RANGES: usize = 0x20; // r: surface distances (metres), stride 4
const SCAN_REG_SIZE: usize = REG_RANGES + 4 * MAX_RAYS;

pub(super) struct Scan {
    /// The shared world; attached at bus build time.
    world: Option<Arc<WorldHost>>,
    /// The sensor model (params + lazily refreshed frame).
    scanner: Scanner,
}

impl Scan {
    pub(super) fn new() -> Self {
        let scanner = Scanner::new(ScanParams::default());
        assert!(
            scanner.params().n_rays <= MAX_RAYS,
            "scan: params ({} rays) exceed the register map",
            scanner.params().n_rays
        );
        Self {
            world: None,
            scanner,
        }
    }

    /// Recast the fan when the world advanced past the cached frame.
    fn refresh(&mut self) {
        let world = self.world.as_ref().expect("scan: world not attached");
        let scanner = &mut self.scanner;
        world.with_core(|core| scanner.refresh(core));
    }
}

impl super::DeviceAccess for Scan {
    fn name(&self) -> &str {
        "scan"
    }

    fn size(&self) -> usize {
        SCAN_REG_SIZE
    }

    fn attach_context(&mut self, ctx: &DeviceContext) {
        self.world = Some(Arc::clone(ctx.world()));
    }

    fn read_32(&mut self, offset: usize) -> Result<u32, BusError> {
        let p = *self.scanner.params();
        match offset {
            REG_N_RAYS => Ok(p.n_rays as u32),
            REG_ANGLE_MIN => Ok(p.angle_min.to_bits()),
            REG_ANGLE_INCR => Ok(p.angle_incr.to_bits()),
            REG_RANGE_MAX => Ok(p.range_max.to_bits()),
            REG_ORIGIN_X => Ok((p.origin[0] as f32).to_bits()),
            REG_ORIGIN_Y => Ok((p.origin[1] as f32).to_bits()),
            REG_ORIGIN_Z => Ok((p.origin[2] as f32).to_bits()),
            REG_SEQ => {
                self.refresh();
                Ok(self.scanner.seq())
            }
            o if (REG_RANGES..REG_RANGES + 4 * p.n_rays).contains(&o) => {
                self.refresh();
                Ok(self.scanner.ranges()[(o - REG_RANGES) / 4].to_bits())
            }
            _ => Err(BusError::UnsupportedAccessWidth(
                32,
                std::backtrace::Backtrace::capture(),
            )),
        }
    }

    fn write_32(&mut self, _offset: usize, _value: u32) -> Result<(), BusError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::bus::device::DeviceAccess as _;

    use super::super::arm::Arm;
    use super::*;

    /// A test context over the built-in toy-arm world, no window.
    fn test_ctx() -> DeviceContext {
        let mut ctx = DeviceContext::default();
        ctx.set_world(Arc::new(WorldHost::new()));
        ctx
    }

    fn scan(ctx: &DeviceContext) -> Scan {
        let mut s = Scan::new();
        s.attach_context(ctx);
        s
    }

    #[test]
    fn frame_geometry_is_reported() {
        let ctx = test_ctx();
        let mut s = scan(&ctx);
        let p = ScanParams::default();
        assert_eq!(s.read_32(REG_N_RAYS).unwrap() as usize, p.n_rays);
        let incr = f32::from_bits(s.read_32(REG_ANGLE_INCR).unwrap());
        assert!((incr - p.angle_incr).abs() < 1e-6);
        let rmax = f32::from_bits(s.read_32(REG_RANGE_MAX).unwrap());
        assert!((rmax - p.range_max).abs() < 1e-6);
        let x = f32::from_bits(s.read_32(REG_ORIGIN_X).unwrap());
        let y = f32::from_bits(s.read_32(REG_ORIGIN_Y).unwrap());
        let z = f32::from_bits(s.read_32(REG_ORIGIN_Z).unwrap());
        assert!((x - p.origin[0] as f32).abs() < 1e-6);
        assert!((y - p.origin[1] as f32).abs() < 1e-6);
        assert!((z - p.origin[2] as f32).abs() < 1e-6);
    }

    #[test]
    fn ranges_are_finite_and_within_the_band() {
        let ctx = test_ctx();
        let mut s = scan(&ctx);
        let n = ScanParams::default().n_rays;
        for i in 0..n {
            let r = f32::from_bits(s.read_32(REG_RANGES + 4 * i).unwrap());
            assert!(
                r > 0.0 && r <= ScanParams::default().range_max,
                "ray {i}: {r}"
            );
        }
    }

    #[test]
    fn seq_tracks_the_shared_world_commits() {
        let ctx = test_ctx();
        let mut a = Arm::new();
        a.attach_context(&ctx);
        let mut s = scan(&ctx);

        assert_eq!(s.read_32(REG_SEQ).unwrap(), 0);
        // The world is untouched: re-reading keeps the same frame.
        assert_eq!(s.read_32(REG_SEQ).unwrap(), 0);

        a.write_32(0x08, 0.5f32.to_bits()).unwrap(); // target 0
        a.write_32(0x28, 1).unwrap(); // commit
        assert_eq!(s.read_32(REG_SEQ).unwrap(), 1);

        // Reading twice inside one commit returns the same frame.
        assert_eq!(s.read_32(REG_SEQ).unwrap(), 1);
        a.write_32(0x28, 1).unwrap();
        assert_eq!(s.read_32(REG_SEQ).unwrap(), 2);
    }
}
