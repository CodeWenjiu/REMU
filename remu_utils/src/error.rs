use snafu::Snafu;

#[derive(Debug, Snafu)]
pub enum ProcessError {
    Recoverable,
    GracefulExit,
    Fatal,
} 

pub type ProcessResult<T> = Result<T, ProcessError>;
