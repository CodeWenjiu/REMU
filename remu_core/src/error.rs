use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid quoting in command string")]
    InvalidQuoting,

    #[error("Command expression parse error: {0}")]
    CommandExpr(String),

    #[error("Command expression parse error (handled)")]
    CommandExprHandled,
}

pub(crate) type Result<T> = std::result::Result<T, Error>;
