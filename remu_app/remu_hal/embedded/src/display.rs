//! Display device (MMIO framebuffer + mouse input).
//!
//! These functions talk to the `display` device (default
//! `display@0x8800_0000`) via its MMIO registers. The framebuffer itself is a
//! fixed memory region (`0x8900_0000`, 2048×2048 0RGB pixels) — this module
//! only handles control/mouse/status, not pixel writes.

use crate::addresses::DISPLAY_BASE;
use core::ptr::{read_volatile, write_volatile};

/// Framebuffer base address (matches the display device's declared region).
pub const FB_BASE: usize = 0x8900_0000;
/// Framebuffer capacity (matches the display device).
pub const FB_WIDTH: usize = 2048;
pub const FB_HEIGHT: usize = 2048;

/// Control register offset: writing here signals "frame finished".
const REG_CTRL: usize = 4;
/// Mouse X position register (framebuffer pixels).
const REG_MOUSE_X: usize = 8;
/// Mouse Y position register (framebuffer pixels).
const REG_MOUSE_Y: usize = 12;
/// Mouse buttons register (bit 0=left, 1=right, 2=middle).
const REG_MOUSE_BUTTONS: usize = 16;
/// Current active display width register (framebuffer pixels).
const REG_DISP_W: usize = 20;
/// Current active display height register (framebuffer pixels).
const REG_DISP_H: usize = 24;

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

/// Signal to the display device that the current frame is complete.
#[inline]
pub fn frame_done() {
    unsafe { write_volatile((DISPLAY_BASE + REG_CTRL) as *mut u32, 1) }
}

/// Read the current active display resolution (framebuffer pixels).
#[inline]
pub fn read_disp_size() -> DisplaySize {
    DisplaySize {
        width: read_disp_w(),
        height: read_disp_h(),
    }
}

/// Read the current active display width (framebuffer pixels).
#[inline]
pub fn read_disp_w() -> usize {
    unsafe { read_volatile((DISPLAY_BASE + REG_DISP_W) as *const u32) as usize }
}

/// Read the current active display height (framebuffer pixels).
#[inline]
pub fn read_disp_h() -> usize {
    unsafe { read_volatile((DISPLAY_BASE + REG_DISP_H) as *const u32) as usize }
}

/// Read the mouse position and button state (framebuffer pixels).
#[inline]
pub fn read_mouse() -> MouseState {
    MouseState {
        x: read_mouse_x(),
        y: read_mouse_y(),
        buttons: read_mouse_buttons(),
    }
}

/// Read the mouse X position (framebuffer pixels).
#[inline]
pub fn read_mouse_x() -> usize {
    unsafe { read_volatile((DISPLAY_BASE + REG_MOUSE_X) as *const u32) as usize }
}

/// Read the mouse Y position (framebuffer pixels).
#[inline]
pub fn read_mouse_y() -> usize {
    unsafe { read_volatile((DISPLAY_BASE + REG_MOUSE_Y) as *const u32) as usize }
}

/// Read the mouse buttons bitmask (bit 0=left, 1=right, 2=middle).
#[inline]
pub fn read_mouse_buttons() -> u32 {
    unsafe { read_volatile((DISPLAY_BASE + REG_MOUSE_BUTTONS) as *const u32) }
}
