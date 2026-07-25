remu_macro::mod_prv!(
    dpi,
    link_args,
    nzea_ffi,
    nzea_target,
    runtime,
    simulator_trait,
    supported_isa,
    watchdog,
);

// Public API
pub use link_args::emit_linker_args;
pub use simulator_trait::SimulatorNzea;
pub use supported_isa::NzeaIsaKind;

// Internal cross-module access
pub(crate) use dpi::{CommitMsg, NzeaDpi, clear_nzea, set_nzea};
pub(crate) use nzea_ffi::{NzeaFns, NzeaIsa};
pub(crate) use nzea_target::NzeaTarget;
pub(crate) use runtime::{ensure_nzea_loaded, get_nzea_fns};
pub(crate) use watchdog::Watchdog;
