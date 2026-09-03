//! NES `Screen` implementation: renders the PPU's 256×240 output into the
//! display framebuffer (0RGB), scaled to fit the active window.

use remu_hal::{put_pixel, read_disp_size};
use runes_core::ppu::Screen;

/// Debug counter: number of `put` calls per frame. Single-threaded; the NES
/// loop both writes (in `put`) and reads (in main) on the same thread.
pub(crate) static mut PUTS_PER_FRAME: u32 = 0;
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
        for y in 0..NES_H {
            for x in 0..NES_W {
                let pix = self.buf[y * NES_W + x];
                let px = x_off + x * scale;
                let py = y_off + y * scale;
                for sy in 0..scale {
                    for sx in 0..scale {
                        put_pixel(fb, px + sx, py + sy, pix);
                    }
                }
            }
        }
    }
}

impl Screen for NesScreen {
    #[inline(always)]
    fn put(&mut self, x: u8, y: u8, color: u8) {
        // SAFETY: single-threaded; only touched from `put`/main on one thread.
        unsafe { PUTS_PER_FRAME = PUTS_PER_FRAME.wrapping_add(1) };
        let x = x as usize;
        let y = y as usize;
        if x < NES_W && y < NES_H {
            self.buf[y * NES_W + x] = PALETTE[(color as usize) & 0x3f];
        }
    }

    #[inline(always)]
    fn render(&mut self) {
        // Called once per frame at vblank: all pixels are in the buffer.
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
