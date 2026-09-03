//! NES controller input from the remu keyboard device.
//!
//! The keyboard device (both embedded MMIO and host) maintains a **live NES
//! joypad button bitmask** in its render thread: press sets the bit, release
//! clears it. `read_key_buttons()` returns that bitmask, so this poller is just
//! a thin read-through — held keys and quick taps are captured reliably even at
//! the low embedded frame rate.

use core::cell::Cell;

use remu_hal::read_key_buttons;
use runes_core::controller::InputPoller;

/// A NES joypad poller backed by the keyboard device's live button bitmask.
///
/// Uses interior mutability (`Cell`) so the NES bus can hold a shared `&`
/// borrow (via `stdctl::Joystick`) while the poller still reads the device.
pub(crate) struct KeyboardPoller {
    /// Current button bitmask (read from the device each poll).
    buttons: Cell<u8>,
}

impl KeyboardPoller {
    pub(crate) fn new() -> Self {
        KeyboardPoller {
            buttons: Cell::new(0),
        }
    }
}

impl InputPoller for KeyboardPoller {
    fn poll(&self) -> u8 {
        // Read the device's live bitmask and cache it in a Cell so `Joystick`
        // (which holds a shared `&` borrow) can read it through `&self`.
        let b = (read_key_buttons() & 0xff) as u8;
        self.buttons.set(b);
        b
    }
}
