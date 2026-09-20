//! Display device on host: heap-backed framebuffer + window-size accessors.
//!
//! The embedded app talks to the display via MMIO registers and a fixed
//! framebuffer region (`FB_BASE`). On host there is no MMIO, so the same calls
//! are backed by:
//!   - a real `[u32; FB_WIDTH*FB_HEIGHT]` buffer (so `fb_base` is a genuine
//!     writable address and app code works unchanged),
//!   - the lazily-spawned window from [`crate::window`], which blits the
//!     framebuffer and feeds the active window size back to the app.

use std::sync::OnceLock;

/// Framebuffer capacity (matches the display device).
pub const FB_WIDTH: usize = 2048;
pub const FB_HEIGHT: usize = 2048;

/// The host framebuffer backing store (heap-allocated, writable).
/// The app writes through a raw pointer derived from [`fb_base`].
static FRAMEBUFFER: OnceLock<Box<[u32]>> = OnceLock::new();

/// Address of the host framebuffer (used as `fb_base`).
#[inline]
fn fb_addr() -> usize {
    // Allocate on first use; never freed. The boxed slice is writable heap
    // memory, so raw-pointer writes by the app are valid.
    FRAMEBUFFER
        .get_or_init(|| vec![0u32; FB_WIDTH * FB_HEIGHT].into_boxed_slice())
        .as_ptr() as usize
}

/// Base address of the display framebuffer (canonical runtime API).
#[inline]
pub fn fb_base() -> usize {
    fb_addr()
}

/// Write a single 0RGB pixel into the framebuffer (bounds-checked to capacity).
///
/// `v` is a 0RGB u32: 0x00RRGGBB (XRGB).
///
/// Safe API by design (parity with the embedded twin in `remu_hal_embedded`):
/// the caller contract is that `fb` comes from [`fb_base`], so the write lands
/// in the leaked framebuffer, and the bounds check below keeps it in range.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[inline]
pub fn put_pixel(fb: *mut u32, x: usize, y: usize, v: u32) {
    if x < FB_WIDTH && y < FB_HEIGHT {
        unsafe {
            *fb.add(y * FB_WIDTH + x) = v;
        }
    }
}

/// Active display resolution in framebuffer pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DisplaySize {
    /// Width in framebuffer pixels.
    pub width: usize,
    /// Height in framebuffer pixels.
    pub height: usize,
}

/// Signal to the display device that the current frame is complete.
#[inline]
pub fn frame_done() {
    crate::window::wake();
}

/// Whether the display window is currently alive (host: not closed).
#[inline]
pub fn display_alive() -> bool {
    crate::window::alive()
}

/// Read the current active display resolution (framebuffer pixels).
#[inline]
pub fn read_disp_size() -> DisplaySize {
    *crate::window::shared().disp.lock().unwrap()
}

/// Read the current active display width (framebuffer pixels).
#[inline]
pub fn read_disp_w() -> usize {
    read_disp_size().width
}

/// Read the current active display height (framebuffer pixels).
#[inline]
pub fn read_disp_h() -> usize {
    read_disp_size().height
}
