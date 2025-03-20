remu_macro::mod_flat!(simple_debugger, cmd_impl);
remu_macro::mod_pub!(cmd_parser);
use snafu::Snafu;

#[derive(Debug, Snafu)]
pub enum ProcessError {
    Recoverable,
    GracefulExit,
    Fatal,
} 

pub type ProcessResult<T> = Result<T, ProcessError>;
