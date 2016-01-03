#![cfg(unix)]

use {Scale, Key, KeyRepeat};
use key_handler::KeyHandler;

use libc::{c_void, c_char, c_uchar};
use std::ffi::{CString};
use std::ptr;
//use std::mem;

#[link(name = "X11")]
extern {
    fn mfb_open(name: *const c_char, width: u32, height: u32, scale: i32) -> *mut c_void;
    fn mfb_close(window: *mut c_void);
    fn mfb_update(window: *mut c_void, buffer: *const c_uchar);
    //fn mfb_set_key_callback(window: *mut c_void, target: *mut c_void, cb: unsafe extern fn(*mut c_void, i32, i32));
    //fn mfb_should_close(window: *mut c_void) -> i32;
    //fn mfb_get_screen_size() -> u32;
}

pub struct Window {
    window_handle: *mut c_void,
    key_handler: KeyHandler,
}

impl Window {
    pub fn new(name: &str, width: usize, height: usize, scale: Scale) -> Result<Window, &str> {
        let n = match CString::new(name) {
            Err(_) => { 
                println!("Unable to convert {} to c_string", name);
                return Err("Unable to set correct name"); 
            }
            Ok(n) => n,
        };

        unsafe {
            let handle = mfb_open(n.as_ptr(), width as u32, height as u32, Self::get_scale_factor(width, height, scale));

            if handle == ptr::null_mut() {
                return Err("Unable to open Window");
            }

            Ok(Window { 
                window_handle: handle,
                key_handler: KeyHandler::new(),
            })
        }
    }

    pub fn update(&mut self, buffer: &[u32]) {
        self.key_handler.update();

        unsafe {
            mfb_update(self.window_handle, buffer.as_ptr() as *const u8);
            //mfb_set_key_callback(self.window_handle, mem::transmute(self), key_callback);
        }
    }

    #[inline]
    pub fn get_keys(&self) -> Option<Vec<Key>> {
        self.key_handler.get_keys()
    }

    #[inline]
    pub fn get_keys_pressed(&self, repeat: KeyRepeat) -> Option<Vec<Key>> {
        self.key_handler.get_keys_pressed(repeat)
    }

    #[inline]
    pub fn is_key_down(&self, key: Key) -> bool {
        self.key_handler.is_key_down(key)
    }

    #[inline]
    pub fn set_key_repeat_delay(&mut self, delay: f32) {
        self.key_handler.set_key_repeat_delay(delay)
    }

    #[inline]
    pub fn set_key_repeat_rate(&mut self, rate: f32) {
        self.key_handler.set_key_repeat_rate(rate)
    }

    #[inline]
    pub fn is_key_pressed(&self, key: Key, repeat: KeyRepeat) -> bool {
        self.key_handler.is_key_pressed(key, repeat)
    }

    #[inline]
    pub fn is_open(&self) -> bool {
    	true
        //unsafe { mfb_should_close(self.window_handle) == 0 }
    }

    unsafe fn get_scale_factor(_: usize, _: usize, scale: Scale) -> i32 {
        let factor: i32 = match scale {
            Scale::X1 => 1,
            Scale::X2 => 2,
            Scale::X4 => 4,
            Scale::X8 => 8,
            Scale::X16 => 16,
            Scale::X32 => 32,
            Scale::FitScreen => {
            	1
            }
        };

        return factor;
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            mfb_close(self.window_handle);
        }
    }
}

