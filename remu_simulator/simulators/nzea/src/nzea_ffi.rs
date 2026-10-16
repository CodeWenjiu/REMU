//! FFI bindings: function pointers loaded from libnzea.so at runtime.
//! Each (platform, isa) combination loads its own .so.

use std::ffi::{c_char, c_void};

use dlopen2::wrapper::WrapperApi;

use remu_isa::isa::extension_enum::{RV32I, RV32IM, RV64I, RV64IM};

/// Callback invoked by `nzea_iter_stats` for each stat_* counter: the leaf name
/// (NUL-terminated), the raw VPI words (`lo` first) and the signal's actual
/// width in bits. See [`assemble_stat_value`].
pub(crate) type NzeaStatCallback =
    unsafe extern "C" fn(name: *const c_char, lo: u32, hi: u32, width: u32, userdata: *mut c_void);

/// Assemble a counter value from the raw VPI words: `lo` holds the
/// least-significant 32 bits, `hi` the next word (0 for signals 32 bits or
/// narrower), `width` is the signal's actual width in bits. The value is
/// zero-extended to `width`.
pub(crate) fn assemble_stat_value(lo: u32, hi: u32, width: u32) -> u64 {
    let raw = u64::from(lo) | (u64::from(hi) << 32);
    if width >= 64 {
        raw
    } else {
        raw & ((1u64 << width) - 1)
    }
}

// ---------------------------------------------------------------------------
// Function table: the field (method) names are the short names used by the
// simulator; `#[dlopen2_name]` maps them to the exported C symbols. Loading
// happens once in `Container::load` (see runtime.rs).
// ---------------------------------------------------------------------------

#[derive(WrapperApi)]
pub(crate) struct NzeaApi {
    #[dlopen2_name = "nzea_create"]
    create: unsafe extern "C" fn(model: *const i8) -> *mut c_void,

    #[dlopen2_name = "nzea_destroy"]
    destroy: unsafe extern "C" fn(sim: *mut c_void, model: *const i8),

    #[dlopen2_name = "nzea_set_clock"]
    set_clock: unsafe extern "C" fn(sim: *mut c_void, model: *const i8, val: i32),

    #[dlopen2_name = "nzea_set_reset"]
    set_reset: unsafe extern "C" fn(sim: *mut c_void, model: *const i8, val: i32),

    #[dlopen2_name = "nzea_eval"]
    eval: unsafe extern "C" fn(sim: *mut c_void, model: *const i8),

    #[dlopen2_name = "nzea_trace_open"]
    trace_open: unsafe extern "C" fn(sim: *mut c_void, model: *const i8, filename: *const i8),

    #[dlopen2_name = "nzea_trace_dump"]
    trace_dump: unsafe extern "C" fn(sim: *mut c_void),

    #[dlopen2_name = "nzea_iter_stats"]
    iter_stats: unsafe extern "C" fn(
        sim: *mut c_void,
        cb: Option<NzeaStatCallback>,
        userdata: *mut c_void,
    ) -> i32,
}

// ---------------------------------------------------------------------------
// ISA string mapping
// ---------------------------------------------------------------------------

/// ISA string for nzea DPI; must match nzea `just dump --isa <str>`. The
/// associated type bound ties the ISA's XLEN to the machine words the DPI
/// boundary has a channel for (`u32`/`u64`): an ISA whose XLEN is neither is
/// rejected when it tries to implement this trait, not at runtime.
pub trait NzeaIsa: remu_isa::isa::RvIsa<XLEN: crate::dpi::DpiChannel> {
    const NZEA_ISA_STR: &'static str;
}

impl NzeaIsa for RV32I {
    const NZEA_ISA_STR: &'static str = "riscv32i";
}

impl NzeaIsa for RV32IM {
    const NZEA_ISA_STR: &'static str = "riscv32im";
}

impl NzeaIsa for RV64I {
    const NZEA_ISA_STR: &'static str = "riscv64i";
}

impl NzeaIsa for RV64IM {
    const NZEA_ISA_STR: &'static str = "riscv64im";
}

#[cfg(test)]
mod tests {
    use super::assemble_stat_value;

    #[test]
    fn assembles_vpi_words_at_actual_width() {
        // Up to 32 bits only the low word is meaningful; the high word is
        // ignored even if the RTL leaves something in it.
        assert_eq!(assemble_stat_value(0xDEAD_BEEF, 0, 32), 0xDEAD_BEEF);
        assert_eq!(assemble_stat_value(0x1234, 0xFFFF, 32), 0x1234);
        // Wider signals: the high word carries bits 32..
        assert_eq!(assemble_stat_value(0xFFFF_FFFF, 0x1, 33), 0x1_FFFF_FFFF);
        assert_eq!(
            assemble_stat_value(0xFFFF_FFFF, 0xFFFF, 48),
            0x0000_FFFF_FFFF_FFFF
        );
        assert_eq!(
            assemble_stat_value(0, 0xFFFF_FFFF, 64),
            0xFFFF_FFFF_0000_0000
        );
        assert_eq!(assemble_stat_value(0xFFFF_FFFF, 0xFFFF_FFFF, 64), u64::MAX);
    }
}
