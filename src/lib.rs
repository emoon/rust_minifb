extern crate libc;

#[cfg(target_os = "macos")]
#[macro_use]
extern crate objc;
#[cfg(target_os = "macos")]
extern crate cgl;
#[cfg(target_os = "macos")]
extern crate cocoa;
#[cfg(target_os = "macos")]
extern crate core_foundation;
#[cfg(target_os = "macos")]

/// Error that can happen while creating a window or a headless renderer.
#[derive(Debug)]
pub enum CreationError {
    OsError(String),
    NotSupported,
}

impl CreationError {
    fn to_string(&self) -> &str {
        match *self {
            CreationError::OsError(ref text) => &text,
            CreationError::NotSupported => "Some of the requested attributes are not supported",
        }
    }
}

#[cfg(target_os = "windows")]
pub mod windows;
#[cfg(target_os = "windows")]
pub use windows::*;

#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "mac")]
pub use macos::*;


/*

#[cfg(target_os = "macos")]
#[link(name = "Cocoa", kind = "framework")]
extern {
    fn mfb_open(name: *const c_char, width: c_int, height: c_int) -> c_int;
    fn mfb_update(buffer: *mut c_void) -> c_int;
    fn mfb_close();
}

/*
#[cfg(target_os = "windows")]
#[link(name = "gdi32")]
extern {
    fn mfb_open(name: *const c_char, width: c_int, height: c_int) -> c_int;
    fn mfb_update(buffer: *mut c_void) -> c_int;
    fn mfb_close();
}
*/

#[cfg(target_os = "linux")]
#[link(name = "X11")]
extern {
    fn mfb_open(name: *const c_char, width: c_int, height: c_int) -> c_int;
    fn mfb_update(buffer: *mut c_void) -> c_int;
    fn mfb_close();
}

///
/// Open up a window
///
#[cfg(any(target_os = "linux", target_os = "mac"))]
pub fn open(name: &str, width: usize, height: usize) -> bool {
    let s = CString::new(name).unwrap();
    let ret;

    unsafe {
        ret = mfb_open(s.as_ptr(), width as c_int, height as c_int);
    }

    match ret {
        0 => false,
        _ => true,
    }
}

///
/// Update 
///
#[cfg(any(target_os = "linux", target_os = "mac"))]
pub fn update(buffer: &[u32]) -> bool {
    let ret;
    unsafe {
        ret = mfb_update(transmute(buffer.as_ptr()));
    }

    if ret < 0 {
        return false;
    } else {
        return true;
    }
}

///
/// Close 
///
#[cfg(any(target_os = "linux", target_os = "mac"))]
pub fn close() {
    unsafe {
        mfb_close();
    }
}

*/
