//! Hardware abstraction for remu apps — works on both RISC-V and host.
//!
//! On embedded RISC-V, delegates to [`remu_hal_embedded`]. On host, provides
//! stdout-based equivalents with the same API.
//!
//! # Portable app pattern
//!
//! ```ignore
//! #![cfg_attr(target_arch = "riscv32", no_std, no_main)]
//! extern crate alloc;
//!
//! #[cfg_attr(target_arch = "riscv32", remu_hal::entry)]
//! fn main() -> ! {
//!     remu_hal::init();
//!     let mut uart = remu_hal::Uart16550::default_base();
//!     let _ = writeln!(uart, "hello");
//!     let _ = remu_hal::Write::flush(&mut uart);
//!     remu_hal::exit_success()
//! }
//! ```

#![cfg_attr(any(target_arch = "riscv32", target_arch = "riscv64"), no_std)]

extern crate alloc;

// ── Platform-specific backend ──
// ── Platform-specific backend ──
#[cfg(not(any(target_arch = "riscv32", target_arch = "riscv64")))]
mod host;

// ── Re-exports (all platforms) ──
remu_macro::mod_prv!(print);
pub use alloc::{boxed::Box, string::String, vec::Vec};
pub use core::fmt::Write as FmtWrite;
pub use print::write_fmt;

/// Logical key kind, UI-framework-agnostic.
///
/// The window host maps a raw key event to a [`KeyKind`] describing *what* was
/// pressed. The discriminant **is the unicode code point Slint's `Key` enum
/// expects** (see `slint::platform::Key`), so converting a `KeyKind` to the
/// code point Slint wants is just `kind as u32` — no mapping table. This is
/// also the value transferred over MMIO (u32), so embedded and host agree on
/// the same encoding. Printable keys carry their character in
/// `KeyState::text` and report [`KeyKind::None`].
///
/// The discriminant order must stay in sync with the `KeyKind` in
/// `remu_state` (producer side).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum KeyKind {
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

/// Map a `KeyKind` discriminant (a code point) back to the enum. Unknown values
/// map to [`KeyKind::None`].
#[inline]
pub fn key_kind_from_u32(v: u32) -> KeyKind {
    match v {
        x if x == KeyKind::None as u32 => KeyKind::None,
        x if x == KeyKind::Enter as u32 => KeyKind::Enter,
        x if x == KeyKind::Escape as u32 => KeyKind::Escape,
        x if x == KeyKind::Up as u32 => KeyKind::Up,
        x if x == KeyKind::Down as u32 => KeyKind::Down,
        x if x == KeyKind::Left as u32 => KeyKind::Left,
        x if x == KeyKind::Right as u32 => KeyKind::Right,
        x if x == KeyKind::Tab as u32 => KeyKind::Tab,
        x if x == KeyKind::Backspace as u32 => KeyKind::Backspace,
        x if x == KeyKind::Space as u32 => KeyKind::Space,
        x if x == KeyKind::Shift as u32 => KeyKind::Shift,
        x if x == KeyKind::Control as u32 => KeyKind::Control,
        x if x == KeyKind::Alt as u32 => KeyKind::Alt,
        x if x == KeyKind::Meta as u32 => KeyKind::Meta,
        _ => KeyKind::None,
    }
}

#[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
pub use embedded_io::Write;

/// Platform-adaptive entry point: on riscv targets this becomes
/// `riscv_rt::entry` (bare-metal `_start`); on host targets the function is
/// passed through untouched as a plain `std main`.
pub use remu_hal_macros::entry;

#[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
pub use remu_hal_embedded::{
    MTIME_TICK_HZ, Uart16550, app_args, exit_failure, exit_success, read_mtime,
};

/// riscv-only entry delegate used by [`entry`]'s cfg-guarded riscv copy.
#[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
pub use remu_hal_embedded::entry as rt_entry;

#[cfg(not(any(target_arch = "riscv32", target_arch = "riscv64")))]
pub use host::{MTIME_TICK_HZ, Stdout as Uart16550, read_mtime};

// ── Display device (both platforms; host returns no-op stubs) ──
#[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
pub use remu_hal_embedded::{
    DisplaySize, FB_BASE, FB_HEIGHT, FB_WIDTH, KeyState, MouseState, display_alive, fb_base,
    frame_done, put_pixel, read_disp_h, read_disp_size, read_disp_w, read_key, read_key_buttons,
    read_key_code, read_key_down, read_key_kind_raw, read_key_seq, read_key_text, read_key_valid,
    read_mouse, read_mouse_buttons, read_mouse_x, read_mouse_y,
};

#[cfg(not(any(target_arch = "riscv32", target_arch = "riscv64")))]
pub use host::{
    DisplaySize, FB_HEIGHT, FB_WIDTH, KeyState, MouseState, display_alive, fb_base, frame_done,
    put_pixel, read_disp_h, read_disp_size, read_disp_w, read_key, read_key_buttons, read_key_code,
    read_key_down, read_key_kind_raw, read_key_seq, read_key_text, read_key_valid, read_mouse,
    read_mouse_buttons, read_mouse_x, read_mouse_y,
};

/// Read the logical key kind of the last key event, mapped to [`KeyKind`].
#[inline]
pub fn read_key_kind() -> KeyKind {
    key_kind_from_u32(read_key_kind_raw())
}

// ── Safe init (single implementation; embedded bring-up, host no-op) ──
/// Performs platform bring-up. On embedded targets this wraps the unsafe
/// heap/UART init from `remu_hal_embedded`; on host targets std has already
/// set everything up, so the whole body compiles away to a no-op.
#[inline]
pub fn init() {
    #[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
    unsafe {
        remu_hal_embedded::init()
    };
}

// ── Exit (both platforms) ──
#[cfg(not(any(target_arch = "riscv32", target_arch = "riscv64")))]
pub fn exit_success() -> ! {
    std::process::exit(0);
}
#[cfg(not(any(target_arch = "riscv32", target_arch = "riscv64")))]
pub fn exit_failure() -> ! {
    std::process::exit(1);
}
