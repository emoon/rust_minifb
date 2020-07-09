#![cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "netbsd",
    target_os = "openbsd"
))]
// turn off a gazillion warnings about X keysym names
#![allow(non_upper_case_globals)]

mod common;
#[cfg(feature = "wayland")]
mod wayland;
#[cfg(feature = "x11")]
mod x11;

use crate::Result;
use crate::{CursorStyle, MenuHandle, UnixMenu};
use crate::{InputCallback, Key, KeyRepeat, MouseButton, MouseMode, WindowOptions};
pub use common::Menu;

use std::os::raw;

// Differentiate between Wayland and X11 at run-time
pub enum Window {
    #[cfg(feature = "x11")]
    X11(x11::Window),
    #[cfg(feature = "wayland")]
    Wayland(wayland::Window),
}

impl Window {
    #[cfg(all(feature = "x11", feature = "wayland"))]
    pub fn new(name: &str, width: usize, height: usize, opts: WindowOptions) -> Result<Window> {
        // Try to create Wayland display first
        let wl_window = wayland::Window::new(name, width, height, opts);
        match wl_window {
            Ok(w) => Ok(Window::Wayland(w)),
            Err(_) => {
                // Create X11 Window when Wayland fails
                let window = Window::X11(x11::Window::new(name, width, height, opts)?);
                Ok(window)
            }
        }
    }

    #[cfg(all(feature = "wayland", not(feature = "x11")))]
    pub fn new(name: &str, width: usize, height: usize, opts: WindowOptions) -> Result<Window> {
        let wl_window = wayland::Window::new(name, width, height, opts)?;
        Ok(Window::Wayland(wl_window))
    }

    #[cfg(all(feature = "x11", not(feature = "wayland")))]
    pub fn new(name: &str, width: usize, height: usize, opts: WindowOptions) -> Result<Window> {
        let window = Window::X11(x11::Window::new(name, width, height, opts)?);
        Ok(window)
    }

    pub fn set_title(&mut self, title: &str) {
        match *self {
            #[cfg(feature = "x11")]
            Window::X11(ref mut w) => w.set_title(title),
            #[cfg(feature = "wayland")]
            Window::Wayland(ref mut w) => w.set_title(title),
        }
    }

    pub fn update_with_buffer_stride(
        &mut self,
        buffer: &[u32],
        buf_width: usize,
        buf_height: usize,
        buf_stride: usize,
    ) -> Result<()> {
        match *self {
            #[cfg(feature = "x11")]
            Window::X11(ref mut w) => {
                w.update_with_buffer_stride(buffer, buf_width, buf_height, buf_stride)
            }
            #[cfg(feature = "wayland")]
            Window::Wayland(ref mut w) => {
                w.update_with_buffer_stride(buffer, buf_width, buf_height, buf_stride)
            }
        }
    }

    pub fn update(&mut self) {
        match *self {
            #[cfg(feature = "x11")]
            Window::X11(ref mut w) => w.update(),
            #[cfg(feature = "wayland")]
            Window::Wayland(ref mut w) => w.update(),
        }
    }

    pub fn get_window_handle(&self) -> *mut raw::c_void {
        match *self {
            #[cfg(feature = "x11")]
            Window::X11(ref w) => w.get_window_handle(),
            #[cfg(feature = "wayland")]
            Window::Wayland(ref w) => w.get_window_handle(),
        }
    }

    pub fn set_background_color(&mut self, bg_color: u32) {
        match *self {
            #[cfg(feature = "x11")]
            Window::X11(ref mut w) => w.set_background_color(bg_color),
            #[cfg(feature = "wayland")]
            Window::Wayland(ref mut w) => w.set_background_color(bg_color),
        }
    }

    pub fn set_cursor_visibility(&mut self, visibility: bool) {
        match *self {
            #[cfg(feature = "x11")]
            Window::X11(ref mut w) => w.set_cursor_visibility(visibility),
            #[cfg(feature = "wayland")]
            Window::Wayland(ref mut w) => w.set_cursor_visibility(visibility),
        }
    }

    pub fn set_position(&mut self, x: isize, y: isize) {
        match *self {
            #[cfg(feature = "x11")]
            Window::X11(ref mut w) => w.set_position(x, y),
            #[cfg(feature = "wayland")]
            Window::Wayland(ref mut w) => w.set_position(x, y),
        }
    }

    pub fn get_size(&self) -> (usize, usize) {
        match *self {
            #[cfg(feature = "x11")]
            Window::X11(ref w) => w.get_size(),
            #[cfg(feature = "wayland")]
            Window::Wayland(ref w) => w.get_size(),
        }
    }

    pub fn get_mouse_pos(&self, mode: MouseMode) -> Option<(f32, f32)> {
        match *self {
            #[cfg(feature = "x11")]
            Window::X11(ref w) => w.get_mouse_pos(mode),
            #[cfg(feature = "wayland")]
            Window::Wayland(ref w) => w.get_mouse_pos(mode),
        }
    }

    pub fn get_unscaled_mouse_pos(&self, mode: MouseMode) -> Option<(f32, f32)> {
        match *self {
            #[cfg(feature = "x11")]
            Window::X11(ref w) => w.get_unscaled_mouse_pos(mode),
            #[cfg(feature = "wayland")]
            Window::Wayland(ref w) => w.get_unscaled_mouse_pos(mode),
        }
    }

    pub fn get_mouse_down(&self, button: MouseButton) -> bool {
        match *self {
            #[cfg(feature = "x11")]
            Window::X11(ref w) => w.get_mouse_down(button),
            #[cfg(feature = "wayland")]
            Window::Wayland(ref w) => w.get_mouse_down(button),
        }
    }

    pub fn get_scroll_wheel(&self) -> Option<(f32, f32)> {
        match *self {
            #[cfg(feature = "x11")]
            Window::X11(ref w) => w.get_scroll_wheel(),
            #[cfg(feature = "wayland")]
            Window::Wayland(ref w) => w.get_scroll_wheel(),
        }
    }

    pub fn set_cursor_style(&mut self, cursor: CursorStyle) {
        match *self {
            #[cfg(feature = "x11")]
            Window::X11(ref mut w) => w.set_cursor_style(cursor),
            #[cfg(feature = "wayland")]
            Window::Wayland(ref mut w) => w.set_cursor_style(cursor),
        }
    }

    pub fn set_rate(&mut self, rate: Option<std::time::Duration>) {
        match *self {
            #[cfg(feature = "x11")]
            Window::X11(ref mut w) => w.set_rate(rate),
            #[cfg(feature = "wayland")]
            Window::Wayland(ref mut w) => w.set_rate(rate),
        }
    }

    pub fn update_rate(&mut self) {
        match *self {
            #[cfg(feature = "x11")]
            Window::X11(ref mut w) => w.update_rate(),
            #[cfg(feature = "wayland")]
            Window::Wayland(ref mut w) => w.update_rate(),
        }
    }

    pub fn get_keys(&self) -> Option<Vec<Key>> {
        match *self {
            #[cfg(feature = "x11")]
            Window::X11(ref w) => w.get_keys(),
            #[cfg(feature = "wayland")]
            Window::Wayland(ref w) => w.get_keys(),
        }
    }

    pub fn get_keys_pressed(&self, repeat: KeyRepeat) -> Option<Vec<Key>> {
        match *self {
            #[cfg(feature = "x11")]
            Window::X11(ref w) => w.get_keys_pressed(repeat),
            #[cfg(feature = "wayland")]
            Window::Wayland(ref w) => w.get_keys_pressed(repeat),
        }
    }

    pub fn get_keys_released(&self) -> Option<Vec<Key>> {
        match *self {
            #[cfg(feature = "x11")]
            Window::X11(ref w) => w.get_keys_released(),
            #[cfg(feature = "wayland")]
            Window::Wayland(ref _w) => unimplemented!(),
        }
    }

    pub fn is_key_down(&self, key: Key) -> bool {
        match *self {
            #[cfg(feature = "x11")]
            Window::X11(ref w) => w.is_key_down(key),
            #[cfg(feature = "wayland")]
            Window::Wayland(ref w) => w.is_key_down(key),
        }
    }

    pub fn set_key_repeat_delay(&mut self, delay: f32) {
        match *self {
            #[cfg(feature = "x11")]
            Window::X11(ref mut w) => w.set_key_repeat_delay(delay),
            #[cfg(feature = "wayland")]
            Window::Wayland(ref mut w) => w.set_key_repeat_delay(delay),
        }
    }

    pub fn set_key_repeat_rate(&mut self, rate: f32) {
        match *self {
            #[cfg(feature = "x11")]
            Window::X11(ref mut w) => w.set_key_repeat_rate(rate),
            #[cfg(feature = "wayland")]
            Window::Wayland(ref mut w) => w.set_key_repeat_rate(rate),
        }
    }

    pub fn is_key_pressed(&self, key: Key, repeat: KeyRepeat) -> bool {
        match *self {
            #[cfg(feature = "x11")]
            Window::X11(ref w) => w.is_key_pressed(key, repeat),
            #[cfg(feature = "wayland")]
            Window::Wayland(ref w) => w.is_key_pressed(key, repeat),
        }
    }

    pub fn is_key_released(&self, key: Key) -> bool {
        match *self {
            #[cfg(feature = "x11")]
            Window::X11(ref w) => w.is_key_released(key),
            #[cfg(feature = "wayland")]
            Window::Wayland(ref w) => w.is_key_released(key),
        }
    }

    pub fn set_input_callback(&mut self, callback: Box<dyn InputCallback>) {
        match *self {
            #[cfg(feature = "x11")]
            Window::X11(ref mut w) => w.set_input_callback(callback),
            #[cfg(feature = "wayland")]
            Window::Wayland(ref mut w) => w.set_input_callback(callback),
        }
    }

    pub fn is_open(&self) -> bool {
        match *self {
            #[cfg(feature = "x11")]
            Window::X11(ref w) => w.is_open(),
            #[cfg(feature = "wayland")]
            Window::Wayland(ref w) => w.is_open(),
        }
    }

    pub fn is_active(&mut self) -> bool {
        match *self {
            #[cfg(feature = "x11")]
            Window::X11(ref mut w) => w.is_active(),
            #[cfg(feature = "wayland")]
            Window::Wayland(ref mut w) => w.is_active(),
        }
    }

    pub fn add_menu(&mut self, menu: &Menu) -> MenuHandle {
        match *self {
            #[cfg(feature = "x11")]
            Window::X11(ref mut w) => w.add_menu(menu),
            #[cfg(feature = "wayland")]
            Window::Wayland(ref mut w) => w.add_menu(menu),
        }
    }

    pub fn get_posix_menus(&self) -> Option<&Vec<UnixMenu>> {
        match *self {
            #[cfg(feature = "x11")]
            Window::X11(ref w) => w.get_posix_menus(),
            #[cfg(feature = "wayland")]
            Window::Wayland(ref w) => w.get_posix_menus(),
        }
    }

    pub fn remove_menu(&mut self, handle: MenuHandle) {
        match *self {
            #[cfg(feature = "x11")]
            Window::X11(ref mut w) => w.remove_menu(handle),
            #[cfg(feature = "wayland")]
            Window::Wayland(ref mut w) => w.remove_menu(handle),
        }
    }

    pub fn is_menu_pressed(&mut self) -> Option<usize> {
        match *self {
            #[cfg(feature = "x11")]
            Window::X11(ref mut w) => w.is_menu_pressed(),
            #[cfg(feature = "wayland")]
            Window::Wayland(ref mut w) => w.is_menu_pressed(),
        }
    }
}

unsafe impl raw_window_handle::HasRawWindowHandle for Window {
    fn raw_window_handle(&self) -> raw_window_handle::RawWindowHandle {
        match *self {
            #[cfg(feature = "x11")]
            Window::X11(ref w) => w.raw_window_handle(),
            #[cfg(feature = "wayland")]
            Window::Wayland(ref w) => w.raw_window_handle(),
        }
    }
}
