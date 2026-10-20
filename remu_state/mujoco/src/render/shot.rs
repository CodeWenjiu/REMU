//! Test-only offscreen capture helpers for the windowed demo.

#![cfg(test)]

use std::os::raw::c_int;

use dlopen2::wrapper::{Container, WrapperApi};

use crate::engine::ffi::{MjrContext, MjrRect};
use crate::engine::runtime;
    #[derive(WrapperApi)]
    pub(super) struct ShotApi {
        #[dlopen2_name = "mjr_resizeOffscreen"]
        resize_offscreen: unsafe extern "C" fn(width: c_int, height: c_int, con: *mut MjrContext),
        #[dlopen2_name = "mjr_readPixels"]
        read_pixels: unsafe extern "C" fn(
            rgb: *mut u8,
            depth: *mut f32,
            viewport: MjrRect,
            con: *const MjrContext,
        ),
    }

    pub(super) fn load() -> Option<Container<ShotApi>> {
        runtime::candidate_paths()
            .into_iter()
            .find_map(|path| unsafe { Container::<ShotApi>::load(&path) }.ok())
    }
/// 24-bit BMP from bottom-up RGB rows (what `mjr_readPixels` returns).
pub(super) fn write_bmp(
    path: &std::path::Path,
    width: usize,
    height: usize,
    rgb: &[u8],
) -> std::io::Result<()> {
    let row = (width * 3).div_ceil(4) * 4;
    let size = 54 + row * height;
    let mut out = Vec::with_capacity(size);
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&(size as u32).to_le_bytes());
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&54u32.to_le_bytes());
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&(width as i32).to_le_bytes());
    out.extend_from_slice(&(height as i32).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&24u16.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&((row * height) as u32).to_le_bytes());
    out.extend_from_slice(&2835u32.to_le_bytes());
    out.extend_from_slice(&2835u32.to_le_bytes());
    out.extend_from_slice(&[0; 8]);
    for y in 0..height {
        for x in 0..width {
            let i = (y * width + x) * 3;
            out.extend_from_slice(&[rgb[i + 2], rgb[i + 1], rgb[i]]);
        }
        out.extend(std::iter::repeat_n(0u8, row - width * 3));
    }
    std::fs::write(path, out)
}
