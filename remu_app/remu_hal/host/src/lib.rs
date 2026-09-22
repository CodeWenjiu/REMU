//! Hosted (std) backend for `remu_hal` — the counterpart of `remu_hal_embedded`.
//!
//! Mirrors the embedded API surface so apps compile unchanged on host targets:
//!
//! - `exit` — process exit via `std::process::exit`.
//! - `time` — millisecond `mtime` (`MTIME_TICK_HZ = 1000`).
//! - `display` — heap framebuffer (`fb_base`, `put_pixel`) + display accessors.
//! - `mouse` / `keyboard` — input-state accessors.
//! - `window` — lazily-started winit/softbuffer event-loop thread that blits
//!   the framebuffer and feeds display/mouse/keyboard state.
//!
//! Output goes through `remu_hal::println!` (writes to std stdout); there is
//! no device object — see `remu_hal::print`.
//!
//! The façade `remu_hal` re-exports these items on host targets; apps depend
//! only on `remu_hal`.

remu_macro::mod_prv!(display, exit, keyboard, mouse, time, window);

pub use display::{
    DisplaySize, FB_HEIGHT, FB_WIDTH, display_alive, fb_base, frame_done, put_pixel, read_disp_h,
    read_disp_size, read_disp_w,
};
pub use exit::{exit_failure, exit_success};
pub use keyboard::{
    KeyState, read_key, read_key_buttons, read_key_code, read_key_down, read_key_kind_raw,
    read_key_seq, read_key_text, read_key_valid,
};
pub use mouse::{MouseState, read_mouse, read_mouse_buttons, read_mouse_x, read_mouse_y};
pub use time::{MTIME_TICK_HZ, read_mtime};
