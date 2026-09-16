//! Platform-adaptive entry-point attribute for remu apps.
//!
//! `#[remu_hal::entry]` on `fn main` removes all target-arch plumbing from app
//! code:
//!
//! - **riscv32 / riscv64 targets**: the function is delegated to
//!   `riscv_rt::entry` (via the `remu_hal::rt_entry` re-export), producing the
//!   bare-metal `_start` entry.
//! - **host targets**: the function is emitted unchanged as a plain `std`
//!   `fn main`.
//!
//! Both copies are guarded by mutually exclusive `#[cfg]`s, so exactly one
//! survives per build; the `cfg` is evaluated before macro expansion, so the
//! `::remu_hal::rt_entry` path is only ever resolved on riscv targets.

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn entry(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = item.to_string();
    format!(
        "#[cfg(any(target_arch = \"riscv32\", target_arch = \"riscv64\"))]\n\
         #[::remu_hal::rt_entry]\n\
         {item}\n\
         #[cfg(not(any(target_arch = \"riscv32\", target_arch = \"riscv64\")))]\n\
         {item}"
    )
    .parse()
    .expect("generated tokens are valid")
}
