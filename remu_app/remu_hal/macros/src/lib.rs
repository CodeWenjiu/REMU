//! Platform-adaptive entry-point attribute for remu apps.
//!
//! `#[remu_hal::entry]` on `fn main` removes all target plumbing from app
//! code:
//!
//! - **bare-metal targets** (`target_os = "none"`, e.g. remu's riscv32/riscv64):
//!   the function is delegated to `riscv_rt::entry` (via the
//!   `remu_hal::rt_entry` re-export), producing the bare-metal `_start` entry.
//! - **hosted targets** (linux, macos, …, including a RISC-V host): the
//!   function is emitted unchanged as a plain `std` `fn main`.
//!
//! Both copies are guarded by mutually exclusive `#[cfg]`s, so exactly one
//! survives per build; the `cfg` is evaluated before macro expansion, so the
//! `::remu_hal::rt_entry` path is only ever resolved on bare-metal targets.

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn entry(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = item.to_string();
    format!(
        "#[cfg(target_os = \"none\")]\n\
         #[::remu_hal::rt_entry]\n\
         {item}\n\
         #[cfg(not(target_os = \"none\"))]\n\
         {item}"
    )
    .parse()
    .expect("generated tokens are valid")
}
