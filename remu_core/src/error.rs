use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid quoting in command string")]
    InvalidQuoting,
}

pub(crate) type Result<T> = std::result::Result<T, Error>;
