//! Regression test for https://github.com/emoon/rust_minifb/issues/404
//!
//! `Window` is an unboxed public type, but the Windows and macOS backends used
//! to stash a raw `self` pointer for event dispatch. Moving the `Window` (into
//! a `Box`, a reallocating `Vec`, a function return, ...) left that pointer
//! stale and any later message/callback dereferenced freed or invalid memory.
//!
//! The fix heap-allocates the backend state so its address is stable. This test
//! moves a live `Window` and then drives the message/event path that used to
//! crash.

#![cfg(any(windows, target_os = "macos"))]

use minifb::{Window, WindowOptions};

#[test]
fn window_remains_usable_after_move() {
    let mut window = Window::new("move test", 64, 64, WindowOptions::default()).unwrap();

    // Prime the backend so it registers a pointer to its state.
    window.update();

    // Relocate the `Window` value. This is what invalidated the old backend
    // pointer.
    let mut window = Box::new(window);

    // Windows: `SetWindowTextW` synchronously re-enters `wnd_proc` through the
    // stored pointer. macOS: the next `update()` pumps events against the
    // callback target.
    window.set_title("moved");
    window.update();

    assert!(window.is_open());
    let (width, height) = window.get_size();
    assert!(width > 0 && height > 0);
}
