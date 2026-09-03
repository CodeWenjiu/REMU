//! Display device (MMIO framebuffer) + mouse + keyboard input devices.
//!
//! The framebuffer is a fixed memory region (`0x8900_0000`, 2048×2048 0RGB
//! pixels); the display registers (`display@0x8800_0000`) control frame sync and
//! report the active window size. Mouse (`mouse@0x8800_1000`) and keyboard
//! (`keyboard@0x8800_2000`) are separate devices but grouped here as the HAL's
//! input surface.

use crate::addresses::{DISPLAY_BASE, KEYBOARD_BASE, MOUSE_BASE};
use core::ptr::{read_volatile, write_volatile};

/// Framebuffer base address (matches the display device's declared region).
pub const FB_BASE: usize = 0x8900_0000;
/// Framebuffer capacity (matches the display device).
pub const FB_WIDTH: usize = 2048;
pub const FB_HEIGHT: usize = 2048;

/// Runtime framebuffer base address (portable across embedded/host).
#[inline]
pub fn fb_base() -> usize {
    FB_BASE
}

/// Status register offset: bit 0 = framebuffer ready, bit 1 = window alive.
const REG_STATUS: usize = 0;
/// Control register offset: writing here signals "frame finished".
const REG_CTRL: usize = 4;
/// Current active display width register (framebuffer pixels).
const REG_DISP_W: usize = 20;
/// Current active display height register (framebuffer pixels).
const REG_DISP_H: usize = 24;

// Mouse device register offsets (relative to `MOUSE_BASE`).
const REG_MOUSE_X: usize = 0;
const REG_MOUSE_Y: usize = 4;
const REG_MOUSE_BUTTONS: usize = 8;

// Keyboard device register offsets (relative to `KEYBOARD_BASE`).
const REG_KEY_CODE: usize = 0;
const REG_KEY_DOWN: usize = 4;
const REG_KEY_TEXT: usize = 8;
const REG_KEY_VALID: usize = 12;
const REG_KEY_BUTTONS: usize = 16;

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

/// Key state (keycode + press/release + text char).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KeyState {
    /// Physical key code (winit `PhysicalKey::Code` value).
    pub code: u32,
    /// Whether the last event was a press (true) or release (false).
    pub down: bool,
    /// Last text character (if printable), else 0.
    pub text: u32,
    /// Set once a key event has occurred.
    pub valid: bool,
}

/// Write a single 0RGB pixel into the framebuffer (bounds-checked to capacity).
///
/// `v` is a 0RGB u32: 0x00RRGGBB (XRGB, byte order `0x00RRGGBB`).
#[inline]
pub fn put_pixel(fb: *mut u32, x: usize, y: usize, v: u32) {
    if x < FB_WIDTH && y < FB_HEIGHT {
        unsafe {
            *fb.add(y * FB_WIDTH + x) = v;
        }
    }
}

/// Signal to the display device that the current frame is complete.
#[inline]
pub fn frame_done() {
    unsafe { write_volatile((DISPLAY_BASE + REG_CTRL) as *mut u32, 1) }
}

/// Whether the display window is currently alive.
///
/// Reads the display STATUS register's bit 1. Returns `false` once the window
/// is closed (or if no display device is present), so apps can stop rendering
/// and exit gracefully instead of spinning forever.
#[inline]
pub fn display_alive() -> bool {
    unsafe { read_volatile((DISPLAY_BASE + REG_STATUS) as *const u32) & 2 != 0 }
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
    unsafe { read_volatile((MOUSE_BASE + REG_MOUSE_X) as *const u32) as usize }
}

/// Read the mouse Y position (framebuffer pixels).
#[inline]
pub fn read_mouse_y() -> usize {
    unsafe { read_volatile((MOUSE_BASE + REG_MOUSE_Y) as *const u32) as usize }
}

/// Read the mouse buttons bitmask (bit 0=left, 1=right, 2=middle).
#[inline]
pub fn read_mouse_buttons() -> u32 {
    unsafe { read_volatile((MOUSE_BASE + REG_MOUSE_BUTTONS) as *const u32) }
}

/// Read the keyboard state (keycode + press/release + text).
#[inline]
pub fn read_key() -> KeyState {
    KeyState {
        code: read_key_code(),
        down: read_key_down() != 0,
        text: read_key_text(),
        valid: read_key_valid() != 0,
    }
}

/// Read the last key code (physical position).
#[inline]
pub fn read_key_code() -> u32 {
    unsafe { read_volatile((KEYBOARD_BASE + REG_KEY_CODE) as *const u32) }
}

/// Read whether the last key event was a press (1) or release (0).
#[inline]
pub fn read_key_down() -> u32 {
    unsafe { read_volatile((KEYBOARD_BASE + REG_KEY_DOWN) as *const u32) }
}

/// Read the last text character (ASCII), or 0 if non-printable.
#[inline]
pub fn read_key_text() -> u32 {
    unsafe { read_volatile((KEYBOARD_BASE + REG_KEY_TEXT) as *const u32) }
}

/// Read whether any key event has occurred yet (1) or not (0).
#[inline]
pub fn read_key_valid() -> u32 {
    unsafe { read_volatile((KEYBOARD_BASE + REG_KEY_VALID) as *const u32) }
}

/// Read the live NES joypad button bitmask (bit 0=A, 1=B, 2=SELECT, 3=START,
/// 4=UP, 5=DOWN, 6=LEFT, 7=RIGHT). Maintained by the window render thread.
#[inline]
pub fn read_key_buttons() -> u32 {
    unsafe { read_volatile((KEYBOARD_BASE + REG_KEY_BUTTONS) as *const u32) }
}
