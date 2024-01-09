#![cfg(target_os = "redox")]

use crate::os::redox::orbclient::Renderer;

use crate::buffer_helper;
use crate::error::Error;
use crate::icon::Icon;
use crate::key_handler::KeyHandler;
use crate::InputCallback;
use crate::Result;
use crate::{CursorStyle, MouseButton, MouseMode};
use crate::{Key, KeyRepeat};
use crate::{MenuHandle, MenuItem, MenuItemHandle, UnixMenu, UnixMenuItem};
use crate::{Scale, WindowOptions};

use orbclient::Renderer;
use std::cmp;
use std::os::raw;

pub struct Window {
    is_open: bool,
    is_active: bool,
    mouse_pos: Option<(i32, i32)>,
    mouse_scroll: Option<(i32, i32)>,
    /// The state of the left, middle and right mouse buttons
    mouse_state: (bool, bool, bool),
    buffer_width: usize,
    buffer_height: usize,
    window: orbclient::Window,
    window_scale: usize,
    key_handler: KeyHandler,
    menu_counter: MenuHandle,
    menus: Vec<UnixMenu>,
}

impl Window {
    pub fn new(name: &str, width: usize, height: usize, opts: WindowOptions) -> Result<Window> {
        let window_scale = match opts.scale {
            Scale::X1 => 1,
            Scale::X2 => 2,
            Scale::X4 => 4,
            Scale::X8 => 8,
            Scale::X16 => 16,
            Scale::X32 => 32,
            Scale::FitScreen => {
                let display_size = orbclient::get_display_size()
                    .map_err(|_| Error::WindowCreate("Unable to get display size".to_owned()))?;
                let mut scale = 32;
                while scale > 1 {
                    if width * scale < display_size.0 as usize
                        && height * scale < display_size.1 as usize
                    {
                        break;
                    }
                    scale -= 1;
                }
                scale
            }
        };

        let window_width = width as u32 * window_scale as u32;
        let window_height = height as u32 * window_scale as u32;

        let mut window_flags = vec![orbclient::WindowFlag::Async];
        if opts.resize && !opts.none {
            window_flags.push(orbclient::WindowFlag::Resizable);
        }
        if !opts.title {
            window_flags.push(orbclient::WindowFlag::Borderless);
        }
        if opts.transparency {
            window_flags.push(orbclient::WindowFlag::Transparent);
        }

        let window_opt =
            orbclient::Window::new_flags(-1, -1, window_width, window_height, name, &window_flags);
        match window_opt {
            Some(window) => Ok(Window {
                mouse_pos: None,
                mouse_scroll: None,
                mouse_state: (false, false, false),
                is_open: true,
                is_active: true,
                buffer_width: width,
                buffer_height: height,
                window,
                window_scale,
                key_handler: KeyHandler::new(),
                menu_counter: MenuHandle(0),
                menus: Vec::new(),
            }),
            None => Err(Error::WindowCreate("Unable to open Window".to_owned())),
        }
    }

    #[inline]
    pub fn set_title(&mut self, title: &str) {
        self.window.set_title(title)
    }

    #[inline]
    pub fn set_icon(&mut self, _icon: Icon) {
        unimplemented!("Currenty not implemented on RedoxOS")
    }

    #[inline]
    pub fn get_window_handle(&self) -> *mut raw::c_void {
        0 as *mut raw::c_void
    }

    pub fn update_with_buffer(&mut self, buffer: &[u32]) -> Result<()> {
        self.process_events();
        self.key_handler.update();

        let check_res = buffer_helper::check_buffer_size(
            buffer,
            self.buffer_width,
            self.buffer_height,
            self.window_scale,
        );
        if check_res.is_err() {
            return check_res;
        }

        self.render_buffer(buffer);
        self.window.sync();

        Ok(())
    }

    #[inline]
    pub fn update(&mut self) {
        self.process_events();
        self.key_handler.update();
        self.window.sync();
    }

    #[inline]
    pub fn set_position(&mut self, x: isize, y: isize) {
        self.window.set_pos(x as i32, y as i32)
    }

    #[inline]
    pub fn get_size(&self) -> (usize, usize) {
        (self.window.width() as usize, self.window.height() as usize)
    }

    #[inline]
    pub fn get_scroll_wheel(&self) -> Option<(f32, f32)> {
        if let Some((scroll_x, scroll_y)) = self.mouse_scroll {
            Some((scroll_x as f32, scroll_y as f32))
        } else {
            None
        }
    }

    #[inline]
    pub fn get_mouse_down(&self, button: MouseButton) -> bool {
        match button {
            MouseButton::Left => self.mouse_state.0,
            MouseButton::Middle => self.mouse_state.1,
            MouseButton::Right => self.mouse_state.2,
        }
    }

    #[inline]
    pub fn get_mouse_pos(&self, mode: MouseMode) -> Option<(f32, f32)> {
        if let Some((mouse_x, mouse_y)) = self.mouse_pos {
            mode.get_pos(
                mouse_x as f32,
                mouse_y as f32,
                self.window_scale as f32,
                self.buffer_width as f32 * self.window_scale as f32,
                self.buffer_height as f32 * self.window_scale as f32,
            )
        } else {
            None
        }
    }

    #[inline]
    pub fn get_unscaled_mouse_pos(&self, mode: MouseMode) -> Option<(f32, f32)> {
        if let Some((mouse_x, mouse_y)) = self.mouse_pos {
            mode.get_pos(
                mouse_x as f32,
                mouse_y as f32,
                1.0 as f32,
                self.buffer_width as f32 * self.window_scale as f32,
                self.buffer_height as f32 * self.window_scale as f32,
            )
        } else {
            None
        }
    }

    #[inline]
    pub fn set_cursor_style(&mut self, _cursor: CursorStyle) {
        // Orbital doesn't support cursor styles yet
    }

    #[inline]
    pub fn set_cursor_visibility(&mut self, visibility: bool) {
        self.window.set_mouse_cursor(visibility);
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
    pub fn set_input_callback(&mut self, callback: Box<InputCallback>) {
        self.key_handler.set_input_callback(callback)
    }

    #[inline]
    pub fn is_open(&self) -> bool {
        self.is_open
    }

    #[inline]
    pub fn is_active(&mut self) -> bool {
        self.is_active
    }

    fn process_events(&mut self) {
        self.mouse_scroll = None;

        for event in self.window.events() {
            match event.to_option() {
                orbclient::EventOption::Key(key_event) => {
                    let key_opt = self.map_key_to_minifb(key_event.scancode);
                    if let Some(key) = key_opt {
                        self.key_handler.set_key_state(key, key_event.pressed);
                    }
                }
                orbclient::EventOption::Mouse(mouse_event) => {
                    self.mouse_pos = Some((mouse_event.x, mouse_event.y));
                }
                orbclient::EventOption::Button(button_event) => {
                    self.mouse_state = (button_event.left, button_event.middle, button_event.right);
                }
                orbclient::EventOption::Quit(_) => {
                    self.is_open = false;
                }
                orbclient::EventOption::Focus(focus_event) => {
                    self.is_active = focus_event.focused;
                    if !self.is_active {
                        self.mouse_pos = None;
                    }
                }
                orbclient::EventOption::Scroll(scroll_event) => {
                    self.mouse_pos = Some((scroll_event.x, scroll_event.y));
                }
                _ => {}
            }
        }
    }

    /// Maps Orbital scancodes to MiniFB Key enums
    fn map_key_to_minifb(&self, scancode: u8) -> Option<Key> {
        match scancode {
            orbclient::K_0 => Some(Key::Key0),
            orbclient::K_1 => Some(Key::Key1),
            orbclient::K_2 => Some(Key::Key2),
            orbclient::K_3 => Some(Key::Key3),
            orbclient::K_4 => Some(Key::Key4),
            orbclient::K_5 => Some(Key::Key5),
            orbclient::K_6 => Some(Key::Key6),
            orbclient::K_7 => Some(Key::Key7),
            orbclient::K_8 => Some(Key::Key8),
            orbclient::K_9 => Some(Key::Key9),
            orbclient::K_A => Some(Key::A),
            orbclient::K_B => Some(Key::B),
            orbclient::K_C => Some(Key::C),
            orbclient::K_D => Some(Key::D),
            orbclient::K_E => Some(Key::E),
            orbclient::K_F => Some(Key::F),
            orbclient::K_G => Some(Key::G),
            orbclient::K_H => Some(Key::H),
            orbclient::K_I => Some(Key::I),
            orbclient::K_J => Some(Key::J),
            orbclient::K_K => Some(Key::K),
            orbclient::K_L => Some(Key::L),
            orbclient::K_M => Some(Key::M),
            orbclient::K_N => Some(Key::N),
            orbclient::K_O => Some(Key::O),
            orbclient::K_P => Some(Key::P),
            orbclient::K_Q => Some(Key::Q),
            orbclient::K_R => Some(Key::R),
            orbclient::K_S => Some(Key::S),
            orbclient::K_T => Some(Key::T),
            orbclient::K_U => Some(Key::U),
            orbclient::K_V => Some(Key::V),
            orbclient::K_W => Some(Key::W),
            orbclient::K_X => Some(Key::X),
            orbclient::K_Y => Some(Key::Y),
            orbclient::K_Z => Some(Key::Z),
            orbclient::K_F1 => Some(Key::F1),
            orbclient::K_F2 => Some(Key::F2),
            orbclient::K_F3 => Some(Key::F3),
            orbclient::K_F4 => Some(Key::F4),
            orbclient::K_F5 => Some(Key::F5),
            orbclient::K_F6 => Some(Key::F6),
            orbclient::K_F7 => Some(Key::F7),
            orbclient::K_F8 => Some(Key::F8),
            orbclient::K_F9 => Some(Key::F9),
            orbclient::K_F10 => Some(Key::F10),
            orbclient::K_F11 => Some(Key::F11),
            orbclient::K_F12 => Some(Key::F12),
            orbclient::K_DOWN => Some(Key::Down),
            orbclient::K_LEFT => Some(Key::Left),
            orbclient::K_RIGHT => Some(Key::Right),
            orbclient::K_UP => Some(Key::Up),
            orbclient::K_TICK => Some(Key::Apostrophe),
            orbclient::K_BACKSLASH => Some(Key::Backslash),
            orbclient::K_COMMA => Some(Key::Comma),
            orbclient::K_EQUALS => Some(Key::Equal),
            orbclient::K_BRACE_OPEN => Some(Key::LeftBracket),
            orbclient::K_MINUS => Some(Key::Minus),
            orbclient::K_PERIOD => Some(Key::Period),
            orbclient::K_BRACE_CLOSE => Some(Key::RightBracket),
            orbclient::K_SEMICOLON => Some(Key::Semicolon),
            orbclient::K_SLASH => Some(Key::Slash),
            orbclient::K_BKSP => Some(Key::Backspace),
            orbclient::K_DEL => Some(Key::Delete),
            orbclient::K_END => Some(Key::End),
            orbclient::K_ENTER => Some(Key::Enter),
            orbclient::K_ESC => Some(Key::Escape),
            orbclient::K_HOME => Some(Key::Home),
            orbclient::K_PGDN => Some(Key::PageDown),
            orbclient::K_PGUP => Some(Key::PageUp),
            orbclient::K_SPACE => Some(Key::Space),
            orbclient::K_TAB => Some(Key::Tab),
            orbclient::K_CAPS => Some(Key::CapsLock),
            orbclient::K_LEFT_SHIFT => Some(Key::LeftShift),
            orbclient::K_RIGHT_SHIFT => Some(Key::RightShift),
            orbclient::K_CTRL => Some(Key::LeftCtrl),
            orbclient::K_ALT => Some(Key::LeftAlt),
            _ => {
                println!("Unknown Orbital scancode 0x{:2x}", scancode);
                None
            }
        }
    }

    /// Renders the given pixel data into the Orbital window
    fn render_buffer(&mut self, buffer: &[u32]) {
        let render_width = cmp::min(
            self.buffer_width * self.window_scale,
            self.window.width() as usize,
        );
        let render_height = cmp::min(
            self.buffer_height * self.window_scale,
            self.window.height() as usize,
        );

        let window_width = self.window.width() as usize;
        let window_buffer = self.window.data_mut();

        for y in 0..render_height {
            for x in 0..render_width {
                let buffer_x = x / self.window_scale;
                let buffer_y = y / self.window_scale;

                window_buffer[y * window_width + x] = orbclient::Color {
                    data: buffer[buffer_y * self.buffer_width + buffer_x],
                };
            }
        }
    }

    #[inline]
    fn next_menu_handle(&mut self) -> MenuHandle {
        let handle = self.menu_counter;
        self.menu_counter.0 += 1;
        handle
    }

    #[inline]
    pub fn add_menu(&mut self, menu: &Menu) -> MenuHandle {
        let handle = self.next_menu_handle();
        let mut menu = menu.internal.clone();
        menu.handle = handle;
        self.menus.push(menu);
        handle
    }

    #[inline]
    pub fn get_posix_menus(&self) -> Option<&Vec<UnixMenu>> {
        Some(&self.menus)
    }

    #[inline]
    pub fn remove_menu(&mut self, handle: MenuHandle) {
        self.menus.retain(|ref menu| menu.handle != handle);
    }

    #[inline]
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
            },
        })
    }

    #[inline]
    pub fn add_sub_menu(&mut self, name: &str, sub_menu: &Menu) {
        let handle = self.next_item_handle();
        self.internal.items.push(UnixMenuItem {
            label: name.to_owned(),
            handle,
            sub_menu: Some(Box::new(sub_menu.internal.clone())),
            id: 0,
            enabled: true,
            key: Key::Unknown,
            modifier: 0,
        });
    }

    #[inline]
    fn next_item_handle(&mut self) -> MenuItemHandle {
        let handle = self.internal.item_counter;
        self.internal.item_counter.0 += 1;
        handle
    }

    #[inline]
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

    #[inline]
    pub fn remove_item(&mut self, handle: &MenuItemHandle) {
        self.internal
            .items
            .retain(|ref item| item.handle.0 != handle.0);
    }
}
