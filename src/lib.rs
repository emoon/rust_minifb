extern crate libc;
#[cfg(target_os = "macos")]
#[macro_use]
extern crate objc;
#[cfg(target_os = "macos")]
extern crate cgl;
#[cfg(target_os = "macos")]
extern crate cocoa;
#[cfg(target_os = "macos")]
extern crate core_foundation;
#[cfg(target_os = "macos")]

/// Scale will scale the frame buffer and the window that is being sent in when calling the update
/// function. This is useful if you for example want to display a 320 x 256 window on a screen with
/// much higher resolution which would result in that the window is very small.
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

/// Vsync will allow syncronized rendering with the screen refresh rate.
/// Currently Vsync isn't implemented so nothing will change regardless of given value right now
pub enum Vsync {
    /// No vsync
    No,
    /// Require accurate vsync. Notice that if the library is unable to to setup an accurate
    /// syncing the window creation will fail.
    Accurate,
    /// Setup a best guess syncing with the screen. This will always succesed but may not be
    /// accurate. What this means is if the lib is unable to create a accurate syncing approach
    /// a 'emulated' one will be used (for example using a timer to approximate syncing)
    BestGuess,
}

/// 
pub enum Key {
    Key1,
    Key2,
    Key3,
    Key4,
    Key5,
    Key6,
    Key7,
    Key8,
    Key9,
    Key0,

    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,

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
    CapsLock,

    Count = 80,
}

#[cfg(target_os = "windows")]
pub mod windows;
#[cfg(target_os = "windows")]
pub use windows::*;

#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "macos")]
pub use macos::*;

