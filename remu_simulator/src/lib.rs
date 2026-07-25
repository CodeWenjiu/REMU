remu_macro::mod_pub!(crate, flow);
remu_macro::mod_prv!(simulator_trait, error, func, stat, platform_config);
remu_macro::mod_pub!(prelude);

pub use error::{
    BreakpointErrorKind, DifftestMismatchList, RefErrorKind, SimulatorError, SimulatorInnerError,
    from_state_error,
};
pub use flow::{SimulatorOption, SimulatorPolicy};
pub use func::{FuncCmd, TraceCmd};
pub use platform_config::PlatformConfig;
pub use simulator_trait::{SimulatorCore, SimulatorDut, SimulatorRef};
pub use stat::{StatCmd, StatContext, StatEntry};
