use std::{error, fmt, io};

/// All titan errors
#[derive(Debug)]
pub enum Error {
    /// Interface name must be ascii and len must be less than 16
    InvalidName,
    /// Io error
    Io(io::Error),
}

impl Error {
    pub(crate) fn last() -> Self {
        Self::Io(io::Error::last_os_error())
    }
}

/// Titan error result alias
pub type Result<T> = core::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidName => write!(f, "InvalidName"),
            Error::Io(err) => write!(f, "{err}"),
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl error::Error for Error {}
