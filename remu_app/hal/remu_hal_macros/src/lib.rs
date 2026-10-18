//! Platform-adaptive entry-point attribute for remu apps.
//!
//! `#[remu_hal::entry]` on `fn main` removes all target plumbing from app
//! code — one `main` body runs on every platform:
//!
//! - **bare-metal targets** (`target_os = "none"`, remu/spike/qemu riscv):
//!   delegated to `riscv_rt::entry` (via the `remu_hal::rt_entry`
//!   re-export), producing the bare-metal `_start` entry.
//! - **rcore** (`target_os = "rcore"`, U-mode user programs under
//!   `rcore_kernel`): emitted as `#[unsafe(no_mangle)] extern "C" fn main()
//!   -> i32`, which `remu_hal_rcore`'s `_start` calls and `sys_exit`s. The
//!   original `-> !` body never returns, so the value is unreachable.
//! - **hosted targets** (`any(unix, windows)`): emitted unchanged in spirit
//!   as a plain `std` `fn main`.
//!
//! All three copies share one renamed inner function (`__remu_app_main`); the
//! `cfg`s are mutually exclusive, so exactly one entry survives per build.
//! The `cfg` is evaluated before macro expansion, so the `::remu_hal::rt_entry`
//! path is only ever resolved on bare-metal targets.

use proc_macro::TokenStream;
use quote::quote;
use syn::{Ident, ItemFn, parse_macro_input};

#[proc_macro_attribute]
pub fn entry(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let f = parse_macro_input!(item as ItemFn);
    let inner_ident = f.sig.ident.clone();
    let inner = Ident::new("__remu_app_main", inner_ident.span());
    // Rename the user's `fn main` so the generated per-platform entries can
    // call it (Rust forbids two `fn main` in one crate).
    let mut inner_fn = f;
    inner_fn.sig.ident = inner.clone();

    quote! {
        // The user's `fn main` body: defined once, called by every platform
        // entry below (bare-metal via riscv_rt, rcore via `_start`'s `call
        // main`, host via the std `main`).
        #inner_fn

        #[cfg(target_os = "none")]
        #[::remu_hal::rt_entry]
        fn main() -> ! {
            #inner()
        }

        #[cfg(target_os = "rcore")]
        #[unsafe(no_mangle)]
        extern "C" fn main() -> i32 {
            #inner()
        }

        #[cfg(any(unix, windows))]
        fn main() {
            #inner()
        }
    }
    .into()
}
