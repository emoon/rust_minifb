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

pub enum Window{
	X11(x11::Window),
	Wayland(())//WlWindow
}
