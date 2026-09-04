//! Keyboard device: exposes the shared window's key state as MMIO read
//! registers. The shared state is written by the window render thread.

use std::backtrace::Backtrace;
use std::sync::{Arc, Mutex};

use crate::bus::BusError;
use crate::bus::device::DeviceContext;

/// UI-agnostic logical key kind, written to the keyboard MMIO registers as a
/// u32 discriminant.
///
/// The discriminant **is the unicode code point Slint's `Key` enum expects**
/// (see `slint::platform::Key`), and must stay in sync with `remu_hal::KeyKind`
/// (the app-side HAL decodes the register with the same mapping): None=0,
/// Enter=`\n`(0x0a), Escape=0x1b, Up=0xF700, Down=0xF701, Left=0xF702,
/// Right=0xF703, Tab=0x09, Backspace=0x08, Space=0x20, Shift=0x10,
/// Control=0x11, Alt=0x12, Meta=0x17.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub(super) enum KeyKind {
    /// Printable key (see `text`) or a key we don't model.
    #[default]
    None = 0,
    /// Slint `Key::Return` = `\n`.
    Enter = '\n' as u32,
    /// Slint `Key::Escape`.
    Escape = 0x1B,
    /// Slint `Key::UpArrow`.
    Up = 0xF700,
    /// Slint `Key::DownArrow`.
    Down = 0xF701,
    /// Slint `Key::LeftArrow`.
    Left = 0xF702,
    /// Slint `Key::RightArrow`.
    Right = 0xF703,
    /// Slint `Key::Tab`.
    Tab = 0x09,
    /// Slint `Key::Backspace`.
    Backspace = 0x08,
    /// Slint `Key::Space`.
    Space = 0x20,
    /// Slint `Key::Shift`.
    Shift = 0x10,
    /// Slint `Key::Control`.
    Control = 0x11,
    /// Slint `Key::Alt`.
    Alt = 0x12,
    /// Slint `Key::Meta`.
    Meta = 0x17,
}

impl From<winit::keyboard::Key> for KeyKind {
    fn from(key: winit::keyboard::Key) -> Self {
        use winit::keyboard::NamedKey;
        match key {
            winit::keyboard::Key::Named(NamedKey::Enter) => KeyKind::Enter,
            winit::keyboard::Key::Named(NamedKey::Escape) => KeyKind::Escape,
            winit::keyboard::Key::Named(NamedKey::ArrowUp) => KeyKind::Up,
            winit::keyboard::Key::Named(NamedKey::ArrowDown) => KeyKind::Down,
            winit::keyboard::Key::Named(NamedKey::ArrowLeft) => KeyKind::Left,
            winit::keyboard::Key::Named(NamedKey::ArrowRight) => KeyKind::Right,
            winit::keyboard::Key::Named(NamedKey::Tab) => KeyKind::Tab,
            winit::keyboard::Key::Named(NamedKey::Backspace) => KeyKind::Backspace,
            winit::keyboard::Key::Named(NamedKey::Space) => KeyKind::Space,
            winit::keyboard::Key::Named(NamedKey::Shift) => KeyKind::Shift,
            winit::keyboard::Key::Named(NamedKey::Control) => KeyKind::Control,
            winit::keyboard::Key::Named(NamedKey::Alt) => KeyKind::Alt,
            winit::keyboard::Key::Named(NamedKey::Meta) => KeyKind::Meta,
            _ => KeyKind::None,
        }
    }
}

/// MMIO register offsets for the keyboard device.
pub(super) const REG_KEY_CODE: usize = 0; // read: last key (winit PhysicalKey::Code)
pub(super) const REG_KEY_DOWN: usize = 4; // read: 1 = pressed, 0 = released
pub(super) const REG_KEY_TEXT: usize = 8; // read: last text char (ASCII) or 0
pub(super) const REG_KEY_VALID: usize = 12; // read: 1 = a key event has occurred
pub(super) const REG_KEY_BUTTONS: usize = 16; // read: live NES joypad button bitmask
pub(super) const REG_KEY_NAMED: usize = 20; // read: KeyKind discriminant (u32)
pub(super) const REG_KEY_SEQ: usize = 24; // read: monotonic event sequence
pub(super) const KEYBOARD_REG_SIZE: usize = 28;

/// Shared keyboard state (render thread writes, this device reads).
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct KeyboardState {
    /// Physical key code (winit `PhysicalKey::Code` value).
    pub code: u32,
    /// Whether the last event was a press (true) or release (false).
    pub down: bool,
    /// Last text character (if printable), else 0.
    pub text: u32,
    /// Logical key kind discriminant for non-printable keys (see [`KeyKind`]).
    pub key_kind: u32,
    /// Monotonic event sequence number, incremented on every key event.
    pub seq: u32,
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
    fn read_key_kind(&self) -> u32 {
        self.kb.lock().unwrap().key_kind
    }
    fn read_seq(&self) -> u32 {
        self.kb.lock().unwrap().seq
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
            _ if (REG_KEY_NAMED..REG_KEY_NAMED + 4).contains(&offset) => {
                (self.read_key_kind() >> (8 * (offset - REG_KEY_NAMED))) as u8
            }
            _ if (REG_KEY_SEQ..REG_KEY_SEQ + 4).contains(&offset) => {
                (self.read_seq() >> (8 * (offset - REG_KEY_SEQ))) as u8
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
            REG_KEY_NAMED => Ok(self.read_key_kind()),
            REG_KEY_SEQ => Ok(self.read_seq()),
            REG_KEY_VALID => Ok(self.read_valid()),
            REG_KEY_BUTTONS => Ok(self.read_buttons()),
            _ => Err(BusError::UnsupportedAccessWidth(32, Backtrace::capture())),
        }
    }
}
