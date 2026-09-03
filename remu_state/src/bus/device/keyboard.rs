//! Keyboard device: exposes the shared window's key state as MMIO read
//! registers. The shared state is written by the window render thread.

use std::backtrace::Backtrace;
use std::sync::{Arc, Mutex};

use crate::bus::BusError;
use crate::bus::device::DeviceContext;

/// MMIO register offsets for the keyboard device.
pub(super) const REG_KEY_CODE: usize = 0; // read: last key (winit PhysicalKey::Code)
pub(super) const REG_KEY_DOWN: usize = 4; // read: 1 = pressed, 0 = released
pub(super) const REG_KEY_TEXT: usize = 8; // read: last text char (ASCII) or 0
pub(super) const REG_KEY_VALID: usize = 12; // read: 1 = a key event has occurred
pub(super) const REG_KEY_BUTTONS: usize = 16; // read: live NES joypad button bitmask
pub(super) const KEYBOARD_REG_SIZE: usize = 20;

/// Shared keyboard state (render thread writes, this device reads).
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct KeyboardState {
    /// Physical key code (winit `PhysicalKey::Code` value).
    pub code: u32,
    /// Whether the last event was a press (true) or release (false).
    pub down: bool,
    /// Last text character (if printable), else 0.
    pub text: u32,
    /// Set once a key event has occurred.
    pub valid: bool,
    /// Live NES joypad button bitmask (render thread maintains; press sets,
    /// release clears). This is the *current held* state, so slow frame rates
    /// don't drop quick taps that are still held when the app polls.
    pub buttons: u8,
}

pub(super) struct Keyboard {
    kb: Arc<Mutex<KeyboardState>>,
}

impl Keyboard {
    pub(super) fn new() -> Self {
        Self {
            kb: Arc::new(Mutex::new(KeyboardState::default())),
        }
    }

    fn read_code(&self) -> u32 {
        self.kb.lock().unwrap().code
    }
    fn read_down(&self) -> u32 {
        self.kb.lock().unwrap().down as u32
    }
    fn read_text(&self) -> u32 {
        self.kb.lock().unwrap().text
    }
    fn read_valid(&self) -> u32 {
        self.kb.lock().unwrap().valid as u32
    }
    fn read_buttons(&self) -> u32 {
        self.kb.lock().unwrap().buttons as u32
    }
}

impl super::DeviceAccess for Keyboard {
    fn name(&self) -> &str {
        "keyboard"
    }

    fn size(&self) -> usize {
        KEYBOARD_REG_SIZE
    }

    fn attach_context(&mut self, ctx: &DeviceContext) {
        self.kb = Arc::clone(ctx.window().keyboard());
    }

    fn read_8(&mut self, offset: usize) -> Result<u8, BusError> {
        Ok(match offset {
            _ if (REG_KEY_CODE..REG_KEY_CODE + 4).contains(&offset) => {
                (self.read_code() >> (8 * (offset - REG_KEY_CODE))) as u8
            }
            _ if (REG_KEY_DOWN..REG_KEY_DOWN + 4).contains(&offset) => {
                (self.read_down() >> (8 * (offset - REG_KEY_DOWN))) as u8
            }
            _ if (REG_KEY_TEXT..REG_KEY_TEXT + 4).contains(&offset) => {
                (self.read_text() >> (8 * (offset - REG_KEY_TEXT))) as u8
            }
            _ if (REG_KEY_VALID..REG_KEY_VALID + 4).contains(&offset) => {
                (self.read_valid() >> (8 * (offset - REG_KEY_VALID))) as u8
            }
            _ if (REG_KEY_BUTTONS..REG_KEY_BUTTONS + 4).contains(&offset) => {
                (self.read_buttons() >> (8 * (offset - REG_KEY_BUTTONS))) as u8
            }
            _ => 0,
        })
    }

    fn read_32(&mut self, offset: usize) -> Result<u32, BusError> {
        match offset {
            REG_KEY_CODE => Ok(self.read_code()),
            REG_KEY_DOWN => Ok(self.read_down()),
            REG_KEY_TEXT => Ok(self.read_text()),
            REG_KEY_VALID => Ok(self.read_valid()),
            REG_KEY_BUTTONS => Ok(self.read_buttons()),
            _ => Err(BusError::UnsupportedAccessWidth(32, Backtrace::capture())),
        }
    }
}
