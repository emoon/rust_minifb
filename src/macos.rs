#![cfg(target_os = "macos")]

use {Scale, Key, KeyRepeat};

use libc::{c_void, c_char, c_uchar};
use std::ffi::{CString};
use std::ptr;

#[link(name = "Cocoa", kind = "framework")]
extern {
    fn mfb_open(name: *const c_char, width: u32, height: u32, scale: u32) -> *mut c_void;
    fn mfb_close(window: *mut c_void);
    fn mfb_update(window: *mut c_void, buffer: *const c_uchar);
}

pub struct Window {
    window_handle: *mut c_void,
}

impl Window {
    pub fn new(name: &str,
               width: usize,
               height: usize,
               scale: Scale)
               -> Result<Window, &str> {
        let n = match CString::new(name) {
            Err(_) => { 
                println!("Unable to convert {} to c_string", name);
                return Err("Unable to set correct name"); 
            }
            Ok(n) => n,
        };

        unsafe {
            let handle = mfb_open(n.as_ptr(), width as u32, height as u32, scale as u32);

            if handle == ptr::null_mut() {
                return Err("Unable to open Window");
            }
            
            Ok(Window { window_handle: handle })
        }
    }

    pub fn update(&mut self, buffer: &[u32]) {
        unsafe {
            mfb_update(self.window_handle, buffer.as_ptr() as *const u8);
        }
    }

    pub fn get_keys(&self) -> Option<Vec<Key>> {
        None
    }

    pub fn get_keys_pressed(&self, _: KeyRepeat) -> Option<Vec<Key>> {
        None
    }

    #[inline]
    pub fn is_key_down(&self, _: Key) -> bool {
        false
    }

    #[inline]
    pub fn set_key_repeat_delay(&mut self, _: f32) {
    }

    #[inline]
    pub fn set_key_repeat_rate(&mut self, _: f32) {
    }

    pub fn key_pressed(&self, _: usize, _: KeyRepeat) -> bool {
        false
    }

    pub fn is_key_pressed(&self, _: Key, _: KeyRepeat) -> bool {
        false
    }

    #[inline]
    pub fn is_open(&self) -> bool {
        true
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            mfb_close(self.window_handle);
        }
    }
}

