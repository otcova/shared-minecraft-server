use std::error;
use std::panic::Location;

use git2::{ErrorClass, ErrorCode};

#[derive(Debug)]
pub struct Error {
    inner: git2::Error,
    location: &'static Location<'static>,
}

impl Error {
    #[track_caller]
    pub fn unknown() -> Self {
        Self {
            inner: git2::Error::new(ErrorCode::Ambiguous, ErrorClass::None, "unknown"),
            location: std::panic::Location::caller(),
        }
    }
}

impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        self.inner.eq(&other.inner)
    }
    fn ne(&self, other: &Self) -> bool {
        self.inner.ne(&other.inner)
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        Some(&self.inner)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}, {}", self.inner, self.location)
    }
}

impl From<git2::Error> for Error {
    #[track_caller]
    fn from(err: git2::Error) -> Self {
        Error {
            inner: err,
            location: std::panic::Location::caller(),
        }
    }
}

impl From<std::io::Error> for Error {
    #[track_caller]
    fn from(err: std::io::Error) -> Self {
        Error {
            inner: git2::Error::new(ErrorCode::Ambiguous, ErrorClass::Os, err.to_string()),
            location: std::panic::Location::caller(),
        }
    }
}
