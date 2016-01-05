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

/// Key is used by the get key functions to check if some keys on the keyboard has been pressed
#[derive(PartialEq, Clone, Copy)]
pub enum Key {
    Key0 = 0,
    Key1 = 1,
    Key2 = 2,
    Key3 = 3,
    Key4 = 4,
    Key5 = 5,
    Key6 = 6,
    Key7 = 7,
    Key8 = 8,
    Key9 = 9,

    A = 10,
    B = 11,
    C = 12,
    D = 13,
    E = 14,
    F = 15,
    G = 16,
    H = 17,
    I = 18,
    J = 19,
    K = 20,
    L = 21,
    M = 22,
    N = 23,
    O = 24,
    P = 25,
    Q = 26,
    R = 27,
    S = 28,
    T = 29,
    U = 30,
    V = 31,
    W = 32,
    X = 33,
    Y = 34,
    Z = 35,

    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,

    Down,
    Left,
    Right,
    Up,
    Apostrophe,
    Backquote,

    Backslash,
    Comma,
    Equal,
    LeftBracket,
    Minus,
    Period,
    RightBracket,
    Semicolon,

    Slash,
    Backspace,
    Delete,
    End,
    Enter,

    Escape,

    Home,
    Insert,
    Menu,

    PageDown,
    PageUp,

    Pause,
    Space,
    Tab,
    NumLock,
    CapsLock,
    ScrollLock,
    LeftShift,
    RightShift,
    LeftCtrl,
    RightCtrl,

    NumPad0,
    NumPad1,
    NumPad2,
    NumPad3,
    NumPad4,
    NumPad5,
    NumPad6,
    NumPad7,
    NumPad8,
    NumPad9,
    NumPadDot,
    NumPadSlash,
    NumPadAsterisk,
    NumPadMinus,
    NumPadPlus,
    NumPadEnter,

    LeftAlt,
    RightAlt,

    LeftSuper,
    RightSuper,

    /// Used when an Unknown key has been pressed
    Unknown,

    Count = 107,
}

extern crate libc;

pub mod os;
mod key_handler;

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

///
/// Window used for displaying a 32-bit RGB buffer. Here is a small example on how to use it:
/// (without error checking 
///
/// ```ignore
///
/// const WIDTH: usize = 640;
/// const HEIGHT: usize = 360;
///
/// let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];
///
/// let mut window = match Window::new("Test - Press ESC to exit", WIDTH, HEIGHT, Scale::X1).unwrap()
///
/// while window.is_open() && !window.is_key_down(Key::Escape) {
///     for i in buffer.iter_mut() {
///         *i = 0; // write something interesting here
///     }
///     window.update(&buffer);
/// }
/// ```
///

pub struct Window(imp::Window);

impl Window {
    ///
    /// Opens up a new window
    ///
    /// ```ignore
    /// let mut window = match Window::new("Test", 640, 400, Scale::X1) {
    ///    Ok(win) => win,
    ///    Err(err) => {
    ///        println!("Unable to create window {}", err);
    ///        return;
    ///    }
    ///};
    /// ```
    pub fn new(name: &str, width: usize, height: usize, scale: Scale) -> Result<Window, &str> {
        imp::Window::new(name, width, height, scale).map(Window)
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
    /// let mut window = match Window::new("Test", 640, 400, Scale::X1).unwrap();
    ///
    /// window.update(&buffer);
    /// ```
    pub fn update(&mut self, buffer: &[u32]) {
        self.0.update(buffer)
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
    /// Sets the rate in between when the keys has passed the intital repeat_delay. The default
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
}
