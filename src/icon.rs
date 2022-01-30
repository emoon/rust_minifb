#[cfg(target_os = "windows")]
use std::{str::FromStr, ffi::OsStr, os::windows::prelude::OsStrExt};
#[cfg(target_os = "linux")]
use std::convert::TryFrom;

///
/// Represents a window icon
/// 
/// Different under Windows, Linux and MacOS
/// 
/// **Windows**: Icon can be created from a relative path string
/// 
/// **Linux / X11:** Icon can be created from an ARGB buffer
/// 
/// 
#[derive(Clone, Copy, Debug)]
pub enum Icon {
    Path(*const u16),
    Buffer(*const u64, u32),
}

#[cfg(target_os = "windows")]
impl FromStr for Icon {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() == 0 {
            return Err("Path to icon cannot be empty!");
        }

        let v: Vec<u16> = OsStr::new(s)
            .encode_wide()
            .chain(Some(0).into_iter())
            .collect();
        
        Ok(Icon::Path(v.as_ptr()))
    }
}

#[cfg(target_os = "linux")]
impl TryFrom<&[u64]> for Icon {
    type Error = &'static str;

    fn try_from(value: &[u64]) -> Result<Self, Self::Error> {
        if value.len() == 0 {
            return Err("ARGB buffer cannot be empty!");
        }

        Ok(Icon::Buffer(value.as_ptr(), value.len() as u32))
    }
}