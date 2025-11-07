//! minifb is a cross platform library written in [Rust](https://www.rust-lang.org) that makes it
//! easy to open windows (usually native to the running operating system) and can optionally show
//! a 32-bit buffer. minifb also support keyboard, mouse input and menus on selected operating
//! systems.

#![deny(missing_debug_implementations)]

#[cfg(not(any(
    target_os = "macos",
    target_os = "redox",
    windows,
    target_arch = "wasm32"
)))]
#[cfg(feature = "wayland")]
#[macro_use]
extern crate dlib;

mod error;
mod icon;
mod key;
mod key_handler;
mod os;
mod rate;

use raw_window_handle::{DisplayHandle, HandleError, HasDisplayHandle, WindowHandle};
use std::{ffi::c_void, fmt, time::Duration};

#[cfg(target_arch = "wasm32")]
use std::panic;

#[cfg(target_os = "macos")]
use os::macos as imp;
#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "netbsd",
    target_os = "openbsd"
))]
use os::posix as imp;
#[cfg(target_os = "redox")]
use os::redox as imp;
#[cfg(target_arch = "wasm32")]
use os::wasm as imp;
#[cfg(target_os = "windows")]
use os::windows as imp;

pub use error::Error;
pub use icon::Icon;
pub use key::Key;
pub use raw_window_handle::HasWindowHandle;

pub type Result<T> = std::result::Result<T, Error>;

/// Scale will scale the frame buffer and the window that is being sent in when calling the update
/// function. This is useful if you for example want to display a 320 x 256 window on a screen with
/// much higher resolution which would result in that the window is very small.
#[derive(Clone, Copy, Debug)]
pub enum Scale {
    /// This mode checks your current screen resolution and will calculate the largest window size
    /// that can be used within that limit and resize it. Useful if you have a small buffer to
    /// display on a high resolution screen.
    FitScreen,
    /// 1X scale (which means leave the coordinates sent into Window::new untouched)
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
#[derive(PartialEq, Clone, Copy, Debug)]
pub enum KeyRepeat {
    /// Use repeat
    Yes,
    /// Don't use repeat
    No,
}

/// The various mouse buttons that are available
#[derive(PartialEq, Clone, Copy, Debug)]
pub enum MouseButton {
    /// Left mouse button
    Left,
    /// Middle mouse button
    Middle,
    /// Right mouse button
    Right,
    /// Back mouse button
    Back,
    /// Forward mouse button
    Forward,
}

/// The different modes that can be used to decide how mouse coordinates should be handled
#[derive(PartialEq, Clone, Copy, Debug)]
pub enum MouseMode {
    /// Return mouse coords from outside of the window (may be negative)
    Pass,
    /// Clamp the mouse coordinates within the window
    Clamp,
    /// Discard if the mouse is outside the window
    Discard,
}

impl MouseMode {
    pub(crate) fn get_pos(
        self,
        mut x: f32,
        mut y: f32,
        scale: f32,
        mut width: f32,
        mut height: f32,
    ) -> Option<(f32, f32)> {
        x /= scale;
        y /= scale;
        width /= scale;
        height /= scale;

        match self {
            Self::Pass => Some((x, y)),
            Self::Clamp => Some((x.clamp(0.0, width - 1.0), y.clamp(0.0, height - 1.0))),
            Self::Discard => {
                if x < 0.0 || y < 0.0 || x >= width || y >= height {
                    None
                } else {
                    Some((x, y))
                }
            }
        }
    }
}

/// Different style of cursors that can be used
#[derive(PartialEq, Clone, Copy, Debug)]
pub enum CursorStyle {
    /// Regular arrow style (this is what the cursor normal looks like)
    Arrow,
    /// Used when indicating insertion (like text field)
    Ibeam,
    /// Cross-hair cursor
    Crosshair,
    /// Closed hand which useful for dragging things, may use default hand on unsupported OSes.
    ClosedHand,
    /// Open hand which useful for indicating draggable things, may use default hand on unsupported OSes.
    OpenHand,
    /// Resizing left-right direction
    ResizeLeftRight,
    /// Resizing up-down direction
    ResizeUpDown,
    /// Resize in all directions
    ResizeAll,
}

/// This trait can be implemented and set with ```set_input_callback``` to receive a callback
/// when there is inputs.
pub trait InputCallback {
    /// Called when text is added to the window, or a key is pressed. This passes
    /// in a unicode character, and therefore does not report control characters.
    fn add_char(&mut self, uni_char: u32);

    /// Called whenever a key is pressed or released. This reports the state of the
    /// key in the `state` argument, as well as the translated key in the `key` argument.
    /// This includes control characters such as `Key::LeftShift`.
    fn set_key_state(&mut self, _key: Key, _state: bool) {}
}

/// Window is used to open up a window. It's possible to optionally display a 32-bit buffer when
/// the widow is set as non-resizable.
pub struct Window(imp::Window);

impl fmt::Debug for Window {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("Window").field(&format_args!("..")).finish()
    }
}

impl HasWindowHandle for Window {
    fn window_handle(&self) -> std::result::Result<WindowHandle<'_>, HandleError> {
        self.0.window_handle()
    }
}

impl HasDisplayHandle for Window {
    fn display_handle(&self) -> std::result::Result<DisplayHandle<'_>, HandleError> {
        self.0.display_handle()
    }
}

/// On some OS (X11 for example) it's possible a window can resize even if no resize has been set.
/// This causes some issues depending on how the content of an input buffer should be displayed then it's possible
/// to set this scaling mode to get a better behavior.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScaleMode {
    /// Stretch the buffer in the whole window meaning if your buffer is 256x256 and window is 1024x1024 it will be scaled up 4 times
    Stretch,
    /// Keep the correct aspect ratio to be displayed while scaling up fully in the other axis. Fill area will be filed with Window::set_bg_color (default 0, 0, 0)
    AspectRatioStretch,
    /// Places the buffer in the middle of the window without any scaling. Fills the borders with color set `Window::set_background_color` (default 0, 0, 0)
    /// If the window is smaller than the buffer the center of the buffer will be displayed
    Center,
    /// Same as Center but places the buffer in the upper left corner of the window.
    UpperLeft,
}

/// WindowOptions is creation settings for the window. By default the settings are defined for
/// displaying a 32-bit buffer (no scaling of window is possible)
#[derive(Clone, Copy, Debug)]
pub struct WindowOptions {
    /// If the window should be borderless (default: false)
    pub borderless: bool,
    /// If the window should have a title (default: true)
    pub title: bool,
    /// If it should be possible to resize the window (default: false)
    pub resize: bool,
    /// Scale of the window that used in conjunction with update_with_buffer (default: X1)
    pub scale: Scale,
    /// Adjust how the scaling of the buffer used with update_with_buffer should be done.
    pub scale_mode: ScaleMode,
    /// Should the window be the topmost window (default: false)
    pub topmost: bool,
    /// Specifies whether or not the window is allowed to draw transparent pixels (default: false)
    /// Requires borderless to be 'true'
    /// TODO: Currently not implemented on OSX.
    /// TODO: Make it work without none option on windows.
    pub transparency: bool,
    /// Required for transparency on windows.
    /// Should be mutually exclusive to resize, automatically assumes borderless.
    /// Not supported on OSX.
    pub none: bool,
}

#[allow(dead_code)]
impl WindowOptions {
    const WINDOW_BORDERLESS: u32 = 1 << 1;
    const WINDOW_RESIZE: u32 = 1 << 2;
    const WINDOW_TITLE: u32 = 1 << 3;

    // Construct a bitmask of flags (sent to backends) from WindowOpts
    #[inline]
    pub(crate) fn get_flags(self) -> u32 {
        let mut flags = 0u32;

        if self.borderless {
            flags |= Self::WINDOW_BORDERLESS;
        }

        if self.title {
            flags |= Self::WINDOW_TITLE;
        }

        if self.resize {
            flags |= Self::WINDOW_RESIZE;
        }

        flags
    }
}

impl Window {
    #[cfg(not(target_arch = "wasm32"))]
    /// Opens up a new window
    ///
    /// # Examples
    ///
    /// Open up a window with default settings
    ///
    /// ```no_run
    /// # use minifb::*;
    /// let mut window = match Window::new("Test", 640, 400, WindowOptions::default()) {
    ///    Ok(win) => win,
    ///    Err(err) => {
    ///        println!("Unable to create window {}", err);
    ///        return;
    ///    }
    ///};
    /// ```
    ///
    /// Open up a window that is resizable
    ///
    /// ```no_run
    /// # use minifb::*;
    /// let mut window = Window::new("Test", 640, 400,
    ///     WindowOptions {
    ///        resize: true,
    ///        ..WindowOptions::default()
    ///  })
    ///  .expect("Unable to open Window");
    /// ```
    pub fn new(name: &str, width: usize, height: usize, opts: WindowOptions) -> Result<Window> {
        if opts.transparency && !opts.borderless {
            return Err(Error::WindowCreate(
                "Window transparency requires the borderless property".to_owned(),
            ));
        }
        imp::Window::new(name, width, height, opts).map(Window)
    }

    #[cfg(target_arch = "wasm32")]
    /// Opens up a new window
    ///
    /// # Examples
    ///
    /// Open up a window with default settings
    ///
    /// ```no_run
    /// # use minifb::*;
    /// let document = web_sys::window().unwrap().document().unwrap();
    /// let body = document.body().unwrap();
    /// let mut window = match Window::new(body, 640, 400, WindowOptions::default()) {
    ///    Ok(win) => win,
    ///    Err(err) => {
    ///        println!("Unable to create window {}", err);
    ///        return;
    ///    }
    ///};
    /// ```
    ///
    /// Open up a window that is resizable
    /// TODO: Enable web canvas resizing
    ///
    /// ```no_run
    /// # use minifb::*;
    /// let document = web_sys::window().unwrap().document().unwrap();
    /// let body = document.body().unwrap();
    /// let mut window = Window::new(body, 640, 400,
    ///     WindowOptions {
    ///        resize: true,
    ///        ..WindowOptions::default()
    ///  })
    ///  .expect("Unable to open Window");
    /// ```
    pub fn new(
        container: &str,
        width: usize,
        height: usize,
        opts: WindowOptions,
    ) -> Result<Window> {
        panic::set_hook(Box::new(console_error_panic_hook::hook));

        if opts.transparency && !opts.borderless {
            return Err(Error::WindowCreate(
                "Window transparency requires the borderless property".to_owned(),
            ));
        }
        imp::Window::new(container, width, height, opts).map(Window)
    }

    /// Allows you to set a new title of the window after creation
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use minifb::*;
    /// let mut window = Window::new("Test", 640, 400, WindowOptions::default()).unwrap();
    ///
    /// window.set_title("My New Title!");
    /// ```
    #[inline]
    pub fn set_title(&mut self, title: &str) {
        self.0.set_title(title)
    }

    /// Sets the icon of the window after creation.
    ///
    /// The file path has to be relative to the current working directory.
    ///
    /// **Windows:** Has to be a `.ico` file. To also set the icon of the `.exe` file, see the `rc.exe` tool
    ///
    /// **Linux:**
    /// - X11: Needs a `u64` buffer with ARGB data
    /// - Wayland: *not implemented* (use a `.desktop` file)
    ///
    /// **MacOS:**
    ///
    /// **RedoxOS:** *not implemented*
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use minifb::*;
    /// # use std::str::FromStr;
    /// let mut window = Window::new("Test", 640, 400, WindowOptions::default()).unwrap();
    ///
    /// #[cfg(target_os = "windows")]
    /// window.set_icon(Icon::from_str("src/icon.ico").unwrap());
    /// ```
    #[inline]
    pub fn set_icon(&mut self, icon: Icon) {
        self.0.set_icon(icon)
    }

    /// Returns the native handle for a window which is an opaque pointer/handle which
    /// depends on the current operating system:
    ///
    /// ```text
    /// Windows HWND
    /// MacOS   NSWindow
    /// X11     XWindow
    /// ```
    #[inline]
    pub fn get_window_handle(&self) -> *mut c_void {
        self.0.get_window_handle()
    }

    /// Updates the window with a 32-bit pixel buffer. The encoding for each pixel is `0RGB`:
    /// The upper 8-bits are ignored, the next 8-bits are for the red channel, the next 8-bits
    /// afterwards for the green channel, and the lower 8-bits for the blue channel.
    ///
    /// Notice that the buffer needs to be at least the size of the created window. Also only one of
    /// `update_with_buffer` or `update` should be called for updating a single window.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use minifb::*;
    /// fn from_u8_rgb(r: u8, g: u8, b: u8) -> u32 {
    ///     let (r, g, b) = (r as u32, g as u32, b as u32);
    ///     (r << 16) | (g << 8) | b
    /// }
    /// let window_width = 600;
    /// let window_height = 400;
    /// let buffer_width = 600;
    /// let buffer_height = 400;
    ///
    /// let azure_blue = from_u8_rgb(0, 127, 255);
    ///
    /// let mut buffer: Vec<u32> = vec![azure_blue; buffer_width * buffer_height];
    ///
    /// let mut window = Window::new("Test", window_width, window_height, WindowOptions::default()).unwrap();
    ///
    /// window.update_with_buffer(&buffer, buffer_width, buffer_height).unwrap();
    /// ```
    #[inline]
    pub fn update_with_buffer(
        &mut self,
        buffer: &[u32],
        width: usize,
        height: usize,
    ) -> Result<()> {
        let update_result = self
            .0
            .update_with_buffer_stride(buffer, width, height, width);
        self.0.update_rate();
        update_result
    }

    /// Updates the window (this is required to call in order to get keyboard/mouse input, etc)
    ///
    /// Notice that when using this function then `update_with_buffer` should not be called for the same window.
    /// Only one of the functions should be used.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use minifb::*;
    /// let mut buffer: Vec<u32> = vec![0; 640 * 400];
    ///
    /// let mut window = Window::new("Test", 640, 400, WindowOptions::default()).unwrap();
    ///
    /// window.update();
    /// ```
    #[inline]
    pub fn update(&mut self) {
        self.0.update();
        self.0.update_rate();
    }

    /// Checks if the window is still open. A window can be closed by the user (by for example
    /// pressing the close button on the window) It's up to the user to make sure that this is
    /// being checked and take action depending on the state.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use minifb::*;
    /// # let mut window = Window::new("Test", 640, 400, WindowOptions::default()).unwrap();
    /// while window.is_open() {
    ///     // Update window
    /// }
    /// ```
    #[inline]
    pub fn is_open(&self) -> bool {
        self.0.is_open()
    }

    /// Sets the position of the window. This is useful if you have
    /// more than one window and want to align them up on the screen
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use minifb::*;
    /// # let mut window = Window::new("Test", 640, 400, WindowOptions::default()).unwrap();
    /// // Moves the window to pixel position 20, 20 on the screen
    /// window.set_position(20, 20);
    /// ```
    #[inline]
    pub fn set_position(&mut self, x: isize, y: isize) {
        self.0.set_position(x, y)
    }

    /// Gets the position of the window. This is useful if you want
    /// to store the position of the window across sessions
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use minifb::*;
    /// # let mut window = Window::new("Test", 640, 400, WindowOptions::default()).unwrap();
    /// // Retrieves the current window position
    /// let (x,y) = window.get_position();
    /// ```
    #[inline]
    pub fn get_position(&self) -> (isize, isize) {
        self.0.get_position()
    }

    /// Makes the window the topmost window and makes it stay always on top. This is useful if you
    /// want the window to float above all over windows
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use minifb::*;
    /// # let mut window = Window::new("Test", 640, 400, WindowOptions::default()).unwrap();
    /// // Makes the window always on top
    /// window.topmost(true);
    /// ```
    #[inline]
    pub fn topmost(&self, topmost: bool) {
        self.0.topmost(topmost)
    }

    /// Sets the background color that is used with update_with_buffer.
    /// In some cases there will be a blank area around the buffer depending on the ScaleMode that has been set.
    /// This color will be used in the in that area.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use minifb::*;
    /// # let mut window = Window::new("Test", 640, 400, WindowOptions::default()).unwrap();
    /// // Set background color to bright red
    /// window.set_background_color(255, 0, 0);
    /// ```
    #[inline]
    pub fn set_background_color(&mut self, red: u8, green: u8, blue: u8) {
        self.0
            .set_background_color((red as u32) << 16 | (green as u32) << 8 | blue as u32);
    }

    /// Changes whether or not the cursor image should be shown or if the cursor image
    /// should be invisible inside the window
    /// When creating a new window the default is 'false'
    #[inline]
    pub fn set_cursor_visibility(&mut self, visibility: bool) {
        self.0.set_cursor_visibility(visibility);
    }

    /// Limits the update rate of polling for new events in order to reduce CPU usage.
    /// The problem of having a tight loop that does something like this
    ///
    /// ```no_run
    /// # use minifb::*;
    /// # let mut window = Window::new("Test", 640, 400, WindowOptions::default()).unwrap();
    /// loop {
    ///    window.update();
    /// }
    /// ```
    /// Is that lots of CPU time will be spent calling system functions to check for new events in a tight loop making the CPU time go up.
    /// Using `limit_update_rate` minifb will check how much time has passed since the last time and if it's less than the selected time it will sleep for the remainder of it.
    /// This means that if more time has spent than the set time (external code taking longer) minifb will not do any waiting at all so there is no loss in CPU performance with this feature.
    /// By default it's set to 4 milliseconds. Setting this value to None and no waiting will be done
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use minifb::*;
    /// # let mut window = Window::new("Test", 640, 400, WindowOptions::default()).unwrap();
    /// // Make sure that at least 4 ms has passed since the last event poll
    /// window.limit_update_rate(Some(std::time::Duration::from_millis(4)));
    /// ```
    #[inline]
    #[deprecated(
        since = "0.26.0",
        note = "use `set_target_fps` instead, this function will be removed in the future"
    )]
    pub fn limit_update_rate(&mut self, time: Option<Duration>) {
        self.0.set_rate(time)
    }

    /// Limits the FPS of polling for new events in order to reduce CPU usage.
    /// The problem of having a tight loop that does something like this
    ///
    /// ```no_run
    /// # use minifb::*;
    /// # let mut window = Window::new("Test", 640, 400, WindowOptions::default()).unwrap();
    /// loop {
    ///    window.update();
    /// }
    /// ```
    /// Is that lots of CPU time will be spent calling system functions to check for new events in a tight loop making the CPU time go up.
    /// Using `set_target_fps` minifb will check how many frames are left to reach the target FPS and if there are any it will sleep for that amount of frames.
    /// This means that if more frames than the target happened (external code taking longer) minifb will not do any waiting at all so there is no loss in CPU performance with this feature.
    /// By default it's set to 250 FPS. Setting this value to 0 and no waiting will be done
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use minifb::*;
    /// # let mut window = Window::new("Test", 640, 400, WindowOptions::default()).unwrap();
    /// // Set the target rate to 60 fps, meaning events will be polled every ~16.6 ms
    /// window.set_target_fps(60);
    /// ```
    #[inline]
    pub fn set_target_fps(&mut self, fps: usize) {
        match fps {
            0 => self.0.set_rate(None),
            non_zero => {
                let fps = Duration::from_secs_f32(1. / non_zero as f32);
                self.0.set_rate(Some(fps));
            }
        }
    }

    /// Returns the current fps of the window
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use minifb::*;
    /// # let mut window = Window::new("Test", 640, 400, WindowOptions::default()).unwrap();
    /// let fps = window.get_fps();
    /// println!("fps {}", fps);
    /// ```
    #[inline]
    pub fn get_fps(&mut self) -> usize {
        match self.0.get_delta_time() {
            Some(rate) => (1.0 / rate.as_secs_f32()) as usize,
            _ => 0,
        }
    }

    /// Returns the current size of the window
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use minifb::*;
    /// # let mut window = Window::new("Test", 640, 400, WindowOptions::default()).unwrap();
    /// let size = window.get_size();
    /// println!("width {} height {}", size.0, size.1);
    /// ```
    #[inline]
    pub fn get_size(&self) -> (usize, usize) {
        self.0.get_size()
    }

    /// Get the current position of the mouse relative to the current window
    /// The coordinate system is as 0, 0 as the upper left corner
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use minifb::*;
    /// # let mut window = Window::new("Test", 640, 400, WindowOptions::default()).unwrap();
    /// window.get_mouse_pos(MouseMode::Clamp).map(|mouse| {
    ///     println!("x {} y {}", mouse.0, mouse.1);
    /// });
    /// ```
    #[inline]
    pub fn get_mouse_pos(&self, mode: MouseMode) -> Option<(f32, f32)> {
        self.0.get_mouse_pos(mode)
    }

    /// Get the current position of the mouse relative to the current window
    /// The coordinate system is as 0, 0 as the upper left corner and ignores
    /// any scaling set to the window.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use minifb::*;
    /// # let mut window = Window::new("Test", 640, 400, WindowOptions::default()).unwrap();
    /// window.get_unscaled_mouse_pos(MouseMode::Clamp).map(|mouse| {
    ///     println!("x {} y {}", mouse.0, mouse.1);
    /// });
    /// ```
    #[inline]
    pub fn get_unscaled_mouse_pos(&self, mode: MouseMode) -> Option<(f32, f32)> {
        self.0.get_unscaled_mouse_pos(mode)
    }

    /// Check if a mouse button is down or not
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use minifb::*;
    /// # let mut window = Window::new("Test", 640, 400, WindowOptions::default()).unwrap();
    /// let left_down = window.get_mouse_down(MouseButton::Left);
    /// println!("is left down? {}", left_down)
    /// ```
    #[inline]
    pub fn get_mouse_down(&self, button: MouseButton) -> bool {
        self.0.get_mouse_down(button)
    }

    /// Get the current movement of the scroll wheel.
    /// Scroll wheel can mean different thing depending on the device attach.
    /// For example on Mac with trackpad "scroll wheel" means two finger
    /// swiping up/down (y axis) and to the sides (x-axis)
    /// When using a mouse this assumes the scroll wheel which often is only y direction.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use minifb::*;
    /// # let mut window = Window::new("Test", 640, 400, WindowOptions::default()).unwrap();
    /// window.get_scroll_wheel().map(|scroll| {
    ///     println!("scrolling - x {} y {}", scroll.0, scroll.1);
    /// });
    /// ```
    #[inline]
    pub fn get_scroll_wheel(&self) -> Option<(f32, f32)> {
        self.0.get_scroll_wheel()
    }

    /// Set a different cursor style. This can be used if you have resizing
    /// elements or something like that
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use minifb::*;
    /// # let mut window = Window::new("Test", 640, 400, WindowOptions::default()).unwrap();
    /// window.set_cursor_style(CursorStyle::ResizeLeftRight);
    /// ```
    #[inline]
    pub fn set_cursor_style(&mut self, cursor: CursorStyle) {
        self.0.set_cursor_style(cursor)
    }

    /// Get the current keys that are down.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use minifb::*;
    /// # let mut window = Window::new("Test", 640, 400, WindowOptions::default()).unwrap();
    /// window.get_keys().iter().for_each(|key|
    ///         match key {
    ///             Key::W => println!("holding w"),
    ///             Key::T => println!("holding t"),
    ///             _ => (),
    ///         }
    ///     );
    /// ```
    #[inline]
    pub fn get_keys(&self) -> Vec<Key> {
        self.0.get_keys()
    }

    /// Get the current pressed keys. Repeat can be used to control if keys should
    /// be repeated if down or not.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use minifb::*;
    /// # let mut window = Window::new("Test", 640, 400, WindowOptions::default()).unwrap();
    /// window.get_keys_pressed(KeyRepeat::No).iter().for_each(|key|
    ///         match key {
    ///             Key::W => println!("pressed w"),
    ///             Key::T => println!("pressed t"),
    ///             _ => (),
    ///         }
    ///     );
    /// ```
    #[inline]
    pub fn get_keys_pressed(&self, repeat: KeyRepeat) -> Vec<Key> {
        self.0.get_keys_pressed(repeat)
    }

    /// Get the current released keys.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use minifb::*;
    /// # let mut window = Window::new("Test", 640, 400, WindowOptions::default()).unwrap();
    /// window.get_keys_released().iter().for_each(|key|
    ///         match key {
    ///             Key::W => println!("released w"),
    ///             Key::T => println!("released t"),
    ///             _ => (),
    ///         }
    ///     );
    /// ```
    #[inline]
    pub fn get_keys_released(&self) -> Vec<Key> {
        self.0.get_keys_released()
    }

    /// Check if a single key is down.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use minifb::*;
    /// # let mut window = Window::new("Test", 640, 400, WindowOptions::default()).unwrap();
    /// if window.is_key_down(Key::A) {
    ///     println!("Key A is down");
    /// }
    /// ```
    #[inline]
    pub fn is_key_down(&self, key: Key) -> bool {
        self.0.is_key_down(key)
    }

    /// Check if a single key is pressed. KeyRepeat will control if the key should be repeated or
    /// not while being pressed.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use minifb::*;
    /// # let mut window = Window::new("Test", 640, 400, WindowOptions::default()).unwrap();
    /// if window.is_key_pressed(Key::A, KeyRepeat::No) {
    ///     println!("Key A is down");
    /// }
    /// ```
    #[inline]
    pub fn is_key_pressed(&self, key: Key, repeat: KeyRepeat) -> bool {
        self.0.is_key_pressed(key, repeat)
    }

    /// Check if a single key was released since last call to update.
    #[inline]
    pub fn is_key_released(&self, key: Key) -> bool {
        self.0.is_key_released(key)
    }

    /// Sets the delay for when a key is being held before it starts being repeated the default
    /// value is 0.25 sec
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use minifb::*;
    /// # let mut window = Window::new("Test", 640, 400, WindowOptions::default()).unwrap();
    /// window.set_key_repeat_delay(0.5) // 0.5 sec before repeat starts
    /// ```
    #[inline]
    pub fn set_key_repeat_delay(&mut self, delay: f32) {
        self.0.set_key_repeat_delay(delay)
    }

    /// Sets the rate in between when the keys has passed the initial repeat_delay. The default
    /// value is 0.05 sec
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use minifb::*;
    /// # let mut window = Window::new("Test", 640, 400, WindowOptions::default()).unwrap();
    /// window.set_key_repeat_rate(0.01) // 0.01 sec between keys
    /// ```
    #[inline]
    pub fn set_key_repeat_rate(&mut self, rate: f32) {
        self.0.set_key_repeat_rate(rate)
    }

    /// Returns if this windows is the current active one
    #[inline]
    pub fn is_active(&self) -> bool {
        self.0.is_active()
    }

    /// Set input callback to receive callback on char input
    #[inline]
    pub fn set_input_callback(&mut self, callback: Box<dyn InputCallback>) {
        self.0.set_input_callback(callback)
    }

    /// This allows adding menus to your windows. As menus behaves a bit differently depending on
    /// Operating system here is how it works.
    ///
    /// ```text
    /// Windows:
    ///   Each window has their own menu and shortcuts are active depending on active window.
    /// Mac:
    ///   As Mac uses one menu for the whole program the menu will change depending
    ///   on which window you have active.
    /// Linux/BSD/etc:
    ///   Menus aren't supported as they depend on each WindowManager and is outside of the
    ///   scope for this library to support. Use [get_posix_menus] to get a structure
    /// ```
    #[inline]
    pub fn add_menu(&mut self, menu: &Menu) -> MenuHandle {
        self.0.add_menu(&menu.0)
    }

    /// Remove a menu that has been added with [#add_menu]
    #[inline]
    pub fn remove_menu(&mut self, handle: MenuHandle) {
        self.0.remove_menu(handle)
    }

    /// Get POSIX menus. Will only return menus on POSIX-like OSes like Linux or BSD
    /// otherwise ```None```
    #[cfg(any(target_os = "macos", target_os = "windows", target_arch = "wasm32"))]
    #[inline]
    pub fn get_posix_menus(&self) -> Option<&Vec<UnixMenu>> {
        None
    }

    #[cfg(any(
        target_os = "linux",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "redox"
    ))]
    #[inline]
    pub fn get_posix_menus(&self) -> Option<&Vec<UnixMenu>> {
        self.0.get_posix_menus()
    }

    #[deprecated(
        since = "0.17.0",
        note = "`get_unix_menus` will be removed in 1.0.0, use `get_posix_menus` instead"
    )]
    #[inline]
    pub fn get_unix_menus(&self) -> Option<&Vec<UnixMenu>> {
        self.get_posix_menus()
    }

    /// Check if a menu item has been pressed
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

const MENU_ID_SEPARATOR: usize = 0xffffffff;

/// Used on POSIX systems (Linux, FreeBSD, etc) as menus aren't supported in a native way there.
/// This structure can be used by calling [#get_posix_menus] on Window.
///
/// In version 1.0.0, this struct will be renamed to PosixMenu, but it remains UnixMenu for backwards compatibility
/// reasons.
#[derive(Debug, Clone)]
pub struct UnixMenu {
    /// Name of the menu
    pub name: String,
    /// All items of the menu.
    pub items: Vec<UnixMenuItem>,
    #[doc(hidden)]
    pub handle: MenuHandle,
    #[doc(hidden)]
    pub item_counter: MenuItemHandle,
}

/// Used on POSIX systems (Linux, FreeBSD, etc) as menus aren't supported in a native way there.
/// This structure holds info for each item in a #UnixMenu
#[derive(Debug, Clone)]
pub struct UnixMenuItem {
    /// Set to a menu if there is a Item is a sub_menu otherwise None
    pub sub_menu: Option<Box<UnixMenu>>,
    /// Handle of the MenuItem
    pub handle: MenuItemHandle,
    /// Id of the item (set by the user from the outside and should be reported back when pressed)
    pub id: usize,
    /// Name of the item
    pub label: String,
    /// Set to true if enabled otherwise false
    pub enabled: bool,
    /// Shortcut key
    pub key: Key,
    /// Modifier for the key (Shift, Ctrl, etc)
    pub modifier: usize,
}

#[derive(Debug, Copy, Clone)]
#[doc(hidden)]
pub struct MenuItemHandle(pub u64);

#[derive(Debug, Copy, Clone, PartialEq)]
#[doc(hidden)]
pub struct MenuHandle(pub u64);

/// Menu holds info for menus
pub struct Menu(imp::Menu);

impl fmt::Debug for Menu {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("Menu").field(&format_args!("..")).finish()
    }
}

impl Menu {
    /// Create a new menu. Returns error if failed
    pub fn new(name: &str) -> Result<Menu> {
        imp::Menu::new(name).map(Menu)
    }

    /// Destroys a menu. Currently not implemented
    #[inline]
    pub fn destroy_menu(&mut self) {
        //self.0.destroy_menu()
    }

    /// Adds a sub menu to the current menu
    #[inline]
    pub fn add_sub_menu(&mut self, name: &str, menu: &Menu) {
        self.0.add_sub_menu(name, &menu.0)
    }

    /// Adds a menu separator
    #[inline]
    pub fn add_separator(&mut self) {
        self.add_menu_item(&MenuItem {
            id: MENU_ID_SEPARATOR,
            ..MenuItem::default()
        });
    }

    /// Adds an item to the menu
    #[inline]
    pub fn add_menu_item(&mut self, item: &MenuItem) -> MenuItemHandle {
        self.0.add_menu_item(item)
    }

    /// Adds an item to the menu. Notice that you need to call "build" to finish the add
    /// # Examples
    ///
    /// ```no_run
    /// # use minifb::*;
    /// # let mut menu = Menu::new("test").unwrap();
    /// menu.add_item("test", 1).shortcut(Key::A, 0).build()
    /// # ;
    /// ```
    #[inline]
    pub fn add_item(&mut self, name: &str, id: usize) -> MenuItem<'_> {
        MenuItem {
            id,
            label: name.to_owned(),
            menu: Some(self),
            ..MenuItem::default()
        }
    }

    /// Removes an item from the menu
    #[inline]
    pub fn remove_item(&mut self, item: &MenuItemHandle) {
        self.0.remove_item(item)
    }
}

/// Holds info about each item in a menu
#[derive(Debug)]
pub struct MenuItem<'a> {
    pub id: usize,
    pub label: String,
    pub enabled: bool,
    pub key: Key,
    pub modifier: usize,
    #[doc(hidden)]
    pub menu: Option<&'a mut Menu>,
}

impl Default for MenuItem<'_> {
    fn default() -> Self {
        MenuItem {
            id: MENU_ID_SEPARATOR,
            label: String::default(),
            enabled: true,
            key: Key::Unknown,
            modifier: 0,
            menu: None,
        }
    }
}

impl Clone for MenuItem<'_> {
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

impl MenuItem<'_> {
    /// Creates a new menu item
    pub fn new(name: &str, id: usize) -> MenuItem<'_> {
        MenuItem {
            id,
            label: name.to_owned(),
            ..MenuItem::default()
        }
    }

    /// Sets a shortcut key and modifier (and returns itself)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use minifb::*;
    /// # let mut menu = Menu::new("test").unwrap();
    /// menu.add_item("test", 1).shortcut(Key::A, 0).build()
    /// # ;
    /// ```
    #[inline]
    pub fn shortcut(self, key: Key, modifier: usize) -> Self {
        MenuItem {
            key,
            modifier,
            ..self
        }
    }

    /// Sets item to a separator
    /// Notice that it's usually easier to just call ```menu.add_separator()``` directly
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use minifb::*;
    /// # let mut menu = Menu::new("test").unwrap();
    /// menu.add_item("", 0).separator().build()
    /// # ;
    /// ```
    #[inline]
    pub fn separator(self) -> Self {
        MenuItem {
            id: MENU_ID_SEPARATOR,
            ..self
        }
    }

    /// Sets the menu item disabled/or not
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use minifb::*;
    /// # let mut menu = Menu::new("test").unwrap();
    /// menu.add_item("test", 1).enabled(false).build()
    /// # ;
    /// ```
    #[inline]
    pub fn enabled(self, enabled: bool) -> Self {
        MenuItem { enabled, ..self }
    }

    /// Must be called to finalize building of a menu item when started with ```menu.add_item()```
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use minifb::*;
    /// # let mut menu = Menu::new("test").unwrap();
    /// menu.add_item("test", 1).enabled(false).build()
    /// # ;
    /// ```
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

impl Default for WindowOptions {
    fn default() -> WindowOptions {
        WindowOptions {
            borderless: false,
            transparency: false,
            title: true,
            resize: false,
            scale: Scale::X1,
            scale_mode: ScaleMode::Stretch,
            topmost: false,
            none: false,
        }
    }
}

pub(crate) fn check_buffer_size(
    buffer: &[u32],
    mut buf_width: usize,
    buf_height: usize,
    buf_stride: usize,
) -> Result<()> {
    buf_width = buf_width.max(buf_stride);
    let buf_size = std::mem::size_of_val(buffer);
    let required_buf_size = buf_width
        .checked_mul(buf_height)
        .and_then(|v| v.checked_mul(std::mem::size_of::<u32>()))
        .ok_or(Error::UpdateFailed("Buffer size too large".to_string()))?;

    if buf_size == 0 {
        Err(Error::UpdateFailed(
            "Update failed because input buffer is empty".to_string(),
        ))
    } else if buf_size < required_buf_size {
        let err = format!(
        "Update failed because input buffer is too small. Required size for {} ({} stride) x {} buffer is {} bytes but input buffer is {} bytes",
        buf_width, buf_stride, buf_height, required_buf_size, buf_size
    );
        Err(Error::UpdateFailed(err))
    } else {
        Ok(())
    }
}
