#![no_std]
// Vendored from Determinant/runes: silence lints that fire on the original
// code but are not worth refactoring in a vendored core.
// - `unreachable_pub`: the `make_optable!`/`make_addrtable!` macros emit
//   `pub const` tables used only inside this crate.
// - `invalid_value`: the mappers use `MaybeUninit::uninit().assume_init()` as
//   a self-referential construction placeholder that is always overwritten
//   before use (formally UB, practically safe).
#![allow(unreachable_pub, invalid_value)]
pub mod memory;
pub mod utils;
#[macro_use]
pub mod mos6502;
pub mod apu;
pub mod cartridge;
pub mod controller;
pub mod mapper;
pub mod ppu;
