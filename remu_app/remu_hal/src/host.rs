//! Host (x86_64) equivalents of embedded HAL items.

use core::fmt;

/// Stdout writer, API-compatible with `Uart16550`.
pub struct Stdout;

impl Stdout {
    #[inline]
    pub const fn default_base() -> Self {
        Stdout
    }
}

impl fmt::Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        use std::io::Write;
        std::io::stdout()
            .write_all(s.as_bytes())
            .map_err(|_| fmt::Error)
    }
}

/// No-op heap init on host (global allocator already set up by std).
#[inline]
pub fn init() {}

/// MTIME tick frequency (host returns 0 — not applicable).
pub const MTIME_TICK_HZ: u64 = 0;

/// Read CLINT mtime (host returns 0 — not applicable).
#[inline]
pub fn read_mtime() -> u64 {
    0
}

// ── Display device stubs (host has no display device) ──

/// Active display resolution in framebuffer pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DisplaySize {
    /// Width in framebuffer pixels.
    pub width: usize,
    /// Height in framebuffer pixels.
    pub height: usize,
}

/// Mouse position (framebuffer pixels) and button state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MouseState {
    /// Cursor X in framebuffer pixels.
    pub x: usize,
    /// Cursor Y in framebuffer pixels.
    pub y: usize,
    /// Button bitmask (bit 0=left, 1=right, 2=middle).
    pub buttons: u32,
}

/// Framebuffer base address (host stub — not applicable).
pub const FB_BASE: usize = 0;
pub const FB_HEIGHT: usize = 0;
pub const FB_WIDTH: usize = 0;

/// Signal frame-complete (host stub — no-op).
#[inline]
pub fn frame_done() {}

/// Read display resolution (host stub — zeroed).
#[inline]
pub fn read_disp_size() -> DisplaySize {
    DisplaySize::default()
}

/// Read display width (host stub — returns 0).
#[inline]
pub fn read_disp_w() -> usize {
    0
}

/// Read display height (host stub — returns 0).
#[inline]
pub fn read_disp_h() -> usize {
    0
}

/// Read mouse state (host stub — zeroed).
#[inline]
pub fn read_mouse() -> MouseState {
    MouseState::default()
}

/// Read mouse X (host stub — returns 0).
#[inline]
pub fn read_mouse_x() -> usize {
    0
}

/// Read mouse Y (host stub — returns 0).
#[inline]
pub fn read_mouse_y() -> usize {
    0
}

/// Read mouse buttons (host stub — returns 0).
#[inline]
pub fn read_mouse_buttons() -> u32 {
    0
}
