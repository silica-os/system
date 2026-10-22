//! A minimal error type: a message, with context added on the way up.

use std::fmt;

/// An error that carries a human readable message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(String);

impl Error {
    pub fn new(message: impl Into<String>) -> Self {
        Error(message.into())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error(e.to_string())
    }
}

/// What a tool's `run` returns on failure: a usage error (exit code 2) or
/// any other error (exit code 1).
#[derive(Debug)]
pub enum Failure {
    Usage(String),
    Error(Error),
}

impl From<Error> for Failure {
    fn from(e: Error) -> Self {
        Failure::Error(e)
    }
}

impl Failure {
    pub fn exit_code(&self) -> u8 {
        match self {
            Failure::Usage(_) => 2,
            Failure::Error(_) => 1,
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// Adds a context message to any displayable error.
pub trait Context<T> {
    fn context(self, message: impl FnOnce() -> String) -> Result<T>;
}

impl<T, E: fmt::Display> Context<T> for std::result::Result<T, E> {
    fn context(self, message: impl FnOnce() -> String) -> Result<T> {
        self.map_err(|e| Error(format!("{}: {}", message(), e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_prefixes_message() {
        let r: std::result::Result<(), &str> = Err("boom");
        let e = r.context(|| "reading x".to_string()).unwrap_err();
        assert_eq!(e.to_string(), "reading x: boom");
    }
}
