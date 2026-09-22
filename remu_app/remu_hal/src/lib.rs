//! Hardware abstraction for remu apps — works on every target platform.
//!
//! On bare-metal targets (`target_os = "none"`, e.g. remu's riscv32/riscv64)
//! it delegates to [`remu_hal_embedded`]. On hosted targets (`any(unix, windows)`:
//! linux, macos, windows, BSDs, …, including a RISC-V host) it delegates to
//! [`remu_hal_host`] (stdout, a window-backed display, mouse/keyboard). On
//! rcore (`target_os = "rcore"`, U-mode programs under `rcore_kernel`) it
//! delegates to [`remu_hal_rcore`] (syscall-based stdout/exit) — same API on
//! all three. Arms are selected by **positive** predicates; adding another
//! environment is purely additive.
//!
//! # Portable app pattern
//!
//! ```ignore
//! use remu_hal::exit_success;
//!
//! #[remu_hal::entry]
//! fn main() -> ! {
//!     remu_hal::init();
//!     remu_hal::println!("hello");
//!     remu_hal::exit_success()
//! }
//! ```

#![cfg_attr(any(target_os = "none", target_os = "rcore"), no_std)]
// `target_os = "rcore"` comes from the custom rcore64.json target (remu_app/rcore/…);
// rustc has no built-in knowledge of that os value.
#![allow(unexpected_cfgs)]

// `extern crate alloc` + alloc re-exports: not available on rcore (U-mode
// programs have no heap allocator; the kernel has no alloc syscall yet).
#[cfg(not(target_os = "rcore"))]
extern crate alloc;

// ── Re-exports (all platforms) ──
remu_macro::mod_prv!(print);
#[cfg(not(target_os = "rcore"))]
pub use alloc::{boxed::Box, string::String, vec::Vec};
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

/// Platform-adaptive entry point: on bare-metal targets (`target_os = "none"`)
/// this becomes `riscv_rt::entry` (bare-metal `_start`); on hosted targets
/// the function is passed through untouched as a plain `std main`; on rcore
/// it becomes the U-mode entry called by `remu_hal_rcore`'s `_start`.
pub use remu_hal_macros::entry;

#[cfg(target_os = "none")]
pub use remu_hal_embedded::{MTIME_TICK_HZ, app_args, exit_failure, exit_success, read_mtime};

/// Bare-metal entry delegate used by [`entry`]'s cfg-guarded embedded copy.
#[cfg(target_os = "none")]
pub use remu_hal_embedded::entry as rt_entry;

#[cfg(any(unix, windows))]
pub use remu_hal_host::{MTIME_TICK_HZ, read_mtime};

/// rcore arm (U-mode user programs): the same API, served by syscalls. No
/// display/input devices until the kernel grows those syscalls.
#[cfg(target_os = "rcore")]
pub use remu_hal_rcore::{exit_failure, exit_success};

// ── Display device (both backends; the hosted one uses a real window) ──
#[cfg(target_os = "none")]
pub use remu_hal_embedded::{
    DisplaySize, FB_BASE, FB_HEIGHT, FB_WIDTH, KeyState, MouseState, display_alive, fb_base,
    frame_done, put_pixel, read_disp_h, read_disp_size, read_disp_w, read_key, read_key_buttons,
    read_key_code, read_key_down, read_key_kind_raw, read_key_seq, read_key_text, read_key_valid,
    read_mouse, read_mouse_buttons, read_mouse_x, read_mouse_y,
};

#[cfg(any(unix, windows))]
pub use remu_hal_host::{
    DisplaySize, FB_HEIGHT, FB_WIDTH, KeyState, MouseState, display_alive, fb_base, frame_done,
    put_pixel, read_disp_h, read_disp_size, read_disp_w, read_key, read_key_buttons, read_key_code,
    read_key_down, read_key_kind_raw, read_key_seq, read_key_text, read_key_valid, read_mouse,
    read_mouse_buttons, read_mouse_x, read_mouse_y,
};

/// Read the logical key kind of the last key event, mapped to [`KeyKind`].
///
/// Not available on rcore (user programs have no MMIO keyboard; they can only
/// use syscalls).
#[cfg(any(target_os = "none", unix, windows))]
#[inline]
pub fn read_key_kind() -> KeyKind {
    key_kind_from_u32(read_key_kind_raw())
}

// ── Safe init (single implementation; bare-metal bring-up, host no-op) ──
/// Performs platform bring-up. On bare-metal targets this wraps the unsafe
/// heap/UART init from `remu_hal_embedded`; on hosted targets std has already
/// set everything up, so the whole body compiles away to a no-op.
#[inline]
pub fn init() {
    #[cfg(target_os = "none")]
    unsafe {
        remu_hal_embedded::init()
    };
}

// ── Exit (hosted: process exit; the bare-metal counterparts live in remu_hal_embedded) ──
#[cfg(any(unix, windows))]
pub use remu_hal_host::{exit_failure, exit_success};
