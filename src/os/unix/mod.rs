#![cfg(any(target_os="linux",
    target_os="freebsd",
    target_os="dragonfly",
    target_os="netbsd",
    target_os="openbsd"))]

extern crate x11_dl;

use {MouseMode, MouseButton, Scale, Key, KeyRepeat, WindowOptions, InputCallback};
use key_handler::KeyHandler;
use self::x11_dl::keysym::*;
use self::x11_dl::xlib;

use error::Error;
use Result;
use {CursorStyle, MenuItem, MenuItemHandle, MenuHandle, UnixMenu, UnixMenuItem};

use std::os::raw::{c_void, c_char, c_uchar};
use std::ffi::{CString};
use std::ptr;
use std::mem;
use std::os::raw;
use mouse_handler;
use buffer_helper;
use window_flags;

struct DisplayInfo {
    lib: x11_dl::xlib::Xlib,
    display: *mut xlib::Display,
    screen: i32,
    visual: *mut xlib::Visual,
    gc: xlib::GC,
    depth: i32,
    screen_width: i32,
    screen_height: i32,
    context: xlib::XContext,

/* TODO
    int s_keyb_ext = 0;
    Atom s_wm_delete_window;
*/

// TODO cursors: [10]
}

impl DisplayInfo {
    fn new() -> Result<DisplayInfo> {
        let mut display = Self::setup()?;

        display.check_formats()?;
        display.init_cursors()?;

        Ok(display)
    }

    fn setup() -> Result<DisplayInfo> {
        unsafe {
            // load the Xlib library
            let lib = xlib::Xlib::open();

            if let Err(_) = lib {
                return Err(Error::WindowCreate("failed to load Xlib".to_owned()));
            }

            let lib = lib.unwrap();

            let display = (lib.XOpenDisplay)(ptr::null());

            if display.is_null() {
                return Err(Error::WindowCreate("XOpenDisplay failed".to_owned()));
            }

            let screen = (lib.XDefaultScreen)(display);
            let visual = (lib.XDefaultVisual)(display, screen);
            let gc     = (lib.XDefaultGC)    (display, screen);
            let depth  = (lib.XDefaultDepth) (display, screen);

            let screen_width  = (lib.XDisplayWidth) (display, screen);
            let screen_height = (lib.XDisplayHeight)(display, screen);

            // andrewj: using this instead of XUniqueContext(), as the latter
            // seems to be erroneously feature guarded in the x11_dl crate.
            let context = (lib.XrmUniqueQuark)();

            // TODO keyb_ext

            Ok(DisplayInfo {
                lib,
                display,
                screen,
                visual,
                gc,
                depth,
                screen_width,
                screen_height,
                context,
            })
        }
    }

    fn check_formats(&mut self) -> Result<()> {
        // We only support 32-bit right now
/*
        if (convDepth != 32) {
            printf("Unable to find 32-bit format for X11 display\n");
            XCloseDisplay(s_display);
            return 0;
        } */

        Ok(())
    }

    fn init_cursors(&mut self) -> Result<()> {
        Ok(())
    }
}

impl Drop for DisplayInfo {
    fn drop(&mut self) {
        // FIXME !!!
    }
}


#[derive(Default)]
#[repr(C)]
pub struct SharedData {
    pub width: u32,
    pub height: u32,
    pub scale: f32,
    pub mouse_x: f32,
    pub mouse_y: f32,
    pub scroll_x: f32,
    pub scroll_y: f32,
    pub state: [u8; 3],
}

pub struct Window {
    display: DisplayInfo,

    handle: xlib::Window,

    shared_data: SharedData,
    key_handler: KeyHandler,
    menu_counter: MenuHandle,
    menus: Vec<UnixMenu>,
}

#[allow(non_upper_case_globals)]
unsafe extern "C" fn key_callback(window: *mut c_void, key: i32, s: i32) {
    let win: *mut Window = mem::transmute(window);

    let state = s == 1;

    match key as u32 {
        XK_0 => (*win).key_handler.set_key_state(Key::Key0, state),
        XK_1 => (*win).key_handler.set_key_state(Key::Key1, state),
        XK_2 => (*win).key_handler.set_key_state(Key::Key2, state),
        XK_3 => (*win).key_handler.set_key_state(Key::Key3, state),
        XK_4 => (*win).key_handler.set_key_state(Key::Key4, state),
        XK_5 => (*win).key_handler.set_key_state(Key::Key5, state),
        XK_6 => (*win).key_handler.set_key_state(Key::Key6, state),
        XK_7 => (*win).key_handler.set_key_state(Key::Key7, state),
        XK_8 => (*win).key_handler.set_key_state(Key::Key8, state),
        XK_9 => (*win).key_handler.set_key_state(Key::Key9, state),
        XK_a => (*win).key_handler.set_key_state(Key::A, state),
        XK_b => (*win).key_handler.set_key_state(Key::B, state),
        XK_c => (*win).key_handler.set_key_state(Key::C, state),
        XK_d => (*win).key_handler.set_key_state(Key::D, state),
        XK_e => (*win).key_handler.set_key_state(Key::E, state),
        XK_f => (*win).key_handler.set_key_state(Key::F, state),
        XK_g => (*win).key_handler.set_key_state(Key::G, state),
        XK_h => (*win).key_handler.set_key_state(Key::H, state),
        XK_i => (*win).key_handler.set_key_state(Key::I, state),
        XK_j => (*win).key_handler.set_key_state(Key::J, state),
        XK_k => (*win).key_handler.set_key_state(Key::K, state),
        XK_l => (*win).key_handler.set_key_state(Key::L, state),
        XK_m => (*win).key_handler.set_key_state(Key::M, state),
        XK_n => (*win).key_handler.set_key_state(Key::N, state),
        XK_o => (*win).key_handler.set_key_state(Key::O, state),
        XK_p => (*win).key_handler.set_key_state(Key::P, state),
        XK_q => (*win).key_handler.set_key_state(Key::Q, state),
        XK_r => (*win).key_handler.set_key_state(Key::R, state),
        XK_s => (*win).key_handler.set_key_state(Key::S, state),
        XK_t => (*win).key_handler.set_key_state(Key::T, state),
        XK_u => (*win).key_handler.set_key_state(Key::U, state),
        XK_v => (*win).key_handler.set_key_state(Key::V, state),
        XK_w => (*win).key_handler.set_key_state(Key::W, state),
        XK_x => (*win).key_handler.set_key_state(Key::X, state),
        XK_y => (*win).key_handler.set_key_state(Key::Y, state),
        XK_z => (*win).key_handler.set_key_state(Key::Z, state),
        XK_F1 => (*win).key_handler.set_key_state(Key::F1, state),
        XK_F2 => (*win).key_handler.set_key_state(Key::F2, state),
        XK_F3 => (*win).key_handler.set_key_state(Key::F3, state),
        XK_F4 => (*win).key_handler.set_key_state(Key::F4, state),
        XK_F5 => (*win).key_handler.set_key_state(Key::F5, state),
        XK_F6 => (*win).key_handler.set_key_state(Key::F6, state),
        XK_F7 => (*win).key_handler.set_key_state(Key::F7, state),
        XK_F8 => (*win).key_handler.set_key_state(Key::F8, state),
        XK_F9 => (*win).key_handler.set_key_state(Key::F9, state),
        XK_F10 => (*win).key_handler.set_key_state(Key::F10, state),
        XK_F11 => (*win).key_handler.set_key_state(Key::F11, state),
        XK_F12 => (*win).key_handler.set_key_state(Key::F12, state),
        XK_Down => (*win).key_handler.set_key_state(Key::Down, state),
        XK_Left => (*win).key_handler.set_key_state(Key::Left, state),
        XK_Right => (*win).key_handler.set_key_state(Key::Right, state),
        XK_Up => (*win).key_handler.set_key_state(Key::Up, state),
        XK_Escape => (*win).key_handler.set_key_state(Key::Escape, state),
        XK_apostrophe => (*win).key_handler.set_key_state(Key::Apostrophe, state),
        XK_grave => (*win).key_handler.set_key_state(Key::Backquote, state),
        XK_backslash => (*win).key_handler.set_key_state(Key::Backslash, state),
        XK_comma => (*win).key_handler.set_key_state(Key::Comma, state),
        XK_equal => (*win).key_handler.set_key_state(Key::Equal, state),
        XK_bracketleft => (*win).key_handler.set_key_state(Key::LeftBracket, state),
        XK_minus => (*win).key_handler.set_key_state(Key::Minus, state),
        XK_period => (*win).key_handler.set_key_state(Key::Period, state),
        XK_braceright => (*win).key_handler.set_key_state(Key::RightBracket, state),
        XK_semicolon => (*win).key_handler.set_key_state(Key::Semicolon, state),
        XK_slash => (*win).key_handler.set_key_state(Key::Slash, state),
        XK_BackSpace => (*win).key_handler.set_key_state(Key::Backspace, state),
        XK_Delete => (*win).key_handler.set_key_state(Key::Delete, state),
        XK_End => (*win).key_handler.set_key_state(Key::End, state),
        XK_Return => (*win).key_handler.set_key_state(Key::Enter, state),
        XK_Home => (*win).key_handler.set_key_state(Key::Home, state),
        XK_Insert => (*win).key_handler.set_key_state(Key::Insert, state),
        XK_Menu => (*win).key_handler.set_key_state(Key::Menu, state),
        XK_Page_Down => (*win).key_handler.set_key_state(Key::PageDown, state),
        XK_Page_Up => (*win).key_handler.set_key_state(Key::PageUp, state),
        XK_Pause => (*win).key_handler.set_key_state(Key::Pause, state),
        XK_space => (*win).key_handler.set_key_state(Key::Space, state),
        XK_Tab => (*win).key_handler.set_key_state(Key::Tab, state),
        XK_Num_Lock => (*win).key_handler.set_key_state(Key::NumLock, state),
        XK_Caps_Lock => (*win).key_handler.set_key_state(Key::CapsLock, state),
        XK_Scroll_Lock => (*win).key_handler.set_key_state(Key::ScrollLock, state),
        XK_Shift_L => (*win).key_handler.set_key_state(Key::LeftShift, state),
        XK_Shift_R => (*win).key_handler.set_key_state(Key::RightShift, state),
        XK_Control_L => (*win).key_handler.set_key_state(Key::LeftCtrl, state),
        XK_Control_R => (*win).key_handler.set_key_state(Key::RightCtrl, state),
        XK_KP_0 => (*win).key_handler.set_key_state(Key::NumPad0, state),
        XK_KP_1 => (*win).key_handler.set_key_state(Key::NumPad1, state),
        XK_KP_2 => (*win).key_handler.set_key_state(Key::NumPad2, state),
        XK_KP_3 => (*win).key_handler.set_key_state(Key::NumPad3, state),
        XK_KP_4 => (*win).key_handler.set_key_state(Key::NumPad4, state),
        XK_KP_5 => (*win).key_handler.set_key_state(Key::NumPad5, state),
        XK_KP_6 => (*win).key_handler.set_key_state(Key::NumPad6, state),
        XK_KP_7 => (*win).key_handler.set_key_state(Key::NumPad7, state),
        XK_KP_8 => (*win).key_handler.set_key_state(Key::NumPad8, state),
        XK_KP_9 => (*win).key_handler.set_key_state(Key::NumPad9, state),
        XK_KP_Decimal => (*win).key_handler.set_key_state(Key::NumPadDot, state),
        XK_KP_Divide => (*win).key_handler.set_key_state(Key::NumPadSlash, state),
        XK_KP_Multiply => (*win).key_handler.set_key_state(Key::NumPadAsterisk, state),
        XK_KP_Subtract => (*win).key_handler.set_key_state(Key::NumPadMinus, state),
        XK_KP_Add => (*win).key_handler.set_key_state(Key::NumPadPlus, state),
        XK_KP_Enter => (*win).key_handler.set_key_state(Key::NumPadEnter, state),
        XK_Super_L => (*win).key_handler.set_key_state(Key::LeftSuper, state),
        XK_Super_R => (*win).key_handler.set_key_state(Key::RightSuper, state),
    	_ => (),
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

impl Window {
    pub fn new(name: &str, width: usize, height: usize, opts: WindowOptions) -> Result<Window> {
        let n = match CString::new(name) {
            Err(_) => {
                println!("Unable to convert {} to c_string", name);
                return Err(Error::WindowCreate("Unable to set correct name".to_owned()));
            }
            Ok(n) => n,
        };

        let display = DisplayInfo::new()?;

        let scale = Self::get_scale_factor(width, height, opts.scale);
        let handle: xlib::Window = 0;  // FIXME

// !!                        mfb_open(n.as_ptr(),
// !!            					  width as u32,
// !!            					  height as u32,
// !!            					  window_flags::get_flags(opts),
// !!            					  scale);

        if handle == 0 {
            return Err(Error::WindowCreate("Unable to open Window".to_owned()));
        }

        Ok(Window {
            display,
            handle: 0, // !!!! FIXME

            shared_data: SharedData {
                scale: scale as f32,
                .. SharedData::default()
            },
            key_handler: KeyHandler::new(),
            menu_counter: MenuHandle(0),
            menus: Vec::new(),
        })
    }

    pub fn set_title(&mut self, title: &str) {
// !!        unsafe {
// !!            let t = CString::new(title).unwrap();
// !!            mfb_set_title(self.window_handle, t.as_ptr());
// !!        }

/*
        WindowInfo* info = (WindowInfo*)window_info;
        XStoreName(s_display, info->window, title);   */
    }

    unsafe fn set_shared_data(&mut self) {
// !!        mfb_set_shared_data(self.window_handle, &mut self.shared_data);
    }

    pub fn update_with_buffer(&mut self, buffer: &[u32]) -> Result<()> {
        self.key_handler.update();

        unsafe {
            Self::set_shared_data(self);
        }

        let check_res = buffer_helper::check_buffer_size(self.shared_data.width as usize,
                                                         self.shared_data.height as usize,
                                                         self.shared_data.scale as usize,
                                                         buffer);
        if check_res.is_err() {
            return check_res;
        }

        unsafe {
// !!            mfb_update_with_buffer(self.window_handle, buffer.as_ptr() as *const u8);
// !!            mfb_set_key_callback(self.window_handle,
// !!            					 mem::transmute(self),
// !!            					 key_callback,
// !!            					 char_callback);
        }

        Ok(())
    }

    pub fn update(&mut self) {
        self.key_handler.update();

// !!        unsafe {
// !!            Self::set_shared_data(self);
// !!            mfb_update(self.window_handle);
// !!            mfb_set_key_callback(self.window_handle,
// !!            					 mem::transmute(self),
// !!            					 key_callback,
// !!            					 char_callback);
// !!        }
    }

    #[inline]
    pub fn get_window_handle(&self) -> *mut raw::c_void {
// !!    	unsafe { mfb_get_window_handle(self.window_handle) as *mut raw::c_void }
        unsafe { mem::transmute(self.handle as usize) }
    }

    #[inline]
    pub fn set_position(&mut self, x: isize, y: isize) {
// !!        unsafe { mfb_set_position(self.window_handle, x as i32, y as i32) }
    }

    #[inline]
    pub fn get_size(&self) -> (usize, usize) {
        (self.shared_data.width as usize, self.shared_data.height as usize)
    }

    pub fn get_mouse_pos(&self, mode: MouseMode) -> Option<(f32, f32)> {
        let s = self.shared_data.scale as f32;
        let w = self.shared_data.width as f32;
        let h = self.shared_data.height as f32;

        mouse_handler::get_pos(mode, self.shared_data.mouse_x, self.shared_data.mouse_y, s, w, h)
    }

    pub fn get_unscaled_mouse_pos(&self, mode: MouseMode) -> Option<(f32, f32)> {
        let w = self.shared_data.width as f32;
        let h = self.shared_data.height as f32;

        mouse_handler::get_pos(mode, self.shared_data.mouse_x, self.shared_data.mouse_y, 1.0, w, h)
    }

    pub fn get_mouse_down(&self, button: MouseButton) -> bool {
    	match button {
    		MouseButton::Left => self.shared_data.state[0] > 0,
    		MouseButton::Middle => self.shared_data.state[1] > 0,
    		MouseButton::Right => self.shared_data.state[2] > 0,
    	}
    }

    pub fn get_scroll_wheel(&self) -> Option<(f32, f32)> {
        if self.shared_data.scroll_x.abs() > 0.0 ||
           self.shared_data.scroll_y.abs() > 0.0 {
            Some((self.shared_data.scroll_x, self.shared_data.scroll_y))
        } else {
            None
        }
    }

    #[inline]
    pub fn set_cursor_style(&mut self, cursor: CursorStyle) {
// !!        unsafe {
// !!            mfb_set_cursor_style(self.window_handle, cursor as u32);
// !!        }
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
    pub fn set_input_callback(&mut self, callback: Box<InputCallback>)  {
        self.key_handler.set_input_callback(callback)
    }

    #[inline]
    pub fn is_open(&self) -> bool {
// !!        unsafe { mfb_should_close(self.window_handle) == 1 }
        true
    }

    #[inline]
    pub fn is_active(&mut self) -> bool {
        // TODO: Proper implementation
        true
    }

    fn get_scale_factor(width: usize, height: usize, scale: Scale) -> i32 {
        let factor: i32 = match scale {
            Scale::X1 => 1,
            Scale::X2 => 2,
            Scale::X4 => 4,
            Scale::X8 => 8,
            Scale::X16 => 16,
            Scale::X32 => 32,
            Scale::FitScreen => {
// !!                 let wh: u32 = mfb_get_screen_size();
                let wh: u32 = unimplemented!();
                let screen_x = (wh >> 16) as i32;
                let screen_y = (wh & 0xffff) as i32;

                println!("{} - {}", screen_x, screen_y);

                let mut scale = 1i32;

                loop {
                    let w = width as i32 * (scale + 1);
                    let h = height as i32 * (scale + 1);

                    if w >= screen_x || h >= screen_y {
                        break;
                    }

                    scale *= 2;
                }

                if scale >= 32 {
                	32
                } else {
                	scale
				}
            }
        };

        return factor;
    }

    fn next_menu_handle(&mut self) -> MenuHandle {
    	let handle = self.menu_counter;
    	self.menu_counter.0 += 1;
    	handle
    }

    pub fn add_menu(&mut self, menu: &Menu) -> MenuHandle {
    	let handle = self.next_menu_handle();
        let mut menu = menu.internal.clone();
        menu.handle = handle;
        self.menus.push(menu);
        handle
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

pub struct Menu {
	pub internal: UnixMenu,
}

impl Menu {
    pub fn new(name: &str) -> Result<Menu> {
    	Ok(Menu {
    		internal: UnixMenu {
				handle: MenuHandle(0),
				item_counter: MenuItemHandle(0),
				name: name.to_owned(),
				items: Vec::new(),
    		}
    	})
    }

    pub fn add_sub_menu(&mut self, name: &str, sub_menu: &Menu) {
    	let handle = self.next_item_handle();
    	self.internal.items.push(UnixMenuItem {
    	    label: name.to_owned(),
    	    handle: handle,
    	    sub_menu: Some(Box::new(sub_menu.internal.clone())),
			id: 0,
			enabled: true,
			key: Key::Unknown,
			modifier: 0,
        });
    }

    fn next_item_handle(&mut self) -> MenuItemHandle {
    	let handle = self.internal.item_counter;
    	self.internal.item_counter.0 += 1;
    	handle
    }

    pub fn add_menu_item(&mut self, item: &MenuItem) -> MenuItemHandle {
    	let item_handle = self.next_item_handle();
    	self.internal.items.push(UnixMenuItem {
			sub_menu: None,
			handle: self.internal.item_counter,
			id: item.id,
			label: item.label.clone(),
			enabled: item.enabled,
			key: item.key,
			modifier: item.modifier,
    	});
    	item_handle
    }

    pub fn remove_item(&mut self, handle: &MenuItemHandle) {
        self.internal.items.retain(|ref item| item.handle.0 != handle.0);
    }
}


impl Drop for Window {
    fn drop(&mut self) {
// !!        unsafe {
// !!            mfb_close(self.window_handle);
// !!        }
    }
}

