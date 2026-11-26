//! Regression test for https://github.com/emoon/rust_minifb/issues/416
//!
//! `WM_XBUTTONDOWN` and `WM_XBUTTONUP` used to index `wnd.mouse.state` directly
//! with `GET_XBUTTON_WPARAM(wparam)` without validating that the button is
//! `XBUTTON1` or `XBUTTON2`, causing an out-of-bounds panic inside `wnd_proc`
//! (an `extern "system"` callback) and aborting the process.

#![cfg(windows)]

use minifb::{HasWindowHandle, MouseButton, Window, WindowOptions};
use raw_window_handle::RawWindowHandle;
use winapi::shared::windef::HWND;
use winapi::um::winuser::{SendMessageW, WM_XBUTTONDOWN, WM_XBUTTONUP, XBUTTON1, XBUTTON2};

fn get_hwnd(window: &Window) -> HWND {
    match window.window_handle().unwrap().as_raw() {
        RawWindowHandle::Win32(handle) => handle.hwnd.get() as HWND,
        other => panic!("unexpected window handle: {:?}", other),
    }
}

#[test]
fn wm_xbutton_handles_valid_buttons_and_ignores_zero() {
    let mut window = Window::new("xbutton test", 64, 64, WindowOptions::default()).unwrap();
    window.update();

    let hwnd = get_hwnd(&window);

    // Valid XBUTTON1 (Back) press and release.
    unsafe {
        SendMessageW(hwnd, WM_XBUTTONDOWN, (XBUTTON1 as usize) << 16, 0);
    }
    assert!(window.get_mouse_down(MouseButton::Back));
    unsafe {
        SendMessageW(hwnd, WM_XBUTTONUP, (XBUTTON1 as usize) << 16, 0);
    }
    assert!(!window.get_mouse_down(MouseButton::Back));

    // Valid XBUTTON2 (Forward) press and release.
    unsafe {
        SendMessageW(hwnd, WM_XBUTTONDOWN, (XBUTTON2 as usize) << 16, 0);
    }
    assert!(window.get_mouse_down(MouseButton::Forward));
    unsafe {
        SendMessageW(hwnd, WM_XBUTTONUP, (XBUTTON2 as usize) << 16, 0);
    }
    assert!(!window.get_mouse_down(MouseButton::Forward));

    // Button 0 used to alias state[2] (MouseButton::Right).
    unsafe {
        SendMessageW(hwnd, WM_XBUTTONDOWN, 0, 0);
    }
    assert!(!window.get_mouse_down(MouseButton::Right));
}

#[test]
fn wm_xbutton_ignores_out_of_range_values() {
    let mut window = Window::new("xbutton oob test", 64, 64, WindowOptions::default()).unwrap();
    window.update();

    let hwnd = get_hwnd(&window);

    // Values >= 6 used to index past the 8-element `mouse.state` array.
    for invalid in [6usize, 8, 0xffff] {
        unsafe {
            SendMessageW(hwnd, WM_XBUTTONDOWN, invalid << 16, 0);
            SendMessageW(hwnd, WM_XBUTTONUP, invalid << 16, 0);
        }
    }

    assert!(!window.get_mouse_down(MouseButton::Back));
    assert!(!window.get_mouse_down(MouseButton::Forward));
}
