use std::panic::Location;

#[derive(Debug)]
enum InnerError {
    Unknown,
    None(String),
    Io(std::io::Error),
    Git(git2::Error),
    Http(http_req::error::Error),
}

#[derive(Debug)]
pub struct Error {
    inner: InnerError,
    location: &'static Location<'static>,
}

impl Error {
    #[track_caller]
    pub fn from_str<S: Into<String>>(msg: S) -> Self {
        Self {
            inner: InnerError::None(msg.into()),
            location: std::panic::Location::caller(),
        }
    }
    #[track_caller]
    pub fn unknown() -> Self {
        Self {
            inner: InnerError::Unknown,
            location: std::panic::Location::caller(),
        }
    }

    pub fn is_unknown(&self) -> bool {
        match &self.inner {
            InnerError::Unknown => true,
            _ => false,
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.inner {
            InnerError::Unknown => None,
            InnerError::None(_) => None,
            InnerError::Io(err) => Some(err),
            InnerError::Git(err) => Some(err),
            InnerError::Http(err) => Some(err),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}, {}", self.inner, self.location)
    }
}

impl From<http_req::error::Error> for Error {
    #[track_caller]
    fn from(err: http_req::error::Error) -> Self {
        Error {
            inner: InnerError::Http(err),
            location: std::panic::Location::caller(),
        }
    }
}

impl From<git2::Error> for Error {
    #[track_caller]
    fn from(err: git2::Error) -> Self {
        Error {
            inner: InnerError::Git(err),
            location: std::panic::Location::caller(),
        }
    }
}

impl From<std::io::Error> for Error {
    #[track_caller]
    fn from(err: std::io::Error) -> Self {
        Error {
            inner: InnerError::Io(err),
            location: std::panic::Location::caller(),
        }
    }
}
