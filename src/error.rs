use std::fmt;
use std::error::Error as StdError;

/// Errors that can be returned from various operations
///
pub enum Error {
    /// Returned if menu Menu function isn't supported
    MenusNotSupported,
    /// Menu already exists
    MenuExists(String),
    /// Menu already exists
    WindowCreate(String),
    /// Unable to Update
    UpdateFailed(String),
}

impl fmt::Debug for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Error::MenusNotSupported => write!(formatter, "Menus not supported"),
            Error::MenuExists(_) => write!(formatter, "Menu already exists"),
            Error::WindowCreate(_) => write!(formatter, "Failed to create window"),
            Error::UpdateFailed(_) => write!(formatter, "Failed to Update"),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::MenusNotSupported => write!(fmt, "{:?}", self),
            Error::MenuExists(ref e) => write!(fmt, "{:?} {:?}", self, e),
            Error::WindowCreate(ref e) => write!(fmt, "{:?} {:?}", self, e),
            Error::UpdateFailed(ref e) => write!(fmt, "{:?} {:?}", self, e),
        }
    }
}

impl StdError for Error {}
