use std::error::Error as StdError;
use std::fmt;

/// Errors that can be return from various operatiors
///
#[derive(Debug)]
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

impl StdError for Error {
    fn description(&self) -> &str {
        match *self {
            Error::MenusNotSupported => "Menus not supported",
            Error::MenuExists(_) => "Menu already exists",
            Error::WindowCreate(_) => "Failed to create window",
            Error::UpdateFailed(_) => "Failed to Update",
        }
    }

    fn cause(&self) -> Option<&dyn StdError> {
        match *self {
            Error::MenusNotSupported => None,
            Error::MenuExists(_) => None,
            Error::WindowCreate(_) => None,
            Error::UpdateFailed(_) => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::MenusNotSupported => {
                write!(fmt, "{}", self.description())
            },
            Error::MenuExists(ref e) => {
                write!(fmt, "{} {:?}", self.description(), e)
            },
            Error::WindowCreate(ref e) => {
                write!(fmt, "{} {:?}", self.description(), e)
            }
            Error::UpdateFailed(ref e) => {
                write!(fmt, "{} {:?}", self.description(), e)
            }
        }
    }
}
