//! NES controller input from the remu keyboard device.
//!
//! Maps keyboard keys to NES joypad buttons:
//!   A (Z), B (X), Select (Q), Start (W), D-Pad (arrow keys).
//!
//! `read_key()` returns the last event's physical code + press/release + text
//! char. We track a sticky bitmask so a held key stays set until released.

use core::cell::Cell;

use remu_hal::read_key;
use runes_core::controller::InputPoller;
use runes_core::controller::stdctl;

/// winit `PhysicalKey::Code` discriminants for the arrow keys (stable layout).
const KEY_ARROW_DOWN: u32 = 79;
const KEY_ARROW_LEFT: u32 = 80;
const KEY_ARROW_RIGHT: u32 = 81;
const KEY_ARROW_UP: u32 = 82;

/// A NES joypad poller backed by the keyboard device's last-event state.
///
/// Uses interior mutability (`Cell`) so the NES bus can hold a shared `&`
/// borrow (via `stdctl::Joystick`) while we still update the button state.
pub(crate) struct KeyboardPoller {
    /// Current button bitmask (sticky: set on press, cleared on release).
    buttons: Cell<u8>,
    /// Code/down of the last processed event (to detect repeats).
    last_code: Cell<u32>,
    last_down: Cell<bool>,
}

impl KeyboardPoller {
    pub(crate) fn new() -> Self {
        KeyboardPoller {
            buttons: Cell::new(0),
            last_code: Cell::new(0),
            last_down: Cell::new(false),
        }
    }

    /// Map a key event (code + text) to an NES button bit, or 0.
    fn map(code: u32, text: u32) -> u8 {
        match text as u8 as char {
            'z' | 'Z' => stdctl::A,
            'x' | 'X' => stdctl::B,
            'q' | 'Q' => stdctl::SELECT,
            'w' | 'W' => stdctl::START,
            _ => match code {
                KEY_ARROW_UP => stdctl::UP,
                KEY_ARROW_DOWN => stdctl::DOWN,
                KEY_ARROW_LEFT => stdctl::LEFT,
                KEY_ARROW_RIGHT => stdctl::RIGHT,
                _ => 0,
            },
        }
    }

    /// Process the latest keyboard event into the sticky bitmask.
    pub(crate) fn update(&self) {
        let key = read_key();
        if !key.valid {
            return;
        }
        if key.code == self.last_code.get() && key.down == self.last_down.get() {
            return;
        }
        self.last_code.set(key.code);
        self.last_down.set(key.down);
        let bit = Self::map(key.code, key.text);
        if bit != 0 {
            let mut b = self.buttons.get();
            if key.down {
                b |= bit;
            } else {
                b &= !bit;
            }
            self.buttons.set(b);
        }
    }
}

impl InputPoller for KeyboardPoller {
    fn poll(&self) -> u8 {
        self.buttons.get()
    }
}
