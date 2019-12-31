#![cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "netbsd",
    target_os = "openbsd"
))]
// turn off a gazillion warnings about X keysym names
#![allow(non_upper_case_globals)]

//mod wayland;
mod x11;
mod key_mapping;

use crate::key_handler::KeyHandler;
use crate::rate::UpdateRate;
use crate::{InputCallback, Key, KeyRepeat, MouseButton, MouseMode, WindowOptions, ScaleMode};
use crate::Result;
use crate::{CursorStyle, MenuHandle, UnixMenu};
pub use x11::Menu;
use crate::mouse_handler;

use std::os::raw;

//Common window attributes between X11 and Wayland
pub(self) struct CommonWindowData{
	pub(self) width: u32,
	pub(self) height: u32,
	
	pub(self) scale: i32,
	pub(self) bg_color: u32,
	pub(self) scale_mode: ScaleMode,
	
	pub(self) mouse_x: f32,
	pub(self) mouse_y: f32,
	pub(self) scroll_x: f32,
	pub(self) scroll_y: f32,
	pub(self) buttons: [u8; 3],
	pub(self) prev_cursor: CursorStyle,
	
	pub(self) should_close: bool,
	
	pub(self) key_handler: KeyHandler,
	pub(self) update_rate: UpdateRate,
	pub(self) menus: Vec<UnixMenu>,
	pub(self) menu_counter: MenuHandle,
}

impl CommonWindowData{
	#[inline]
	pub fn set_background_color(&mut self, bg_color: u32){
		self.bg_color = bg_color;
	}

	#[inline]
	pub fn get_size(&self) -> (usize, usize){
		(self.width as usize, self.height as usize)
	}

	pub fn get_mouse_pos(&self, mode: MouseMode) -> Option<(f32, f32)> {
		let s = self.scale as f32;
		let w = self.width as f32;
		let h = self.height as f32;

		mouse_handler::get_pos(mode, self.mouse_x, self.mouse_y, s, w, h)
	}

	pub fn get_unscaled_mouse_pos(&self, mode: MouseMode) -> Option<(f32, f32)> {
		let w = self.width as f32;
		let h = self.height as f32;
		
		mouse_handler::get_pos(mode, self.mouse_x, self.mouse_y, 1.0, w, h)
	}

	pub fn get_mouse_down(&self, button: MouseButton) -> bool {
		match button {
			MouseButton::Left => self.buttons[0] > 0,
			MouseButton::Middle => self.buttons[1] > 0,
			MouseButton::Right => self.buttons[2] > 0,
		}
	}

	pub fn get_scroll_wheel(&self) -> Option<(f32, f32)> {
		if self.scroll_x.abs() > 0.0 || self.scroll_y.abs() > 0.0 {
			Some((self.scroll_x, self.scroll_y))
		} else {
			None
		}
	}

	#[inline]
    pub fn set_rate(&mut self, rate: Option<std::time::Duration>) {
        self.update_rate.set_rate(rate);
    }

    #[inline]
    pub fn update_rate(&mut self) {
        self.update_rate.update();
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
    pub fn is_key_released(&self, key: Key) -> bool {
        self.key_handler.is_key_released(key)
    }

    #[inline]
    pub fn set_input_callback(&mut self, callback: Box<dyn InputCallback>) {
        self.key_handler.set_input_callback(callback)
    }

    #[inline]
    pub fn is_open(&self) -> bool {
        !self.should_close
    }

    pub fn get_unix_menus(&self) -> Option<&Vec<UnixMenu>> {
        Some(&self.menus)
    }

    pub fn remove_menu(&mut self, handle: MenuHandle) {
        self.menus.retain(|ref menu| menu.handle != handle);
    }

    pub fn is_menu_pressed(&mut self) -> Option<usize> {
        None
    }

}

pub enum Window{
	X11(x11::Window),
	Wayland(())//WlWindow
}

impl Window{
    pub fn new(name: &str, width: usize, height: usize, opts: WindowOptions) -> Result<Window> {
		//TODO: Try to create Wayland display first
		//..

		//Create X11 Window when Wayland fails
		let window = Window::X11(x11::Window::new(name, width, height, opts)?);

		Ok(window)
	}

	
    pub fn set_title(&mut self, title: &str) {
		match *self{
			Window::X11(ref mut w) => w.set_title(title),
			Window::Wayland(ref mut _w) => unimplemented!(),
		}
	}

    pub fn update_with_buffer_stride(
        &mut self,
        buffer: &[u32],
        buf_width: usize,
        buf_height: usize,
        buf_stride: usize,
    ) -> Result<()> {
    	match *self{
			Window::X11(ref mut w) => w.update_with_buffer_stride(buffer, buf_width, buf_height, buf_stride),
			Window::Wayland(ref mut _w) => unimplemented!(),
		}
	}
	
	pub fn update(&mut self) {
    	match *self{
			Window::X11(ref mut w) => w.update(),
			Window::Wayland(ref mut _w) => unimplemented!(),
		}
	}
	
	pub fn get_window_handle(&self) -> *mut raw::c_void {
		match *self{
			Window::X11(ref w) => w.get_window_handle(),
			Window::Wayland(ref _w) => unimplemented!(),
		}
	}
	
	pub fn set_background_color(&mut self, bg_color: u32) {
		match *self{
			Window::X11(ref mut w) => w.get_common_data_mut().set_background_color(bg_color),
			Window::Wayland(ref mut _w) => unimplemented!(),
		}
	}
    
	pub fn set_position(&mut self, x: isize, y: isize) {
		match *self{
			Window::X11(ref mut w) => w.set_position(x, y),
			Window::Wayland(ref mut _w) => unimplemented!(),
		}
	}
    
	pub fn get_size(&self) -> (usize, usize) {
		match *self{
			Window::X11(ref w) => w.get_common_data().get_size(),
			Window::Wayland(ref _w) => unimplemented!(),
		}
	}
    
	pub fn get_mouse_pos(&self, mode: MouseMode) -> Option<(f32, f32)> {
		match *self{
			Window::X11(ref w) => w.get_common_data().get_mouse_pos(mode),
			Window::Wayland(ref _w) => unimplemented!(),
		}
	}
    
	pub fn get_unscaled_mouse_pos(&self, mode: MouseMode) -> Option<(f32, f32)> {
		match *self{
			Window::X11(ref w) => w.get_common_data().get_unscaled_mouse_pos(mode),
			Window::Wayland(ref _w) => unimplemented!(),
		}
	}
    
	pub fn get_mouse_down(&self, button: MouseButton) -> bool {
		match *self{
			Window::X11(ref w) => w.get_common_data().get_mouse_down(button),
			Window::Wayland(ref _w) => unimplemented!(),
		}
	}
    
	pub fn get_scroll_wheel(&self) -> Option<(f32, f32)> {
		match *self{
			Window::X11(ref w) => w.get_common_data().get_scroll_wheel(),
			Window::Wayland(ref _w) => unimplemented!(),
		}
	}
    
	pub fn set_cursor_style(&mut self, cursor: CursorStyle) {
		match *self{
			Window::X11(ref mut w) => w.set_cursor_style(cursor),
			Window::Wayland(ref mut _w) => unimplemented!(),
		}
	}
    
	pub fn set_rate(&mut self, rate: Option<std::time::Duration>) {
		match *self{
			Window::X11(ref mut w) => w.get_common_data_mut().set_rate(rate),
			Window::Wayland(ref mut _w) => unimplemented!(),
		}
	}
    
	pub fn update_rate(&mut self) {
		match *self{
			Window::X11(ref mut w) => w.get_common_data_mut().update_rate(),
			Window::Wayland(ref mut _w) => unimplemented!(),
		}
	}
    
	pub fn get_keys(&self) -> Option<Vec<Key>> {
		match *self{
			Window::X11(ref w) => w.get_common_data().get_keys(),
			Window::Wayland(ref _w) => unimplemented!(),
		}
	}
    
	pub fn get_keys_pressed(&self, repeat: KeyRepeat) -> Option<Vec<Key>> {
		match *self{
			Window::X11(ref w) => w.get_common_data().get_keys_pressed(repeat),
			Window::Wayland(ref _w) => unimplemented!(),
		}
	}
    
	pub fn is_key_down(&self, key: Key) -> bool {
		match *self{
			Window::X11(ref w) => w.get_common_data().is_key_down(key),
			Window::Wayland(ref _w) => unimplemented!(),
		}
	}
    
	pub fn set_key_repeat_delay(&mut self, delay: f32) {
		match *self{
			Window::X11(ref mut w) => w.get_common_data_mut().set_key_repeat_delay(delay),
			Window::Wayland(ref mut _w) => unimplemented!(),
		}
	}
    
	pub fn set_key_repeat_rate(&mut self, rate: f32) {
		match *self{
			Window::X11(ref mut w) => w.get_common_data_mut().set_key_repeat_rate(rate),
			Window::Wayland(ref mut _w) => unimplemented!(),
		}
	}
    
	pub fn is_key_pressed(&self, key: Key, repeat: KeyRepeat) -> bool {
		match *self{
			Window::X11(ref w) => w.get_common_data().is_key_pressed(key, repeat),
			Window::Wayland(ref _w) => unimplemented!(),
		}
	}
    
	pub fn is_key_released(&self, key: Key) -> bool {
		match *self{
			Window::X11(ref w) => w.get_common_data().is_key_released(key),
			Window::Wayland(ref _w) => unimplemented!(),
		}
	}
    
	pub fn set_input_callback(&mut self, callback: Box<dyn InputCallback>) {
		match *self{
			Window::X11(ref mut w) => w.get_common_data_mut().set_input_callback(callback),
			Window::Wayland(ref mut _w) => unimplemented!(),
		}
	}
    
	pub fn is_open(&self) -> bool {
		match *self{
			Window::X11(ref w) => w.get_common_data().is_open(),
			Window::Wayland(ref _w) => unimplemented!(),
		}
	}
    
	pub fn is_active(&mut self) -> bool {
		match *self{
			Window::X11(ref mut w) => w.is_active(),
			Window::Wayland(ref mut _w) => unimplemented!(),
		}
	}

	pub fn add_menu(&mut self, menu: &Menu) -> MenuHandle {
		match *self{
			Window::X11(ref mut w) => w.add_menu(menu),
			Window::Wayland(ref mut _w) => unimplemented!(),
		}
	}

	pub fn get_unix_menus(&self) -> Option<&Vec<UnixMenu>> {
		match *self{
			Window::X11(ref w) => w.get_common_data().get_unix_menus(),
			Window::Wayland(ref _w) => unimplemented!(),
		}
	}

    pub fn remove_menu(&mut self, handle: MenuHandle) {
		match *self{
			Window::X11(ref mut w) => w.get_common_data_mut().remove_menu(handle),
			Window::Wayland(ref mut _w) => unimplemented!(),
		}
	}

    pub fn is_menu_pressed(&mut self) -> Option<usize> {
		match *self{
			Window::X11(ref mut w) => w.get_common_data_mut().is_menu_pressed(),
			Window::Wayland(ref mut _w) => unimplemented!(),
		}
	}
}

unsafe impl raw_window_handle::HasRawWindowHandle for Window {
    fn raw_window_handle(&self) -> raw_window_handle::RawWindowHandle {
		match *self{
			Window::X11(ref w) => w.raw_window_handle(),
			Window::Wayland(ref w) => unimplemented!(),
		}
	}
}
