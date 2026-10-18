//! Mouse device on host: shared cursor state + winit event mapping.

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

/// Convert a winit button to our button-bitmask.
pub(crate) fn button_bit(b: winit::event::MouseButton) -> u32 {
    match b {
        winit::event::MouseButton::Left => 1,
        winit::event::MouseButton::Right => 2,
        winit::event::MouseButton::Middle => 4,
        winit::event::MouseButton::Back
        | winit::event::MouseButton::Forward
        | winit::event::MouseButton::Other(_) => 0,
    }
}

/// Read the mouse position and button state (framebuffer pixels).
#[inline]
pub fn read_mouse() -> MouseState {
    *crate::window::shared().mouse.lock().unwrap()
}

/// Read the mouse X position (framebuffer pixels).
#[inline]
pub fn read_mouse_x() -> usize {
    read_mouse().x
}

/// Read the mouse Y position (framebuffer pixels).
#[inline]
pub fn read_mouse_y() -> usize {
    read_mouse().y
}

/// Read the mouse buttons bitmask (bit 0=left, 1=right, 2=middle).
#[inline]
pub fn read_mouse_buttons() -> u32 {
    read_mouse().buttons
}
