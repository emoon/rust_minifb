use std::fmt;

/// Errors that can be returned from various operations
pub enum Error {
    /// Returned if menu Menu function isn't supported
    MenusNotSupported,
    /// Menu already exists
    MenuExists(String),
    /// Failed to create window
    WindowCreate(String),
    /// Unable to Update
    UpdateFailed(String),
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::MenusNotSupported => write!(formatter, "Menus not supported"),
            Error::MenuExists(_) => write!(formatter, "Menu already exists"),
            Error::WindowCreate(_) => write!(formatter, "Failed to create window"),
            Error::UpdateFailed(_) => write!(formatter, "Failed to Update"),
        }
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::MenusNotSupported => write!(fmt, "{}", self),
            Error::MenuExists(ref e) => write!(fmt, "{}, {:?}", self, e),
            Error::WindowCreate(ref e) => write!(fmt, "{}, {:?}", self, e),
            Error::UpdateFailed(ref e) => write!(fmt, "{}, {:?}", self, e),
        }
    }
}

impl std::error::Error for Error {}

#[cfg(target_arch = "wasm32")]
impl From<wasm_bindgen::JsValue> for Error {
    fn from(js_value: wasm_bindgen::JsValue) -> Self {
        Error::UpdateFailed(
            js_value
                .as_string()
                .unwrap_or("Non string error.".to_string()),
        )
    }
}
