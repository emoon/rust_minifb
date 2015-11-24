extern crate libc;
use std::ffi::CString;
use std::mem::transmute;
use libc::{c_char, c_int, c_void};

#[cfg(target_os = "windows")]
pub mod windows;

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
