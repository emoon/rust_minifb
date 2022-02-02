#![cfg(target_os = "macos")]

use crate::error::Error;
use crate::key_handler::KeyHandler;
use crate::rate::UpdateRate;
use crate::Result;
use crate::{Key, KeyRepeat, MouseButton, MouseMode, Scale, WindowOptions};
// use MenuItem;
use crate::buffer_helper;
use crate::mouse_handler;
use crate::window_flags;
use crate::InputCallback;
use crate::{CursorStyle, MenuHandle, MenuItem, MenuItemHandle};
// use menu::Menu;

use std::ffi::CString;
use std::mem;
use std::os::raw;
use std::os::raw::{c_char, c_uchar, c_void};
use std::ptr;

// Table taken from GLFW and slightly modified

static KEY_MAPPINGS: [Key; 128] = [
    /* 00 */ Key::A,
    /* 01 */ Key::S,
    /* 02 */ Key::D,
    /* 03 */ Key::F,
    /* 04 */ Key::H,
    /* 05 */ Key::G,
    /* 06 */ Key::Z,
    /* 07 */ Key::X,
    /* 08 */ Key::C,
    /* 09 */ Key::V,
    /* 0a */ Key::Unknown, // GraveAccent
    /* 0b */ Key::B,
    /* 0c */ Key::Q,
    /* 0d */ Key::W,
    /* 0e */ Key::E,
    /* 0f */ Key::R,
    /* 10 */ Key::Y,
    /* 11 */ Key::T,
    /* 12 */ Key::Key1,
    /* 13 */ Key::Key2,
    /* 14 */ Key::Key3,
    /* 15 */ Key::Key4,
    /* 16 */ Key::Key6,
    /* 17 */ Key::Key5,
    /* 18 */ Key::Equal,
    /* 19 */ Key::Key9,
    /* 1a */ Key::Key7,
    /* 1b */ Key::Minus,
    /* 1c */ Key::Key8,
    /* 1d */ Key::Key0,
    /* 1e */ Key::RightBracket,
    /* 1f */ Key::O,
    /* 20 */ Key::U,
    /* 21 */ Key::LeftBracket,
    /* 22 */ Key::I,
    /* 23 */ Key::P,
    /* 24 */ Key::Enter,
    /* 25 */ Key::L,
    /* 26 */ Key::J,
    /* 27 */ Key::Apostrophe,
    /* 28 */ Key::K,
    /* 29 */ Key::Semicolon,
    /* 2a */ Key::Backslash,
    /* 2b */ Key::Comma,
    /* 2c */ Key::Slash,
    /* 2d */ Key::N,
    /* 2e */ Key::M,
    /* 2f */ Key::Period,
    /* 30 */ Key::Tab,
    /* 31 */ Key::Space,
    /* 32 */ Key::Unknown, // World1
    /* 33 */ Key::Backspace,
    /* 34 */ Key::Unknown,
    /* 35 */ Key::Escape,
    /* 36 */ Key::RightSuper,
    /* 37 */ Key::LeftSuper,
    /* 38 */ Key::LeftShift,
    /* 39 */ Key::CapsLock,
    /* 3a */ Key::LeftAlt,
    /* 3b */ Key::LeftCtrl,
    /* 3c */ Key::RightShift,
    /* 3d */ Key::RightAlt,
    /* 3e */ Key::RightCtrl,
    /* 3f */ Key::Unknown, // Function
    /* 40 */ Key::Unknown, // F17
    /* 41 */ Key::Unknown, // Decimal
    /* 42 */ Key::Unknown,
    /* 43 */ Key::Unknown, // Multiply
    /* 44 */ Key::Unknown,
    /* 45 */ Key::Unknown, // Add
    /* 46 */ Key::Unknown,
    /* 47 */ Key::NumLock, // Really KeypadClear...
    /* 48 */ Key::Unknown, // VolumeUp
    /* 49 */ Key::Unknown, // VolumeDown
    /* 4a */ Key::Unknown, // Mute
    /* 4b */ Key::Unknown,
    /* 4c */ Key::Enter,
    /* 4d */ Key::Unknown,
    /* 4e */ Key::Unknown, // Subtrackt
    /* 4f */ Key::Unknown, // F18
    /* 50 */ Key::Unknown, // F19
    /* 51 */ Key::Equal,
    /* 52 */ Key::NumPad0,
    /* 53 */ Key::NumPad1,
    /* 54 */ Key::NumPad2,
    /* 55 */ Key::NumPad3,
    /* 56 */ Key::NumPad4,
    /* 57 */ Key::NumPad5,
    /* 58 */ Key::NumPad6,
    /* 59 */ Key::NumPad7,
    /* 5a */ Key::Unknown, // F20
    /* 5b */ Key::NumPad8,
    /* 5c */ Key::NumPad9,
    /* 5d */ Key::Unknown,
    /* 5e */ Key::Unknown,
    /* 5f */ Key::Unknown,
    /* 60 */ Key::F5,
    /* 61 */ Key::F6,
    /* 62 */ Key::F7,
    /* 63 */ Key::F3,
    /* 64 */ Key::F8,
    /* 65 */ Key::F9,
    /* 66 */ Key::Unknown,
    /* 67 */ Key::F11,
    /* 68 */ Key::Unknown,
    /* 69 */ Key::Unknown, // PrintScreen
    /* 6a */ Key::Unknown, // F16
    /* 6b */ Key::F14,
    /* 6c */ Key::Unknown,
    /* 6d */ Key::F10,
    /* 6e */ Key::Unknown,
    /* 6f */ Key::F12,
    /* 70 */ Key::Unknown,
    /* 71 */ Key::F15,
    /* 72 */ Key::Insert, /* Really Help... */
    /* 73 */ Key::Home,
    /* 74 */ Key::PageUp,
    /* 75 */ Key::Delete,
    /* 76 */ Key::F4,
    /* 77 */ Key::End,
    /* 78 */ Key::F2,
    /* 79 */ Key::PageDown,
    /* 7a */ Key::F1,
    /* 7b */ Key::Left,
    /* 7c */ Key::Right,
    /* 7d */ Key::Down,
    /* 7e */ Key::Up,
    /* 7f */ Key::Unknown,
];

#[link(name = "Cocoa", kind = "framework")]
#[link(name = "Carbon", kind = "framework")]
extern "C" {
    fn mfb_open(
        name: *const c_char,
        width: u32,
        height: u32,
        flags: u32,
        scale: i32,
        view_handle: *mut *const c_void,
    ) -> *mut c_void;
    fn mfb_set_title(window: *mut c_void, title: *const c_char);
    fn mfb_close(window: *mut c_void);
    fn mfb_update(window: *mut c_void);
    fn mfb_update_with_buffer(
        window: *mut c_void,
        buffer: *const c_uchar,
        buf_width: u32,
        buf_height: u32,
        buf_stride: u32,
    );
    fn mfb_set_position(window: *mut c_void, x: i32, y: i32);
    fn mfb_get_position(window: *const c_void, x: *mut i32, y: *mut i32);
    fn mfb_set_key_callback(
        window: *mut c_void,
        target: *mut c_void,
        cb: unsafe extern "C" fn(*mut c_void, i32, i32),
        cb: unsafe extern "C" fn(*mut c_void, u32),
    );
    fn mfb_set_mouse_data(window_handle: *mut c_void, shared_data: *mut SharedData);
    fn mfb_set_cursor_style(window: *mut c_void, cursor: u32);
    fn mfb_set_cursor_visibility(window: *mut c_void, visibility: bool);
    fn mfb_should_close(window: *mut c_void) -> i32;
    fn mfb_get_screen_size() -> u32;
    fn mfb_is_active(window: *mut c_void) -> u32;
    fn mfb_add_menu(window: *mut c_void, menu: *mut c_void) -> u64;
    fn mfb_add_sub_menu(parent_menu: *mut c_void, name: *const c_char, menu: *mut c_void);
    fn mfb_active_menu(window: *mut c_void) -> i32;

    fn mfb_create_menu(name: *const c_char) -> *mut c_void;
    fn mfb_remove_menu_at(window: *mut c_void, index: i32);

    /// Sets the whether or not the window is the topmost window
    fn mfb_topmost(window: *mut c_void, topmost: bool);

    fn mfb_add_menu_item(
        menu_item: *mut c_void,
        menu_id: i32,
        name: *const c_char,
        enabled: bool,
        key: u32,
        modifier: u32,
    ) -> u64;
    fn mfb_remove_menu_item(menu: *mut c_void, item_handle: u64);
}

#[derive(Default)]
#[repr(C)]
pub struct SharedData {
    pub bg_color: u32,
    pub scale_mode: u32,
    pub width: u32,
    pub height: u32,
    pub mouse_x: f32,
    pub mouse_y: f32,
    pub scroll_x: f32,
    pub scroll_y: f32,
    pub state: [u8; 8],
}

pub struct Window {
    window_handle: *mut c_void,
    view_handle: *const c_void,
    scale_factor: usize,
    pub shared_data: SharedData,
    key_handler: KeyHandler,
    update_rate: UpdateRate,
    pub has_set_data: bool,
    menus: Vec<MenuHandle>,
}

unsafe extern "C" fn key_callback(window: *mut c_void, key: i32, state: i32) {
    let win: *mut Window = mem::transmute(window);

    let s = state == 1;

    if key > 128 {
        (*win).key_handler.set_key_state(Key::Unknown, s);
    } else {
        (*win)
            .key_handler
            .set_key_state(KEY_MAPPINGS[key as usize], s);
    }
}

unsafe extern "C" fn char_callback(window: *mut c_void, code_point: u32) {
    let win: *mut Window = mem::transmute(window);

    // Taken from GLFW
    if code_point < 32 || (code_point > 126 && code_point < 160) {
        return;
    }

    if let Some(ref mut callback) = (*win).key_handler.key_callback {
        callback.add_char(code_point);
    }
}

unsafe impl raw_window_handle::HasRawWindowHandle for Window {
    fn raw_window_handle(&self) -> raw_window_handle::RawWindowHandle {
        let mut handle = raw_window_handle::AppKitHandle::empty();
        handle.ns_window = self.window_handle as *mut _;
        handle.ns_view = self.view_handle as *mut _;
        raw_window_handle::RawWindowHandle::AppKit(handle)
    }
}

impl Window {
    pub fn new(name: &str, width: usize, height: usize, opts: WindowOptions) -> Result<Window> {
        let n = match CString::new(name) {
            Err(_) => {
                println!("Unable to convert {} to c_string", name);
                return Err(Error::WindowCreate("Unable to set correct name".to_owned()));
            }
            Ok(n) => n,
        };

        unsafe {
            let scale_factor = Self::get_scale_factor(width, height, opts.scale) as usize;
            let mut view_handle = ptr::null();
            let handle = mfb_open(
                n.as_ptr(),
                width as u32,
                height as u32,
                window_flags::get_flags(opts),
                scale_factor as i32,
                &mut view_handle,
            );

            if opts.topmost {
                mfb_topmost(handle, true);
            }

            if handle == ptr::null_mut() {
                return Err(Error::WindowCreate("Unable to open Window".to_owned()));
            }

            Ok(Window {
                window_handle: handle,
                view_handle,
                scale_factor,
                shared_data: SharedData {
                    bg_color: 0,
                    scale_mode: opts.scale_mode as u32,
                    width: width as u32 * scale_factor as u32,
                    height: height as u32 * scale_factor as u32,
                    ..SharedData::default()
                },
                key_handler: KeyHandler::new(),
                update_rate: UpdateRate::new(),
                has_set_data: false,
                menus: Vec::new(),
            })
        }
    }

    #[inline]
    pub fn set_title(&mut self, title: &str) {
        unsafe {
            let t = CString::new(title).unwrap();
            mfb_set_title(self.window_handle, t.as_ptr());
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
    pub fn get_window_handle(&self) -> *mut raw::c_void {
        self.window_handle as *mut raw::c_void
    }

    #[inline]
    unsafe fn set_mouse_data(&mut self) {
        mfb_set_mouse_data(self.window_handle, &mut self.shared_data);
    }

    #[inline]
    pub fn set_background_color(&mut self, color: u32) {
        self.shared_data.bg_color = color;
    }

    #[inline]
    pub fn set_cursor_visibility(&mut self, visibility: bool) {
        unsafe {
            mfb_set_cursor_visibility(self.window_handle, visibility);
        }
    }

    pub fn update_with_buffer_stride(
        &mut self,
        buffer: &[u32],
        buf_width: usize,
        buf_height: usize,
        buf_stride: usize,
    ) -> Result<()> {
        self.key_handler.update();

        buffer_helper::check_buffer_size(buf_width, buf_height, buf_stride, buffer)?;

        unsafe {
            mfb_update_with_buffer(
                self.window_handle,
                buffer.as_ptr() as *const u8,
                buf_width as u32,
                buf_height as u32,
                buf_stride as u32,
            );
            Self::set_mouse_data(self);
            mfb_set_key_callback(
                self.window_handle,
                mem::transmute(self),
                key_callback,
                char_callback,
            );
        }

        Ok(())
    }

    pub fn update(&mut self) {
        self.key_handler.update();

        unsafe {
            mfb_update(self.window_handle);
            Self::set_mouse_data(self);
            mfb_set_key_callback(
                self.window_handle,
                mem::transmute(self),
                key_callback,
                char_callback,
            );
        }
    }

    #[inline]
    pub fn set_position(&mut self, x: isize, y: isize) {
        unsafe { mfb_set_position(self.window_handle, x as i32, y as i32) }
    }

    #[inline]
    pub fn get_position(&self) -> (isize, isize) {
        let (mut x, mut y) = (0, 0);
        unsafe {
            mfb_get_position(self.window_handle, &mut x, &mut y);
        }
        (x as isize, y as isize)
    }

    #[inline]
    pub fn topmost(&self, topmost: bool) {
        unsafe { mfb_topmost(self.window_handle, topmost) }
    }

    pub fn get_size(&self) -> (usize, usize) {
        (
            self.shared_data.width as usize,
            self.shared_data.height as usize,
        )
    }

    pub fn get_scroll_wheel(&self) -> Option<(f32, f32)> {
        let sx = self.shared_data.scroll_x;
        let sy = self.shared_data.scroll_y;

        if sx.abs() > 0.0001 || sy.abs() > 0.0001 {
            Some((sx, sy))
        } else {
            None
        }
    }

    pub fn get_mouse_down(&self, button: MouseButton) -> bool {
        match button {
            MouseButton::Left => self.shared_data.state[0] > 0,
            MouseButton::Middle => self.shared_data.state[1] > 0,
            MouseButton::Right => self.shared_data.state[2] > 0,
        }
    }

    pub fn get_mouse_pos(&self, mode: MouseMode) -> Option<(f32, f32)> {
        let s = self.scale_factor as f32;
        let w = self.shared_data.width as f32;
        let h = self.shared_data.height as f32;

        mouse_handler::get_pos(
            mode,
            self.shared_data.mouse_x,
            self.shared_data.mouse_y,
            s,
            w,
            h,
        )
    }

    pub fn get_unscaled_mouse_pos(&self, mode: MouseMode) -> Option<(f32, f32)> {
        let s = 1.0;
        let w = self.shared_data.width as f32;
        let h = self.shared_data.height as f32;

        mouse_handler::get_pos(
            mode,
            self.shared_data.mouse_x,
            self.shared_data.mouse_y,
            s,
            w,
            h,
        )
    }

    #[inline]
    pub fn set_cursor_style(&mut self, cursor: CursorStyle) {
        unsafe {
            mfb_set_cursor_style(self.window_handle, cursor as u32);
        }
    }

    #[inline]
    pub fn get_keys(&self) -> Vec<Key> {
        self.key_handler.get_keys()
    }

    #[inline]
    pub fn get_keys_pressed(&self, repeat: KeyRepeat) -> Vec<Key> {
        self.key_handler.get_keys_pressed(repeat)
    }

    #[inline]
    pub fn get_keys_released(&self) -> Vec<Key> {
        self.key_handler.get_keys_released()
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

    pub fn is_menu_pressed(&mut self) -> Option<usize> {
        let menu_id = unsafe { mfb_active_menu(self.window_handle) };

        if menu_id < 0 {
            None
        } else {
            Some(menu_id as usize)
        }
    }

    pub fn add_menu(&mut self, menu: &Menu) -> MenuHandle {
        unsafe {
            let handle = MenuHandle(mfb_add_menu(self.window_handle, menu.menu_handle));
            self.menus.push(handle);
            handle
        }
    }

    pub fn remove_menu(&mut self, handle: MenuHandle) {
        for i in 0..self.menus.len() {
            if self.menus[i] == handle {
                self.menus.remove(i);
                unsafe {
                    // + 1 here as we always have a default menu we shouldn't remove
                    mfb_remove_menu_at(self.window_handle, (i + 1) as i32);
                }
                return;
            }
        }
    }

    #[inline]
    pub fn is_open(&self) -> bool {
        unsafe { mfb_should_close(self.window_handle) == 0 }
    }

    #[inline]
    pub fn is_active(&mut self) -> bool {
        unsafe { mfb_is_active(self.window_handle) == 0 }
    }

    unsafe fn get_scale_factor(width: usize, height: usize, scale: Scale) -> i32 {
        let factor: i32 = match scale {
            Scale::X1 => 1,
            Scale::X2 => 2,
            Scale::X4 => 4,
            Scale::X8 => 8,
            Scale::X16 => 16,
            Scale::X32 => 32,
            Scale::FitScreen => {
                let wh: u32 = mfb_get_screen_size();
                let screen_x = (wh >> 16) as i32;
                let screen_y = (wh & 0xffff) as i32;

                let mut scale = 1i32;

                loop {
                    let w = width as i32 * (scale + 1);
                    let h = height as i32 * (scale + 1);

                    if w > screen_x || h > screen_y {
                        break;
                    }

                    scale *= 2;
                }

                scale
            }
        };

        return factor;
    }
}

pub struct Menu {
    menu_handle: *mut c_void,
}

impl Menu {
    pub fn new(name: &str) -> Result<Menu> {
        unsafe {
            let menu_name = CString::new(name).unwrap();
            Ok(Menu {
                menu_handle: mfb_create_menu(menu_name.as_ptr()),
            })
        }
    }

    unsafe fn map_key_to_menu_key(key: Key) -> u32 {
        match key {
            Key::A => 0x00,
            Key::S => 0x01,
            Key::D => 0x02,
            Key::F => 0x03,
            Key::H => 0x04,
            Key::G => 0x05,
            Key::Z => 0x06,
            Key::X => 0x07,
            Key::C => 0x08,
            Key::V => 0x09,
            Key::B => 0x0b,
            Key::Q => 0x0c,
            Key::W => 0x0d,
            Key::E => 0x0e,
            Key::R => 0x0f,
            Key::Y => 0x10,
            Key::T => 0x11,
            Key::Key1 => 0x12,
            Key::Key2 => 0x13,
            Key::Key3 => 0x14,
            Key::Key4 => 0x15,
            Key::Key6 => 0x16,
            Key::Key5 => 0x17,
            Key::Equal => 0x18,
            Key::Key9 => 0x19,
            Key::Key7 => 0x1a,
            Key::Minus => 0x1b,
            Key::Key8 => 0x1c,
            Key::Key0 => 0x1d,
            Key::RightBracket => 0x1e,
            Key::O => 0x1f,
            Key::U => 0x20,
            Key::LeftBracket => 0x21,
            Key::I => 0x22,
            Key::P => 0x23,
            Key::Enter => 0x24,
            Key::L => 0x25,
            Key::J => 0x26,
            Key::Apostrophe => 0x27,
            Key::K => 0x28,
            Key::Semicolon => 0x29,
            Key::Backslash => 0x2a,
            Key::Comma => 0x2b,
            Key::Slash => 0x2c,
            Key::N => 0x2d,
            Key::M => 0x2e,
            Key::Period => 0x2f,
            // Key::Tab => 0x30,
            Key::Space => 0x31,
            // Key::Backspace => 0x33,
            // Key::Escape => 0x35,
            Key::RightSuper => 0x36,
            Key::LeftSuper => 0x37,
            Key::LeftShift => 0x38,
            Key::CapsLock => 0x39,
            Key::LeftAlt => 0x3a,
            Key::LeftCtrl => 0x3b,
            Key::RightShift => 0x3c,
            Key::RightAlt => 0x3d,
            Key::RightCtrl => 0x3e,
            // Key::Equal => 0x51,
            Key::NumPad0 => 0x52,
            Key::NumPad1 => 0x53,
            Key::NumPad2 => 0x54,
            Key::NumPad3 => 0x55,
            Key::NumPad4 => 0x56,
            Key::NumPad5 => 0x57,
            Key::NumPad6 => 0x58,
            Key::NumPad7 => 0x59,
            Key::NumPad8 => 0x5b,
            Key::NumPad9 => 0x5c,
            Key::F5 => 0x60,
            Key::F6 => 0x61,
            Key::F7 => 0x62,
            Key::F3 => 0x63,
            Key::F8 => 0x64,
            Key::F9 => 0x65,
            Key::F11 => 0x67,
            Key::F14 => 0x6b,
            Key::F10 => 0x6d,
            Key::F12 => 0x6f,
            Key::F15 => 0x71,
            Key::Insert => 0x72, /* Really Help... */
            Key::Home => 0x73,
            // Key::PageUp => 0x74,
            Key::Delete => 0x75,
            Key::F4 => 0x76,
            Key::End => 0x77,
            Key::F2 => 0x78,
            // Key::PageDown => 0x79,
            Key::F1 => 0x7a,
            // Key::Left => 0x7b,
            // Key::Right => 0x7c,
            // Key::Down => 0x7d,
            // Key::Up => 0x7e,
            Key::Left => 0x2190,
            Key::Up => 0x2191,
            Key::Down => 0x2193,
            Key::Right => 0x2192,
            Key::Escape => 0x238b,
            // Key::Enter => 0x000d,
            Key::Backspace => 0x232b,
            Key::Tab => 0x21e4,
            Key::PageUp => 0x21de,
            Key::PageDown => 0x21df,
            _ => 0x7f,
        }
    }

    pub fn add_sub_menu(&mut self, name: &str, sub_menu: &Menu) {
        unsafe {
            let menu_name = CString::new(name).unwrap();
            mfb_add_sub_menu(self.menu_handle, menu_name.as_ptr(), sub_menu.menu_handle)
        }
    }

    pub fn add_menu_item(&mut self, item: &MenuItem) -> MenuItemHandle {
        unsafe {
            let item_name = CString::new(item.label.as_str()).unwrap();
            let conv_key = Self::map_key_to_menu_key(item.key);

            MenuItemHandle(mfb_add_menu_item(
                self.menu_handle,
                item.id as i32,
                item_name.as_ptr(),
                item.enabled,
                conv_key,
                item.modifier as u32,
            ))
        }
    }

    pub fn remove_item(&mut self, handle: &MenuItemHandle) {
        unsafe {
            mfb_remove_menu_item(self.menu_handle, handle.0);
        }
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            mfb_close(self.window_handle);
        }
    }
}
