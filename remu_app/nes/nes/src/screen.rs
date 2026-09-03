//! NES `Screen` implementation: renders the PPU's 256×240 output into the
//! display framebuffer (0RGB), scaled to fit the active window.

use remu_hal::{FB_WIDTH, read_disp_size};
use runes_core::ppu::Screen;

/// Set by `Screen::frame` (vblank) each frame; main clears it after detecting.
pub(crate) static mut FRAME_DONE: bool = false;

/// The NES palette (64 colors, 0RGB). Matches the classic NES 2C02 palette.
const PALETTE: [u32; 64] = [
    0x006666, 0x002a88, 0x1412a7, 0x3b00a4, 0x5c007e, 0x6e0040, 0x6c0600,
    0x561d00, // 0x00-0x07
    0x333500, 0x0b4800, 0x005200, 0x004f08, 0x00404d, 0x000000, 0x000000,
    0x000000, // 0x08-0x0f
    0xadadad, 0x155fd9, 0x4240ff, 0x7527fe, 0xa01acc, 0xb71e7b, 0xb53120,
    0x994e00, // 0x10-0x17
    0x6b6d00, 0x388700, 0x0c9300, 0x008f32, 0x007c8d, 0x000000, 0x000000,
    0x000000, // 0x18-0x1f
    0xfffeff, 0x64b0ff, 0x9290ff, 0xc676ff, 0xf36aff, 0xfe6ecc, 0xfe8170,
    0xea9e22, // 0x20-0x27
    0xbcbe00, 0x88d800, 0x5ce430, 0x45e082, 0x48cdde, 0x4f4f4f, 0x000000,
    0x000000, // 0x28-0x2f
    0xfffeff, 0xc0dfff, 0xd3d2ff, 0xe8c8ff, 0xfbc2ff, 0xfec4ea, 0xfeccc5,
    0xf7d8a5, // 0x30-0x37
    0xe4e594, 0xcfef96, 0xbdf4ab, 0xb3f3cc, 0xb5ebf2, 0xb8b8b8, 0x000000,
    0x000000, // 0x38-0x3f
];

/// NES native resolution.
pub(crate) const NES_W: usize = 256;
pub(crate) const NES_H: usize = 240;

/// A `Screen` that draws into the remu display framebuffer, scaled up to fill
/// the window.
///
/// The PPU calls `put` for every visible pixel across the frame, then `render`
/// once at vblank (all 240 lines are in the buffer), then `frame`.
pub(crate) struct NesScreen {
    fb: *mut u32,
    /// Full 256×240 internal buffer of 0RGB pixels.
    buf: [u32; NES_W * NES_H],
}

impl NesScreen {
    pub(crate) fn new(fb: *mut u32, _disp_w: usize, _disp_h: usize) -> Self {
        NesScreen {
            fb,
            buf: [0; NES_W * NES_H],
        }
    }

    /// Blit the internal buffer to the framebuffer, scaled to fit the current
    /// active display region and centered. Recomputes scale/offset each frame so
    /// it adapts to window resizes.
    ///
    /// Writes the framebuffer directly (bypassing `put_pixel`'s bounds check)
    /// because `x_off`/`y_off`/`scale` are derived from the live display size
    /// which is already clamped to `FB_WIDTH`/`FB_HEIGHT`.
    fn blit(&mut self) {
        let disp = read_disp_size();
        let disp_w = disp.width.max(1);
        let disp_h = disp.height.max(1);
        // Integer scale: largest NES pixel multiple that fits both dimensions.
        let scale = (disp_w / NES_W).min(disp_h / NES_H).max(1);
        // Center the scaled image within the active display region.
        let x_off = disp_w.saturating_sub(NES_W * scale) / 2;
        let y_off = disp_h.saturating_sub(NES_H * scale) / 2;
        let fb = self.fb;
        // SAFETY: all coordinates below are within the framebuffer capacity
        // (the display size is clamped to FB_WIDTH/FB_HEIGHT) and the
        // framebuffer outlives this call (owned by the bus).
        unsafe {
            // Clear the whole active region to black first so the letterbox
            // bars around the (centered) picture are black rather than stale
            // framebuffer contents. Each pixel is a u32 (4 bytes); write_bytes
            // counts in T (=u32) units, so disp_w elements = disp_w pixels.
            for y in 0..disp_h {
                let dst = fb.add(y * FB_WIDTH);
                core::ptr::write_bytes(dst, 0, disp_w);
            }
            if scale == 1 {
                // Fast path: one contiguous copy per row, no per-pixel loop.
                for y in 0..NES_H {
                    let src = self.buf.as_ptr().add(y * NES_W);
                    let dst = fb.add((y_off + y) * FB_WIDTH + x_off);
                    core::ptr::copy_nonoverlapping(src, dst, NES_W);
                }
            } else {
                // Scaled path: replicate each source pixel into a scale×scale
                // block via pointer writes (no bounds check per pixel).
                for y in 0..NES_H {
                    let row = self.buf.as_ptr().add(y * NES_W);
                    for x in 0..NES_W {
                        let pix = *row.add(x);
                        let px = x_off + x * scale;
                        let py = y_off + y * scale;
                        for sy in 0..scale {
                            let dst = fb.add((py + sy) * FB_WIDTH + px);
                            for sx in 0..scale {
                                *dst.add(sx) = pix;
                            }
                        }
                    }
                }
            }
        }
    }
}

impl Screen for NesScreen {
    #[inline]
    fn render(&mut self, pixels: &[u8; 256 * 240]) {
        // Convert the PPU's palette indices to 0RGB and stash them in the
        // internal buffer, then blit to the framebuffer. One conversion per
        // pixel per frame (no per-pixel trait dispatch on the emulation path).
        let buf = &mut self.buf;
        for (dst, &idx) in buf.iter_mut().zip(pixels.iter()) {
            *dst = PALETTE[(idx as usize) & 0x3f];
        }
        self.blit();
    }

    #[inline(always)]
    fn frame(&mut self) {
        // Frame complete: signal the display to present it and flag the main
        // loop that a full frame was rendered.
        remu_hal::frame_done();
        // SAFETY: single-threaded; main resets this after each frame.
        unsafe { FRAME_DONE = true };
    }
}
