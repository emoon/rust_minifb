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
            Error::MenuExists(ref e) => write!(formatter, "Menu already exists: {e}"),
            Error::WindowCreate(ref e) => write!(formatter, "Failed to create window: {e}"),
            Error::UpdateFailed(ref e) => write!(formatter, "Failed to Update: {e}"),
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

#[cfg(test)]
mod tests {
    use super::Error;

    #[test]
    fn display_includes_payload() {
        // The payload string must reach `Display` / `to_string()` so detailed
        // messages (e.g. from check_buffer_size) are not lost (see #426).
        assert_eq!(
            Error::UpdateFailed("buffer too small".to_string()).to_string(),
            "Failed to Update: buffer too small"
        );
        assert_eq!(
            Error::WindowCreate("no backend".to_string()).to_string(),
            "Failed to create window: no backend"
        );
        assert_eq!(
            Error::MenuExists("File".to_string()).to_string(),
            "Menu already exists: File"
        );
        assert_eq!(Error::MenusNotSupported.to_string(), "Menus not supported");
    }
}

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
