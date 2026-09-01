//! Mouse device: exposes the shared window's pointer position and buttons as
//! MMIO read registers. The shared state is written by the window render thread.

use std::backtrace::Backtrace;
use std::sync::{Arc, Mutex};

use crate::bus::BusError;
use crate::bus::device::DeviceContext;

/// MMIO register offsets for the mouse device.
pub(super) const REG_MOUSE_X: usize = 0; // read: mouse x (framebuffer px)
pub(super) const REG_MOUSE_Y: usize = 4; // read: mouse y
pub(super) const REG_MOUSE_BUTTONS: usize = 8; // read: bitmask (1=left,2=right,4=mid)
pub(super) const MOUSE_REG_SIZE: usize = 12;

/// Shared mouse position + buttons (render thread writes, this device reads).
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct MouseState {
    pub x: i32,
    pub y: i32,
    pub buttons: u32,
    /// Set once the mouse has moved (distinguishes "no mouse yet" from (0,0)).
    pub valid: bool,
}

pub(super) struct Mouse {
    mouse: Arc<Mutex<MouseState>>,
}

impl Mouse {
    pub(super) fn new() -> Self {
        Self {
            mouse: Arc::new(Mutex::new(MouseState::default())),
        }
    }

    fn read_mouse_x(&self) -> u32 {
        self.mouse.lock().unwrap().x as u32
    }
    fn read_mouse_y(&self) -> u32 {
        self.mouse.lock().unwrap().y as u32
    }
    fn read_mouse_buttons(&self) -> u32 {
        self.mouse.lock().unwrap().buttons
    }
}

impl super::DeviceAccess for Mouse {
    fn name(&self) -> &str {
        "mouse"
    }

    fn size(&self) -> usize {
        MOUSE_REG_SIZE
    }

    fn attach_context(&mut self, ctx: &DeviceContext) {
        self.mouse = Arc::clone(ctx.window().mouse());
    }

    fn read_8(&mut self, offset: usize) -> Result<u8, BusError> {
        Ok(match offset {
            _ if (REG_MOUSE_X..REG_MOUSE_X + 4).contains(&offset) => {
                (self.read_mouse_x() >> (8 * (offset - REG_MOUSE_X))) as u8
            }
            _ if (REG_MOUSE_Y..REG_MOUSE_Y + 4).contains(&offset) => {
                (self.read_mouse_y() >> (8 * (offset - REG_MOUSE_Y))) as u8
            }
            _ if (REG_MOUSE_BUTTONS..REG_MOUSE_BUTTONS + 4).contains(&offset) => {
                (self.read_mouse_buttons() >> (8 * (offset - REG_MOUSE_BUTTONS))) as u8
            }
            _ => 0,
        })
    }

    fn read_32(&mut self, offset: usize) -> Result<u32, BusError> {
        match offset {
            REG_MOUSE_X => Ok(self.read_mouse_x()),
            REG_MOUSE_Y => Ok(self.read_mouse_y()),
            REG_MOUSE_BUTTONS => Ok(self.read_mouse_buttons()),
            _ => Err(BusError::UnsupportedAccessWidth(32, Backtrace::capture())),
        }
    }
}
