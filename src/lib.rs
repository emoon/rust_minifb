/// Scale will scale the frame buffer and the window that is being sent in when calling the update
/// function. This is useful if you for example want to display a 320 x 256 window on a screen with
/// much higher resolution which would result in that the window is very small.
#[derive(Clone, Copy)]
pub enum Scale {
    /// This mode checks your current screen resolution and will caluclate the largest window size
    /// that can be used within that limit and resize it. Useful if you have a small buffer to
    /// display on a high resolution screen.
    FitScreen,
    /// 1X scale (which means leave the corrdinates sent into Window::new untouched)
    X1,
    /// 2X window scale (Example: 320 x 200 -> 640 x 400)
    X2,
    /// 4X window scale (Example: 320 x 200 -> 1280 x 800)
    X4,
    /// 8X window scale (Example: 320 x 200 -> 2560 x 1600)
    X8,
    /// 16X window scale (Example: 320 x 200 -> 5120 x 3200)
    X16,
    /// 32 window scale (Example: 320 x 200 -> 10240 x 6400)
    X32,
}

/// Used for is_key_pressed and get_keys_pressed() to indicated if repeat of presses is wanted
#[derive(PartialEq, Clone, Copy)]
pub enum KeyRepeat {
    /// Use repeat
    Yes,
    /// Don't use repeat
    No,
}

/// The various mouse buttons that are availible
#[derive(PartialEq, Clone, Copy)]
pub enum MouseButton
{
    /// Left mouse button
    Left,
    /// Middle mouse button
    Middle,
    /// Right mouse button
    Right,
}


/// Key is used by the get key functions to check if some keys on the keyboard has been pressed
#[derive(PartialEq, Clone, Copy)]
pub enum MouseMode {
    /// Return mouse coords from outside of the window (may be negative)
    Pass,
    /// Clamp the mouse coordinates within the window
    Clamp,
    /// Discared if the mouse is outside the window
    Discard,
}

/// This trait can be implemented and set with ```set_input_callback``` to reieve a callback
/// whene there is inputs incoming. Currently only support unicode chars.
pub trait InputCallback {
    fn add_char(&mut self, uni_char: u32);
}

extern crate libc;

use std::os::raw;

#[doc(hidden)]
mod error;
pub use self::error::Error;
pub type Result<T> = std::result::Result<T, Error>;

pub mod key;
pub use key::Key as Key;
pub mod os;
mod mouse_handler;
mod key_handler;
mod window_flags;
//mod menu;
//pub use menu::Menu as Menu;
//pub use menu::MENU_KEY_COMMAND;
//pub use menu::MENU_KEY_WIN;
//pub use menu::MENU_KEY_SHIFT;
//pub use menu::MENU_KEY_CTRL;
//pub use menu::MENU_KEY_ALT;


#[cfg(target_os = "macos")]
use self::os::macos as imp;
#[cfg(target_os = "windows")]
use self::os::windows as imp;
#[cfg(any(target_os="linux",
    target_os="freebsd",
    target_os="dragonfly",
    target_os="netbsd",
    target_os="openbsd"))]
use self::os::unix as imp;

pub struct Window(imp::Window);

///
/// WindowOptions is creation settings for the window. By default the settings are defined for
/// displayng a 32-bit buffer (no scaling of window is possible)
///
pub struct WindowOptions {
    /// If the window should be borderless (default: false)
    pub borderless: bool,
    /// If the window should have a title (default: true)
    pub title: bool,
    /// If it should be possible to resize the window (default: false)
    pub resize: bool,
    /// Scale of the window that used in conjunction with update_with_buffer (default: X1)
    pub scale: Scale
}

///
/// Window is used to open up a window. It's possible to optionally display a 32-bit buffer when
/// the widow is set as non-resizable.
///
/// # Examples
///
/// Open up a window and display a 32-bit RGB buffer (without error checking)
///
/// ```ignore
/// const WIDTH: usize = 640;
/// const HEIGHT: usize = 360;
///
/// let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];
///
/// let mut window = match Window::new("Test - Press ESC to exit", WIDTH, HEIGHT,
///                                     WindowOptions::default()).unwrap()
///
/// while window.is_open() && !window.is_key_down(Key::Escape) {
///     for i in buffer.iter_mut() {
///         *i = 0; // write something interesting here
///     }
///     window.update_with_buffer(&buffer);
/// }
/// ```
///
impl Window {
    ///
    /// Opens up a new window
    ///
    /// # Examples
    ///
    /// Open up a window with default settings
    ///
    /// ```ignore
    /// let mut window = match Window::new("Test", 640, 400, WindowOptions::default()) {
    ///    Ok(win) => win,
    ///    Err(err) => {
    ///        println!("Unable to create window {}", err);
    ///        return;
    ///    }
    ///};
    /// ```
    ///
    /// Open up a window that is resizeable
    ///
    /// ```ignore
    /// let mut window = match Window::new("Test", 640, 400,
    ///                                     WindowOptions {
    ///                                         resize: true,
    ///                                         ..WindowOptions::default()
    ///                                     }) {
    ///    Ok(win) => win,
    ///    Err(err) => {
    ///        println!("Unable to create window {}", err);
    ///        return;
    ///    }
    ///};
    /// ```
    pub fn new(name: &str, width: usize, height: usize, opts: WindowOptions) -> Result<Window> {
        imp::Window::new(name, width, height, opts).map(Window)
    }

    ///
    /// Returns the native handle for a window which is an opaque pointer/handle which
    /// dependens on the current operating system:
    ///
    /// ```ignore
    /// Windows HWND
    /// MacOS   NSWindow
    /// X11     XWindow
    /// ```
    ///
    #[inline]
    pub fn get_window_handle(&self) -> *mut raw::c_void {
        self.0.get_window_handle()
    }

    ///
    /// Updates the window with a 32-bit pixel buffer. Notice that the buffer needs to be at least
    /// the size of the created window
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mut buffer: Vec<u32> = vec![0; 640 * 400];
    ///
    /// let mut window = match Window::new("Test", 640, 400, WindowOptions::default()).unwrap();
    ///
    /// window.update_with_buffer(&buffer);
    /// ```
    #[inline]
    pub fn update_with_buffer(&mut self, buffer: &[u32]) {
        self.0.update_with_buffer(buffer)
    }

    ///
    /// Updates the window (this is required to call in order to get keyboard/mouse input, etc)
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mut buffer: Vec<u32> = vec![0; 640 * 400];
    ///
    /// let mut window = match Window::new("Test", 640, 400, WindowOptions::default()).unwrap();
    ///
    /// window.update();
    /// ```
    #[inline]
    pub fn update(&mut self) {
        self.0.update()
    }

    ///
    /// Checks if the window is still open. A window can be closed by the user (by for example
    /// pressing the close button on the window) It's up to the user to make sure that this is
    /// being checked and take action depending on the state.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// while window.is_open() {
    ///     window.update(...)
    /// }
    /// ```
    #[inline]
    pub fn is_open(&self) -> bool {
        self.0.is_open()
    }

    ///
    /// Sets the position of the window. This is useful if you have
    /// more than one window and want to align them up on the screen
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Moves the window to pixel position 20, 20 on the screen
    /// window.set_position(20, 20);
    /// ```
    ///
    #[inline]
    pub fn set_position(&mut self, x: isize, y: isize) {
        self.0.set_position(x, y)
    }

    ///
    /// Returns the current size of the window
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let size = window.get_size();
    /// println!("width {} height {}", size.0, size.1);
    /// ```
    ///
    #[inline]
    pub fn get_size(&self) -> (usize, usize) {
        self.0.get_size()
    }

    ///
    /// Get the current position of the mouse relative to the current window
    /// The coordinate system is as 0, 0 as the upper left corner
    ///
    /// # Examples
    ///
    /// ```ignore
    /// window.get_mouse_pos(MouseMode::Clamp).map(|mouse| {
    ///     println!("x {} y {}", mouse.0, mouse.1);
    /// });
    /// ```
    ///
    #[inline]
    pub fn get_mouse_pos(&self, mode: MouseMode) -> Option<(f32, f32)> {
        self.0.get_mouse_pos(mode)
    }

    ///
    /// Check if a mouse button is down or not
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let left_down = window.get_mouse_down(MouseButton::Left);
    /// println!("is left down? {}", left_down)
    /// ```
    ///
    #[inline]
    pub fn get_mouse_down(&self, button: MouseButton) -> bool {
        self.0.get_mouse_down(button)
    }

    ///
    /// Get the current movement of the scroll wheel.
    /// Scroll wheel can mean different thing depending on the device attach.
    /// For example on Mac with trackpad "scroll wheel" means two finger
    /// swiping up/down (y axis) and to the sides (x-axis)
    /// When using a mouse this assumes the scroll wheel which often is only y direction.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// window.get_scroll_wheel().map(|scroll| {
    ///     println!("scrolling - x {} y {}", scroll.0, scroll.1);
    /// });
    /// ```
    ///
    ///
    #[inline]
    pub fn get_scroll_wheel(&self) -> Option<(f32, f32)> {
        self.0.get_scroll_wheel()
    }

    ///
    /// Get the current keys that are down.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// window.get_keys().map(|keys| {
    ///     for t in keys {
    ///         match t {
    ///             Key::W => println!("holding w"),
    ///             Key::T => println!("holding t"),
    ///             _ => (),
    ///         }
    ///     }
    /// });
    /// ```
    #[inline]
    pub fn get_keys(&self) -> Option<Vec<Key>> {
        self.0.get_keys()
    }

    ///
    /// Get the current pressed keys. Repeat can be used to control if keys should
    /// be repeated if down or not.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// window.get_keys_pressed(KeyRepeat::No).map(|keys| {
    ///     for t in keys {
    ///         match t {
    ///             Key::W => println!("pressed w"),
    ///             Key::T => println!("pressed t"),
    ///             _ => (),
    ///         }
    ///     }
    /// });
    /// ```
    #[inline]
    pub fn get_keys_pressed(&self, repeat: KeyRepeat) -> Option<Vec<Key>> {
        self.0.get_keys_pressed(repeat)
    }

    ///
    /// Check if a single key is down.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// if window.is_key_down(Key::A) {
    ///     println!("Key A is down");
    /// }
    /// ```
    ///
    #[inline]
    pub fn is_key_down(&self, key: Key) -> bool {
        self.0.is_key_down(key)
    }

    ///
    /// Check if a single key is pressed. KeyRepeat will control if the key should be repeated or
    /// not while being pressed.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// if window.is_key_pressed(KeyRepeat::No) {
    ///     println!("Key A is down");
    /// }
    /// ```
    ///
    #[inline]
    pub fn is_key_pressed(&self, key: Key, repeat: KeyRepeat) -> bool {
        self.0.is_key_pressed(key, repeat)
    }

    ///
    /// Sets the delay for when a key is being held before it starts being repeated the default
    /// value is 0.25 sec
    ///
    /// # Examples
    ///
    /// ```ignore
    /// window.set_key_repeat_delay(0.5) // 0.5 sec before repeat starts
    /// ```
    ///
    #[inline]
    pub fn set_key_repeat_delay(&mut self, delay: f32) {
        self.0.set_key_repeat_delay(delay)
    }

    ///
    /// Sets the rate in between when the keys has passed the initial repeat_delay. The default
    /// value is 0.05 sec
    ///
    /// # Examples
    ///
    /// ```ignore
    /// window.set_key_repeat_rate(0.01) // 0.01 sec between keys
    /// ```
    ///
    #[inline]
    pub fn set_key_repeat_rate(&mut self, rate: f32) {
        self.0.set_key_repeat_rate(rate)
    }

    ///
    /// Returns if this windows is the current active one
    ///
    #[inline]
    pub fn is_active(&mut self) -> bool {
        self.0.is_active()
    }

    ///
    /// Set input callback to recive callback on char input
    ///
    #[inline]
    pub fn set_input_callback(&mut self, callback: Box<InputCallback>)  {
        self.0.set_input_callback(callback)
    }

    ///
    /// This allows adding menus to your windows. As menus behaves a bit diffrently depending on
    /// Operating system here is how it works. See [Menu] for description on each field.
    ///
    /// ```ignore
    /// Windows:
    ///   Each window has their own menu and shortcuts are active depending on active window.
    /// Mac:
    ///   As Mac uses one menu for the whole program the menu will change depending
    ///   on which window you have active.
    /// Linux/BSD/etc:
    ///   Menus aren't supported as they depend on each WindowManager and is outside of the
    ///   scope for this library to support.
    /// ```
    ///
    
    #[inline]
    pub fn add_menu(&mut self, menu: &Menu) -> Result<()> {
        self.0.add_menu(&menu.0)
    }
    /*

    ///
    /// Updates an existing menu created with [add_menu]
    ///
    #[inline]
    pub fn update_menu(&mut self, menu_name: &str, menu: &Vec<Menu>) -> Result<()> {
        self.0.update_menu(menu_name, menu)
    }

    ///
    /// Remove a menu that has been added with [add_menu]
    ///
    #[inline]
    pub fn remove_menu(&mut self, menu_name: &str) -> Result<()> {
        self.0.remove_menu(menu_name)
    }
    */

    ///
    /// Check if a menu item has been pressed
    ///
    #[inline]
    pub fn is_menu_pressed(&mut self) -> Option<usize> {
        self.0.is_menu_pressed()
    }
}


/// Command key on Mac OS
pub const MENU_KEY_COMMAND: usize = 1;
/// Windows key on Windows
pub const MENU_KEY_WIN: usize = 2;
/// Shift key
pub const MENU_KEY_SHIFT: usize = 4;
/// Control key
pub const MENU_KEY_CTRL: usize = 8;
/// Alt key
pub const MENU_KEY_ALT: usize = 16;

const MENU_ID_SEPARATOR:usize = 0xffffffff;

pub struct Menu(imp::Menu);

#[derive(Debug)]
pub struct MenuItemHandle(pub u64);

impl Menu {
    pub fn new(name: &str) -> Result<Menu> {
        imp::Menu::new(name).map(Menu)
    }

    #[inline]
    pub fn destroy_menu(&mut self) {
        //self.0.destroy_menu()
    }

    #[inline]
    pub fn add_sub_menu(&mut self, _menu: &Menu) {
        //self.0.add_sub_menu(menu)
    }

    #[inline]
    pub fn add_menu_item(&mut self, item: &MenuItem) -> MenuItemHandle {
        self.0.add_menu_item(item)
    }

    #[inline]
    pub fn add_item(&mut self, name: &str, id: usize) -> MenuItem {
        MenuItem {
            id: id,
            label: name.to_owned(),
            menu: Some(self),
            ..MenuItem::default()
        }
    }

    #[inline]
    pub fn remove_item(&mut self, item: &MenuItemHandle) {
        self.0.remove_item(item)
    }
}

pub struct MenuItem<'a> {
    pub id: usize,
    pub label: String,
    pub enabled: bool,
    pub key: Key,
    pub modifier: u32,
    pub menu: Option<&'a mut Menu>,
}

impl<'a> Default for MenuItem<'a> {
    fn default() -> Self {
        MenuItem {
            id: MENU_ID_SEPARATOR,
            label: "".to_owned(),
            enabled: true,
            key: Key::Unknown,
            modifier: 0,
            menu: None,
        }
    }
}

impl<'a> Clone for MenuItem<'a> {
    fn clone(&self) -> Self {
        MenuItem {
            id: self.id,
            label: self.label.clone(), 
            enabled: self.enabled,
            key: self.key,
            modifier: self.modifier, 
            menu: None,
        }
    }
}

impl<'a> MenuItem<'a> {
    pub fn new(name: &str, id: usize) -> MenuItem {
        MenuItem {
            id: id,
            label: name.to_owned(),
            ..MenuItem::default()
        }
    }
    #[inline]
    pub fn shortcut(self, key: Key, modifier: u32) -> Self {
        MenuItem {
            key: key,
            modifier: modifier,
            .. self
        }
    }
    #[inline]
    pub fn separator(self) -> Self {
        MenuItem {
            id: MENU_ID_SEPARATOR,
            .. self
        }
    }
    #[inline]
    pub fn enabled(self, enabled: bool) -> Self {
        MenuItem {
            enabled: enabled,
            .. self
        }
    }
    #[inline]
    pub fn build(&mut self) -> MenuItemHandle {
        let t = self.clone();
        if let Some(ref mut menu) = self.menu {
            menu.0.add_menu_item(&t)
        } else {
            MenuItemHandle(0)
        }
    }
}

// Impl for WindowOptions

#[doc(hidden)]
impl Default for WindowOptions {
    fn default() -> WindowOptions {
        WindowOptions {
            borderless: false,
            title: true,
            resize: false,
            scale: Scale::X1,
        }
    }
}


