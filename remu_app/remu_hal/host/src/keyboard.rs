//! Keyboard device on host: shared key state + winit event mapping.
//!
//! ## `key_kind` encoding contract
//!
//! `KeyState::key_kind` carries the logical-key discriminant defined by the
//! façade `remu_hal::KeyKind` and produced by the window host in `remu_state`
//! (the same encoding travels over MMIO on embedded). The discriminant **is
//! the unicode code point Slint's `Key` enum expects** — the mapping below is
//! the host-side producer of that encoding and must stay in sync with
//! `remu_hal::KeyKind` / `remu_state`'s `KeyKind` (None=0, Enter='\n',
//! Escape=0x1B, Up=0xF700, Down=0xF701, Left=0xF702, Right=0xF703, Tab=0x09,
//! Backspace=0x08, Space=0x20, Shift=0x10, Control=0x11, Alt=0x12, Meta=0x17).
//! (The impl cannot live here as `impl From<winit::Key> for remu_hal::KeyKind`
//! because that would make `remu_hal_host` depend on `remu_hal` — a cycle.)

/// Key state (keycode + press/release + text char).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KeyState {
    /// Physical key code (winit `PhysicalKey::Code` value).
    pub code: u32,
    /// Whether the last event was a press (true) or release (false).
    pub down: bool,
    /// Last text character (if printable), else 0.
    pub text: u32,
    /// Logical key kind discriminant for non-printable keys (arrows, Enter,
    /// Escape, ...); 0 for printable keys. The discriminant is the Slint code
    /// point for the key, transferred over MMIO as a u32.
    pub key_kind: u32,
    /// Monotonic event sequence number, incremented on every key event. Lets a
    /// poller distinguish a fresh press of the same key from a stale snapshot.
    pub seq: u32,
    /// Set once a key event has occurred.
    pub valid: bool,
    /// Live NES joypad button bitmask (render thread maintains).
    pub buttons: u8,
}

/// Map a winit logical key to the `key_kind` discriminant (see the module
/// docs for the encoding contract). Printable keys (and anything we don't
/// model) map to 0.
pub(crate) fn key_kind_from_winit(key: winit::keyboard::Key) -> u32 {
    use winit::keyboard::NamedKey;
    match key {
        winit::keyboard::Key::Named(NamedKey::Enter) => 0x0A,
        winit::keyboard::Key::Named(NamedKey::Escape) => 0x1B,
        winit::keyboard::Key::Named(NamedKey::ArrowUp) => 0xF700,
        winit::keyboard::Key::Named(NamedKey::ArrowDown) => 0xF701,
        winit::keyboard::Key::Named(NamedKey::ArrowLeft) => 0xF702,
        winit::keyboard::Key::Named(NamedKey::ArrowRight) => 0xF703,
        winit::keyboard::Key::Named(NamedKey::Tab) => 0x09,
        winit::keyboard::Key::Named(NamedKey::Backspace) => 0x08,
        winit::keyboard::Key::Named(NamedKey::Space) => 0x20,
        winit::keyboard::Key::Named(NamedKey::Shift) => 0x10,
        winit::keyboard::Key::Named(NamedKey::Control) => 0x11,
        winit::keyboard::Key::Named(NamedKey::Alt) => 0x12,
        winit::keyboard::Key::Named(NamedKey::Meta) => 0x17,
        _ => 0,
    }
}

/// Map a winit key event (physical code + text) to a NES joypad button
/// bit, or 0. Mirrors the embedded window host mapping.
pub(crate) fn key_to_button(code: u32, text: u32) -> u8 {
    match text as u8 as char {
        'z' | 'Z' => 1 << 0, // A
        'x' | 'X' => 1 << 1, // B
        'q' | 'Q' => 1 << 2, // SELECT
        'w' | 'W' => 1 << 3, // START
        // Vim-style d-pad (plus physical arrows below).
        'h' | 'H' => 1 << 6, // LEFT
        'j' | 'J' => 1 << 5, // DOWN
        'k' | 'K' => 1 << 4, // UP
        'l' | 'L' => 1 << 7, // RIGHT
        _ => match code {
            82 => 1 << 4, // UP
            79 => 1 << 5, // DOWN
            80 => 1 << 6, // LEFT
            81 => 1 << 7, // RIGHT
            _ => 0,
        },
    }
}

/// Read the keyboard state (keycode + press/release + text).
#[inline]
pub fn read_key() -> KeyState {
    *crate::window::shared().keyboard.lock().unwrap()
}

/// Read the last key code (physical position).
#[inline]
pub fn read_key_code() -> u32 {
    read_key().code
}

/// Read whether the last key event was a press (1) or release (0).
#[inline]
pub fn read_key_down() -> u32 {
    read_key().down as u32
}

/// Read the last text character (ASCII), or 0 if non-printable.
#[inline]
pub fn read_key_text() -> u32 {
    read_key().text
}

/// Read the logical key kind discriminant of the last key event (0 for
/// printable keys).
#[inline]
pub fn read_key_kind_raw() -> u32 {
    read_key().key_kind
}

/// Read the monotonic key event sequence number.
#[inline]
pub fn read_key_seq() -> u32 {
    read_key().seq
}

/// Read whether any key event has occurred yet (1) or not (0).
#[inline]
pub fn read_key_valid() -> u32 {
    read_key().valid as u32
}

/// Read the live NES joypad button bitmask.
#[inline]
pub fn read_key_buttons() -> u32 {
    read_key().buttons as u32
}
