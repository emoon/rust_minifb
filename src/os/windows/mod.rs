#![cfg(target_os = "windows")]

const INVALID_ACCEL: usize = 0xffffffff;

use crate::error::Error;
use crate::key_handler::KeyHandler;
use crate::rate::UpdateRate;
use crate::Result;
use crate::{CursorStyle, MenuHandle, MenuItem, MenuItemHandle};
use crate::{
    InputCallback, Key, KeyRepeat, MouseButton, MouseMode, Scale, ScaleMode, WindowOptions,
};
use crate::{MENU_KEY_ALT, MENU_KEY_CTRL, MENU_KEY_SHIFT, MENU_KEY_WIN};

use crate::buffer_helper;
use crate::mouse_handler;
use std::ffi::OsStr;
use std::mem;
use std::os::raw;
use std::os::windows::ffi::OsStrExt;
use std::ptr;

use winapi::shared::basetsd;
use winapi::shared::minwindef;
use winapi::shared::ntdef;
use winapi::shared::windef;
use winapi::um::errhandlingapi;
use winapi::um::libloaderapi;
use winapi::um::wingdi;
use winapi::um::winuser;

// Wrap this so we can have a proper numbef of bmiColors to write in
#[repr(C)]
struct BitmapInfo {
    pub bmi_header: wingdi::BITMAPINFOHEADER,
    pub bmi_colors: [wingdi::RGBQUAD; 3],
}

fn update_key_state(window: &mut Window, wparam: u32, state: bool) {
    match wparam & 0x1ff {
        0x00B => window.key_handler.set_key_state(Key::Key0, state),
        0x002 => window.key_handler.set_key_state(Key::Key1, state),
        0x003 => window.key_handler.set_key_state(Key::Key2, state),
        0x004 => window.key_handler.set_key_state(Key::Key3, state),
        0x005 => window.key_handler.set_key_state(Key::Key4, state),
        0x006 => window.key_handler.set_key_state(Key::Key5, state),
        0x007 => window.key_handler.set_key_state(Key::Key6, state),
        0x008 => window.key_handler.set_key_state(Key::Key7, state),
        0x009 => window.key_handler.set_key_state(Key::Key8, state),
        0x00A => window.key_handler.set_key_state(Key::Key9, state),
        0x01E => window.key_handler.set_key_state(Key::A, state),
        0x030 => window.key_handler.set_key_state(Key::B, state),
        0x02E => window.key_handler.set_key_state(Key::C, state),
        0x020 => window.key_handler.set_key_state(Key::D, state),
        0x012 => window.key_handler.set_key_state(Key::E, state),
        0x021 => window.key_handler.set_key_state(Key::F, state),
        0x022 => window.key_handler.set_key_state(Key::G, state),
        0x023 => window.key_handler.set_key_state(Key::H, state),
        0x017 => window.key_handler.set_key_state(Key::I, state),
        0x024 => window.key_handler.set_key_state(Key::J, state),
        0x025 => window.key_handler.set_key_state(Key::K, state),
        0x026 => window.key_handler.set_key_state(Key::L, state),
        0x032 => window.key_handler.set_key_state(Key::M, state),
        0x031 => window.key_handler.set_key_state(Key::N, state),
        0x018 => window.key_handler.set_key_state(Key::O, state),
        0x019 => window.key_handler.set_key_state(Key::P, state),
        0x010 => window.key_handler.set_key_state(Key::Q, state),
        0x013 => window.key_handler.set_key_state(Key::R, state),
        0x01F => window.key_handler.set_key_state(Key::S, state),
        0x014 => window.key_handler.set_key_state(Key::T, state),
        0x016 => window.key_handler.set_key_state(Key::U, state),
        0x02F => window.key_handler.set_key_state(Key::V, state),
        0x011 => window.key_handler.set_key_state(Key::W, state),
        0x02D => window.key_handler.set_key_state(Key::X, state),
        0x015 => window.key_handler.set_key_state(Key::Y, state),
        0x02C => window.key_handler.set_key_state(Key::Z, state),
        0x03B => window.key_handler.set_key_state(Key::F1, state),
        0x03C => window.key_handler.set_key_state(Key::F2, state),
        0x03D => window.key_handler.set_key_state(Key::F3, state),
        0x03E => window.key_handler.set_key_state(Key::F4, state),
        0x03F => window.key_handler.set_key_state(Key::F5, state),
        0x040 => window.key_handler.set_key_state(Key::F6, state),
        0x041 => window.key_handler.set_key_state(Key::F7, state),
        0x042 => window.key_handler.set_key_state(Key::F8, state),
        0x043 => window.key_handler.set_key_state(Key::F9, state),
        0x044 => window.key_handler.set_key_state(Key::F10, state),
        0x057 => window.key_handler.set_key_state(Key::F11, state),
        0x058 => window.key_handler.set_key_state(Key::F12, state),
        0x150 => window.key_handler.set_key_state(Key::Down, state),
        0x14B => window.key_handler.set_key_state(Key::Left, state),
        0x14D => window.key_handler.set_key_state(Key::Right, state),
        0x148 => window.key_handler.set_key_state(Key::Up, state),
        0x028 => window.key_handler.set_key_state(Key::Apostrophe, state),
        0x029 => window.key_handler.set_key_state(Key::Backquote, state),
        0x02B => window.key_handler.set_key_state(Key::Backslash, state),
        0x033 => window.key_handler.set_key_state(Key::Comma, state),
        0x00D => window.key_handler.set_key_state(Key::Equal, state),
        0x01A => window.key_handler.set_key_state(Key::LeftBracket, state),
        0x00C => window.key_handler.set_key_state(Key::Minus, state),
        0x034 => window.key_handler.set_key_state(Key::Period, state),
        0x01B => window.key_handler.set_key_state(Key::RightBracket, state),
        0x027 => window.key_handler.set_key_state(Key::Semicolon, state),
        0x035 => window.key_handler.set_key_state(Key::Slash, state),
        0x00E => window.key_handler.set_key_state(Key::Backspace, state),
        0x153 => window.key_handler.set_key_state(Key::Delete, state),
        0x14F => window.key_handler.set_key_state(Key::End, state),
        0x01C => window.key_handler.set_key_state(Key::Enter, state),
        0x001 => window.key_handler.set_key_state(Key::Escape, state),
        0x147 => window.key_handler.set_key_state(Key::Home, state),
        0x152 => window.key_handler.set_key_state(Key::Insert, state),
        0x15D => window.key_handler.set_key_state(Key::Menu, state),
        0x151 => window.key_handler.set_key_state(Key::PageDown, state),
        0x149 => window.key_handler.set_key_state(Key::PageUp, state),
        0x045 => window.key_handler.set_key_state(Key::Pause, state),
        0x039 => window.key_handler.set_key_state(Key::Space, state),
        0x00F => window.key_handler.set_key_state(Key::Tab, state),
        0x145 => window.key_handler.set_key_state(Key::NumLock, state),
        0x03A => window.key_handler.set_key_state(Key::CapsLock, state),
        0x046 => window.key_handler.set_key_state(Key::ScrollLock, state),
        0x02A => window.key_handler.set_key_state(Key::LeftShift, state),
        0x036 => window.key_handler.set_key_state(Key::RightShift, state),
        0x01D => window.key_handler.set_key_state(Key::LeftCtrl, state),
        0x11D => window.key_handler.set_key_state(Key::RightCtrl, state),
        0x052 => window.key_handler.set_key_state(Key::NumPad0, state),
        0x04F => window.key_handler.set_key_state(Key::NumPad1, state),
        0x050 => window.key_handler.set_key_state(Key::NumPad2, state),
        0x051 => window.key_handler.set_key_state(Key::NumPad3, state),
        0x04B => window.key_handler.set_key_state(Key::NumPad4, state),
        0x04C => window.key_handler.set_key_state(Key::NumPad5, state),
        0x04D => window.key_handler.set_key_state(Key::NumPad6, state),
        0x047 => window.key_handler.set_key_state(Key::NumPad7, state),
        0x048 => window.key_handler.set_key_state(Key::NumPad8, state),
        0x049 => window.key_handler.set_key_state(Key::NumPad9, state),
        0x053 => window.key_handler.set_key_state(Key::NumPadDot, state),
        0x135 => window.key_handler.set_key_state(Key::NumPadSlash, state),
        0x037 => window.key_handler.set_key_state(Key::NumPadAsterisk, state),
        0x04A => window.key_handler.set_key_state(Key::NumPadMinus, state),
        0x04E => window.key_handler.set_key_state(Key::NumPadPlus, state),
        0x11C => window.key_handler.set_key_state(Key::NumPadEnter, state),
        _ => (),
    }
}

fn char_down(window: &mut Window, code_point: u32) {
    if let Some(ref mut callback) = window.key_handler.key_callback {
        callback.add_char(code_point);
    }
}

#[cfg(target_arch = "x86_64")]
unsafe fn set_window_long(window: windef::HWND, data: basetsd::LONG_PTR) -> basetsd::LONG_PTR {
    winuser::SetWindowLongPtrW(window, winuser::GWLP_USERDATA, data)
}

#[cfg(target_arch = "x86_64")]
unsafe fn get_window_long(window: windef::HWND) -> basetsd::LONG_PTR {
    winuser::GetWindowLongPtrW(window, winuser::GWLP_USERDATA)
}

#[cfg(target_arch = "x86")]
unsafe fn set_window_long(window: windef::HWND, data: ntdef::LONG) -> ntdef::LONG {
    winuser::SetWindowLongW(window, winuser::GWLP_USERDATA, data)
}

#[cfg(target_arch = "x86")]
unsafe fn get_window_long(window: windef::HWND) -> ntdef::LONG {
    winuser::GetWindowLongW(window, winuser::GWLP_USERDATA)
}

unsafe extern "system" fn wnd_proc(
    window: windef::HWND,
    msg: minwindef::UINT,
    wparam: minwindef::WPARAM,
    lparam: minwindef::LPARAM,
) -> minwindef::LRESULT {
    // This make sure we actually don't do anything before the user data has been setup for the window

    let user_data = get_window_long(window);

    if user_data == 0 {
        return winuser::DefWindowProcW(window, msg, wparam, lparam);
    }

    let mut wnd: &mut Window = mem::transmute(user_data);

    match msg {
        winuser::WM_MOUSEWHEEL => {
            let scroll = ((((wparam as u32) >> 16) & 0xffff) as i16) as f32 * 0.1;
            wnd.mouse.scroll = scroll;
        }

        winuser::WM_SETCURSOR => {
            if winapi::shared::minwindef::LOWORD(lparam as u32) == winuser::HTCLIENT as u16 {
                winuser::SetCursor(wnd.cursors[wnd.cursor as usize]);
                return 1;
            }
        }

        winuser::WM_KEYDOWN => {
            update_key_state(wnd, (lparam as u32) >> 16, true);
            return 0;
        }

        winuser::WM_CHAR => {
            char_down(wnd, wparam as u32);
        }

        winuser::WM_SYSCHAR => {
            char_down(wnd, wparam as u32);
        }

        winuser::WM_LBUTTONDOWN => wnd.mouse.state[0] = true,

        winuser::WM_LBUTTONUP => wnd.mouse.state[0] = false,

        winuser::WM_MBUTTONDOWN => wnd.mouse.state[1] = true,

        winuser::WM_MBUTTONUP => wnd.mouse.state[1] = false,

        winuser::WM_RBUTTONDOWN => wnd.mouse.state[2] = true,

        winuser::WM_RBUTTONUP => wnd.mouse.state[2] = false,

        winuser::WM_CLOSE => {
            wnd.is_open = false;
        }

        winuser::WM_KEYUP => {
            update_key_state(wnd, (lparam as u32) >> 16, false);
            return 0;
        }

        winuser::WM_COMMAND => {
            if lparam == 0 {
                wnd.accel_key = (wparam & 0xffff) as usize;
            }
        }

        /*
        winuser::WM_ERASEBKGND => {
            let dc = wnd.dc.unwrap();
            wingdi::SelectObject(dc, wnd.clear_brush as *mut std::ffi::c_void);
            wingdi::Rectangle(dc, 0, 0, wnd.width, wnd.height);
        }
        */
        winuser::WM_SIZE => {
            let width = (lparam as u32) & 0xffff;
            let height = ((lparam as u32) >> 16) & 0xffff;
            wnd.width = width as i32;
            wnd.height = height as i32;
        }

        winuser::WM_PAINT => {
            // if we have nothing to draw here we return the default function
            if wnd.draw_params.buffer == std::ptr::null() {
                return winuser::DefWindowProcW(window, msg, wparam, lparam);
            }

            let mut bitmap_info: BitmapInfo = mem::zeroed();

            bitmap_info.bmi_header.biSize = mem::size_of::<wingdi::BITMAPINFOHEADER>() as u32;
            bitmap_info.bmi_header.biPlanes = 1;
            bitmap_info.bmi_header.biBitCount = 32;
            bitmap_info.bmi_header.biCompression = wingdi::BI_BITFIELDS;
            bitmap_info.bmi_header.biWidth = wnd.draw_params.buffer_width as i32;
            bitmap_info.bmi_header.biHeight = -(wnd.draw_params.buffer_height as i32);
            bitmap_info.bmi_colors[0].rgbRed = 0xff;
            bitmap_info.bmi_colors[1].rgbGreen = 0xff;
            bitmap_info.bmi_colors[2].rgbBlue = 0xff;

            let buffer_width = wnd.draw_params.buffer_width as i32;
            let buffer_height = wnd.draw_params.buffer_height as i32;
            let window_width = wnd.width as i32;
            let window_height = wnd.height as i32;

            let mut new_height = window_height;
            let mut new_width = window_width;
            let mut x_offset = 0;
            let mut y_offset = 0;

            let dc = wnd.dc.unwrap();
            wingdi::SelectObject(dc, wnd.clear_brush as *mut winapi::ctypes::c_void);

            match wnd.draw_params.scale_mode {
                ScaleMode::AspectRatioStretch => {
                    let buffer_aspect = buffer_width as f32 / buffer_height as f32;
                    let win_aspect = window_width as f32 / window_height as f32;

                    if buffer_aspect > win_aspect {
                        new_height = (window_width as f32 / buffer_aspect) as i32;
                        y_offset = (new_height - window_height) / -2;

                        if y_offset != 0 {
                            wingdi::Rectangle(dc, 0, 0, window_width, y_offset);
                            wingdi::Rectangle(
                                dc,
                                0,
                                y_offset + new_height,
                                window_width,
                                window_height,
                            );
                        }
                    } else {
                        new_width = (window_height as f32 * buffer_aspect) as i32;
                        x_offset = (new_width - window_width) / -2;

                        if x_offset != 0 {
                            wingdi::Rectangle(dc, 0, 0, x_offset, window_height);
                            wingdi::Rectangle(
                                dc,
                                x_offset + new_width,
                                0,
                                window_width,
                                window_height,
                            );
                        }
                    }
                }

                ScaleMode::Center => {
                    new_width = buffer_width;
                    new_height = buffer_height;

                    if buffer_height > window_height {
                        y_offset = -(buffer_height - window_height) / 2;
                    } else {
                        y_offset = (window_height - buffer_height) / 2;
                    }

                    if buffer_width > window_width {
                        x_offset = -(buffer_width - window_width) / 2;
                    } else {
                        x_offset = (window_width - buffer_width) / 2;
                    }

                    if y_offset > 0 {
                        wingdi::Rectangle(dc, 0, 0, window_width, y_offset);
                        wingdi::Rectangle(
                            dc,
                            0,
                            y_offset + new_height,
                            window_width,
                            window_height,
                        );
                    }

                    if x_offset > 0 {
                        wingdi::Rectangle(dc, 0, y_offset, x_offset, buffer_height + y_offset);
                        wingdi::Rectangle(
                            dc,
                            x_offset + buffer_width,
                            y_offset,
                            window_width,
                            buffer_height + y_offset,
                        );
                    }
                }

                ScaleMode::UpperLeft => {
                    new_width = buffer_width;
                    new_height = buffer_height;

                    if buffer_width < window_width {
                        wingdi::Rectangle(dc, buffer_width, 0, window_width, window_height);
                    }

                    if buffer_height < window_height {
                        wingdi::Rectangle(dc, 0, buffer_height, window_width, window_height);
                    }
                }

                _ => (),
            }

            wingdi::StretchDIBits(
                dc,
                x_offset,
                y_offset,
                new_width,
                new_height,
                0,
                0,
                wnd.draw_params.buffer_width as i32,
                wnd.draw_params.buffer_height as i32,
                mem::transmute(wnd.draw_params.buffer),
                mem::transmute(&bitmap_info),
                wingdi::DIB_RGB_COLORS,
                wingdi::SRCCOPY,
            );

            winuser::ValidateRect(window, ptr::null_mut());

            return 0;
        }

        _ => (),
    }

    return winuser::DefWindowProcW(window, msg, wparam, lparam);
}

fn to_wstring(str: &str) -> Vec<u16> {
    let v: Vec<u16> = OsStr::new(str)
        .encode_wide()
        .chain(Some(0).into_iter())
        .collect();
    v
}

#[derive(Default)]
struct MouseData {
    pub x: f32,
    pub y: f32,
    pub state: [bool; 8],
    pub scroll: f32,
}

struct DrawParameters {
    buffer: *const u32,
    buffer_width: u32,
    buffer_height: u32,
    scale_mode: ScaleMode,
}

impl Default for DrawParameters {
    fn default() -> Self {
        DrawParameters {
            buffer: std::ptr::null(),
            buffer_width: 0,
            buffer_height: 0,
            scale_mode: ScaleMode::Stretch,
        }
    }
}

pub struct Window {
    mouse: MouseData,
    dc: Option<windef::HDC>,
    window: Option<windef::HWND>,
    clear_brush: windef::HBRUSH,
    is_open: bool,
    scale_factor: i32,
    width: i32,
    height: i32,
    menus: Vec<Menu>,
    key_handler: KeyHandler,
    update_rate: UpdateRate,
    accel_table: windef::HACCEL,
    accel_key: usize,
    cursor: CursorStyle,
    cursors: [windef::HCURSOR; 8],
    draw_params: DrawParameters,
}

unsafe impl raw_window_handle::HasRawWindowHandle for Window {
    fn raw_window_handle(&self) -> raw_window_handle::RawWindowHandle {
        let mut handle = raw_window_handle::Win32Handle::empty();
        handle.hwnd = self.window.unwrap() as *mut raw::c_void;
        handle.hinstance =
            unsafe { libloaderapi::GetModuleHandleA(ptr::null()) } as *mut raw::c_void;
        raw_window_handle::RawWindowHandle::Win32(handle)
    }
}

impl Window {
    fn open_window(
        name: &str,
        width: usize,
        height: usize,
        opts: WindowOptions,
        scale_factor: i32,
    ) -> Option<windef::HWND> {
        unsafe {
            let class_name = to_wstring("minifb_window");
            let class = winuser::WNDCLASSW {
                style: winuser::CS_HREDRAW | winuser::CS_VREDRAW | winuser::CS_OWNDC,
                lpfnWndProc: Some(wnd_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: libloaderapi::GetModuleHandleA(ptr::null()),
                hIcon: ptr::null_mut(),
                hCursor: winuser::LoadCursorW(ptr::null_mut(), winuser::IDC_ARROW),
                hbrBackground: ptr::null_mut(),
                lpszMenuName: ptr::null(),
                lpszClassName: class_name.as_ptr(),
            };

            if winuser::RegisterClassW(&class) == 0 {
                // ignore the "Class already exists" error for multiple windows
                if errhandlingapi::GetLastError() as u32 != 1410 {
                    println!(
                        "Unable to register class, error {}",
                        errhandlingapi::GetLastError() as u32
                    );
                    return None;
                }
            }

            let window_name = to_wstring(name);

            let mut flags = 0;

            if opts.title {
                flags |= winuser::WS_OVERLAPPEDWINDOW as u32;
            }

            if opts.resize {
                flags |= winuser::WS_THICKFRAME as u32 | winuser::WS_MAXIMIZEBOX as u32;
            } else {
                flags &= !winuser::WS_MAXIMIZEBOX;
                flags &= !winuser::WS_THICKFRAME;
            }

            if opts.borderless {
                flags &= !winuser::WS_THICKFRAME;
            }

            //TODO: UpdateLayeredWindow, etc.
            //https://gist.github.com/texus/31676aba4ca774b1298e1e15133b8141
            if opts.transparency {
                flags &= winuser::WS_EX_LAYERED;
            }

            if opts.none {
                flags = winuser::WS_VISIBLE | winuser::WS_POPUP;
            }

            let new_width = width * scale_factor as usize;
            let new_height = height * scale_factor as usize;

            let mut rect = windef::RECT {
                left: 0,
                right: new_width as ntdef::LONG,
                top: 0,
                bottom: new_height as ntdef::LONG,
            };

            winuser::AdjustWindowRect(&mut rect, flags, 0);

            rect.right -= rect.left;
            rect.bottom -= rect.top;

            let handle = winuser::CreateWindowExW(
                0,
                class_name.as_ptr(),
                window_name.as_ptr(),
                flags,
                winuser::CW_USEDEFAULT,
                winuser::CW_USEDEFAULT,
                rect.right,
                rect.bottom,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
            );
            if handle.is_null() {
                println!(
                    "Unable to create window, error {}",
                    errhandlingapi::GetLastError() as u32
                );
                return None;
            }

            winuser::ShowWindow(handle, winuser::SW_NORMAL);

            return Some(handle);
        }
    }

    pub fn new(name: &str, width: usize, height: usize, opts: WindowOptions) -> Result<Window> {
        unsafe {
            let scale_factor = Self::get_scale_factor(width, height, opts.scale);

            let handle = Self::open_window(name, width, height, opts, scale_factor);

            if handle.is_none() {
                return Err(Error::WindowCreate("Unable to create Window".to_owned()));
            }

            let window = Window {
                mouse: MouseData::default(),
                dc: Some(winuser::GetDC(handle.unwrap())),
                window: Some(handle.unwrap()),
                key_handler: KeyHandler::new(),
                update_rate: UpdateRate::new(),
                is_open: true,
                scale_factor,
                width: (width * scale_factor as usize) as i32,
                height: (height * scale_factor as usize) as i32,
                menus: Vec::new(),
                accel_table: ptr::null_mut(),
                accel_key: INVALID_ACCEL,
                cursor: CursorStyle::Arrow,
                clear_brush: wingdi::CreateSolidBrush(0),
                cursors: [
                    winuser::LoadCursorW(ptr::null_mut(), winuser::IDC_ARROW),
                    winuser::LoadCursorW(ptr::null_mut(), winuser::IDC_IBEAM),
                    winuser::LoadCursorW(ptr::null_mut(), winuser::IDC_CROSS),
                    winuser::LoadCursorW(ptr::null_mut(), winuser::IDC_HAND),
                    winuser::LoadCursorW(ptr::null_mut(), winuser::IDC_HAND),
                    winuser::LoadCursorW(ptr::null_mut(), winuser::IDC_SIZEWE),
                    winuser::LoadCursorW(ptr::null_mut(), winuser::IDC_SIZENS),
                    winuser::LoadCursorW(ptr::null_mut(), winuser::IDC_SIZEALL),
                ],
                draw_params: DrawParameters {
                    scale_mode: opts.scale_mode,
                    ..DrawParameters::default()
                },
            };

            if opts.topmost {
                window.topmost(true)
            }

            Ok(window)
        }
    }

    #[inline]
    pub fn set_title(&mut self, title: &str) {
        unsafe {
            let title_name = to_wstring(title);
            winuser::SetWindowTextW(self.window.unwrap(), title_name.as_ptr());
        }
    }

    #[inline]
    pub fn get_window_handle(&self) -> *mut raw::c_void {
        self.window.unwrap() as *mut raw::c_void
    }

    #[inline]
    pub fn set_position(&mut self, x: isize, y: isize) {
        unsafe {
            winuser::SetWindowPos(
                self.window.unwrap(),
                ptr::null_mut(),
                x as i32,
                y as i32,
                0,
                0,
                winuser::SWP_SHOWWINDOW | winuser::SWP_NOSIZE,
            );
        }
    }

    #[inline]
    pub fn get_position(&self) -> (isize, isize) {
        let (mut x, mut y) = (0, 0);

        unsafe {
            let mut rect = windef::RECT {
                left: 0,
                right: 0,
                top: 0,
                bottom: 0,
            };
            if winuser::GetWindowRect(self.window.unwrap(), &mut rect) != 0 {
                x = rect.left;
                y = rect.top;
            }
        }
        (x as isize, y as isize)
    }

    #[inline]
    pub fn topmost(&self, topmost: bool) {
        unsafe {
            winuser::SetWindowPos(
                self.window.unwrap(),
                if topmost == true {
                    winuser::HWND_TOPMOST
                } else {
                    winuser::HWND_TOP
                },
                0,
                0,
                0,
                0,
                winuser::SWP_SHOWWINDOW | winuser::SWP_NOSIZE | winuser::SWP_NOMOVE,
            )
        };
    }

    #[inline]
    pub fn get_size(&self) -> (usize, usize) {
        (self.width as usize, self.height as usize)
    }

    pub fn get_mouse_pos(&self, mode: MouseMode) -> Option<(f32, f32)> {
        let s = self.scale_factor as f32;
        let w = self.width as f32;
        let h = self.height as f32;

        // TODO: Needs to be fixed with resize support
        mouse_handler::get_pos(mode, self.mouse.x, self.mouse.y, s, w, h)
    }

    pub fn get_unscaled_mouse_pos(&self, mode: MouseMode) -> Option<(f32, f32)> {
        let w = self.width as f32;
        let h = self.height as f32;

        // TODO: Needs to be fixed with resize support
        mouse_handler::get_pos(mode, self.mouse.x, self.mouse.y, 1.0, w, h)
    }

    pub fn get_mouse_down(&self, button: MouseButton) -> bool {
        match button {
            MouseButton::Left => self.mouse.state[0],
            MouseButton::Middle => self.mouse.state[1],
            MouseButton::Right => self.mouse.state[2],
        }
    }

    pub fn get_scroll_wheel(&self) -> Option<(f32, f32)> {
        if self.mouse.scroll.abs() > 0.0 {
            Some((0.0, self.mouse.scroll))
        } else {
            None
        }
    }

    #[inline]
    pub fn set_cursor_style(&mut self, cursor: CursorStyle) {
        self.cursor = cursor;
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
    pub fn set_input_callback(&mut self, callback: Box<dyn InputCallback>) {
        self.key_handler.set_input_callback(callback)
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
    pub fn is_open(&self) -> bool {
        return self.is_open;
    }

    fn generic_update(&mut self, window: windef::HWND) {
        unsafe {
            let mut point: windef::POINT = mem::zeroed();

            winuser::GetCursorPos(&mut point);
            winuser::ScreenToClient(window, &mut point);

            self.mouse.x = point.x as f32;
            self.mouse.y = point.y as f32;
            self.mouse.scroll = 0.0;

            self.key_handler.update();

            set_window_long(window, mem::transmute(self));
        }
    }

    fn message_loop(&self, _window: windef::HWND) {
        unsafe {
            let mut msg = mem::zeroed();

            while winuser::PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, winuser::PM_REMOVE)
                != 0
            {
                // Make this code a bit nicer
                if self.accel_table == ptr::null_mut() {
                    winuser::TranslateMessage(&mut msg);
                    winuser::DispatchMessageW(&mut msg);
                } else {
                    if winuser::TranslateAcceleratorW(
                        msg.hwnd,
                        mem::transmute(self.accel_table),
                        &mut msg,
                    ) == 0
                    {
                        winuser::TranslateMessage(&mut msg);
                        winuser::DispatchMessageW(&mut msg);
                    }
                }
            }
        }
    }

    pub fn set_background_color(&mut self, color: u32) {
        unsafe {
            wingdi::DeleteObject(self.clear_brush as *mut winapi::ctypes::c_void);
            let r = (color >> 16) & 0xff;
            let g = (color >> 8) & 0xff;
            let b = (color >> 0) & 0xff;
            self.clear_brush = wingdi::CreateSolidBrush((b << 16) | (g << 8) | r);
        }
    }

    pub fn set_cursor_visibility(&mut self, visibility: bool) {
        unsafe {
            winuser::ShowCursor(visibility as i32);
        }
    }

    pub fn update_with_buffer_stride(
        &mut self,
        buffer: &[u32],
        buf_width: usize,
        buf_height: usize,
        buf_stride: usize,
    ) -> Result<()> {
        let window = self.window.unwrap();

        Self::generic_update(self, window);

        buffer_helper::check_buffer_size(buf_width, buf_height, buf_stride, buffer)?;

        self.draw_params.buffer = buffer.as_ptr();
        self.draw_params.buffer_width = buf_width as u32;
        self.draw_params.buffer_height = buf_height as u32;
        // stride currently not supported
        //self.draw_params.buffer_stride = buf_stride as u32;

        unsafe {
            winuser::InvalidateRect(window, ptr::null_mut(), minwindef::TRUE);
        }

        Self::message_loop(self, window);

        Ok(())
    }

    pub fn update(&mut self) {
        let window = self.window.unwrap();

        Self::generic_update(self, window);
        Self::message_loop(self, window);
    }

    #[inline]
    pub fn is_active(&mut self) -> bool {
        match self.window {
            Some(hwnd) => {
                let active = unsafe { winapi::um::winuser::GetActiveWindow() };
                if !active.is_null() && active == hwnd {
                    true
                } else {
                    false
                }
            }
            None => false,
        }
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
                let screen_x = winuser::GetSystemMetrics(winuser::SM_CXSCREEN) as i32;
                let screen_y = winuser::GetSystemMetrics(winuser::SM_CYSCREEN) as i32;

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

    //
    // When attaching a menu to the window we need to resize it so
    // the current client size is preserved and still show all pixels
    //
    unsafe fn adjust_window_size_for_menu(handle: windef::HWND) {
        let mut rect: windef::RECT = mem::zeroed();

        let menu_height = winuser::GetSystemMetrics(winuser::SM_CYMENU);

        winuser::GetWindowRect(handle, &mut rect);
        winuser::MoveWindow(
            handle,
            rect.left,
            rect.top,
            rect.right - rect.left,
            (rect.bottom - rect.top) + menu_height,
            1,
        );
    }

    unsafe fn set_accel_table(&mut self) {
        let mut temp_accel_table = Vec::<winuser::ACCEL>::new();

        for menu in self.menus.iter() {
            for item in menu.accel_table.iter() {
                temp_accel_table.push(item.clone());
            }
        }

        if self.accel_table != ptr::null_mut() {
            winuser::DestroyAcceleratorTable(self.accel_table);
        }

        self.accel_table = winuser::CreateAcceleratorTableW(
            temp_accel_table.as_mut_ptr(),
            temp_accel_table.len() as i32,
        );
    }

    pub fn add_menu(&mut self, menu: &Menu) -> MenuHandle {
        unsafe {
            let window = self.window.unwrap();
            let mut main_menu = winuser::GetMenu(window);

            if main_menu == ptr::null_mut() {
                main_menu = winuser::CreateMenu();
                winuser::SetMenu(window, main_menu);
                Self::adjust_window_size_for_menu(window);
            }

            winuser::AppendMenuW(
                main_menu,
                0x10,
                menu.menu_handle as basetsd::UINT_PTR,
                menu.name.as_ptr(),
            );

            self.menus.push(menu.clone());
            // TODO: Setup accel table

            //Self::add_menu_store(self, main_menu, menu_name, menu);
            self.set_accel_table();

            winuser::DrawMenuBar(window);
        }

        // TODO: Proper handle

        MenuHandle(menu.menu_handle as u64)
    }

    pub fn remove_menu(&mut self, handle: MenuHandle) {
        let window = self.window.unwrap();
        let main_menu = unsafe { winuser::GetMenu(window) };
        for i in 0..self.menus.len() {
            if self.menus[i].menu_handle == handle.0 as windef::HMENU {
                unsafe {
                    let _t = winuser::RemoveMenu(main_menu, i as minwindef::UINT, 0);
                    winuser::DrawMenuBar(self.window.unwrap());
                }
                self.menus.swap_remove(i);
                return;
            }
        }
    }

    pub fn is_menu_pressed(&mut self) -> Option<usize> {
        if self.accel_key == INVALID_ACCEL {
            None
        } else {
            let t = self.accel_key;
            self.accel_key = INVALID_ACCEL;
            Some(t)
        }
    }
}

#[derive(Clone)]
pub struct Menu {
    menu_handle: windef::HMENU,
    name: Vec<u16>,
    accel_table: Vec<winuser::ACCEL>,
}

impl Menu {
    pub fn new(name: &str) -> Result<Menu> {
        unsafe {
            Ok(Menu {
                menu_handle: winuser::CreatePopupMenu(),
                name: to_wstring(name),
                accel_table: Vec::new(),
            })
        }
    }

    fn map_key_to_vk_accel(key: Key) -> (raw::c_int, &'static str) {
        match key {
            Key::Key0 => (0x30, "0"),
            Key::Key1 => (0x31, "1"),
            Key::Key2 => (0x32, "2"),
            Key::Key3 => (0x33, "3"),
            Key::Key4 => (0x34, "4"),
            Key::Key5 => (0x35, "5"),
            Key::Key6 => (0x36, "6"),
            Key::Key7 => (0x37, "7"),
            Key::Key8 => (0x38, "8"),
            Key::Key9 => (0x39, "9"),

            Key::A => (0x41, "a"),
            Key::B => (0x42, "b"),
            Key::C => (0x43, "c"),
            Key::D => (0x44, "d"),
            Key::E => (0x45, "e"),
            Key::F => (0x46, "f"),
            Key::G => (0x47, "g"),
            Key::H => (0x48, "h"),
            Key::I => (0x49, "i"),
            Key::J => (0x4a, "j"),
            Key::K => (0x4b, "k"),
            Key::L => (0x4c, "l"),
            Key::M => (0x4d, "m"),
            Key::N => (0x4e, "n"),
            Key::O => (0x4f, "o"),
            Key::P => (0x50, "p"),
            Key::Q => (0x51, "q"),
            Key::R => (0x52, "r"),
            Key::S => (0x53, "s"),
            Key::T => (0x54, "t"),
            Key::U => (0x55, "u"),
            Key::V => (0x56, "v"),
            Key::W => (0x57, "w"),
            Key::X => (0x58, "x"),
            Key::Y => (0x59, "y"),
            Key::Z => (0x5a, "z"),

            Key::F1 => (winuser::VK_F1, "F1"),
            Key::F2 => (winuser::VK_F2, "F2"),
            Key::F3 => (winuser::VK_F3, "F3"),
            Key::F4 => (winuser::VK_F4, "F4"),
            Key::F5 => (winuser::VK_F5, "F5"),
            Key::F6 => (winuser::VK_F6, "F6"),
            Key::F7 => (winuser::VK_F7, "F7"),
            Key::F8 => (winuser::VK_F8, "F8"),
            Key::F9 => (winuser::VK_F9, "F9"),
            Key::F10 => (winuser::VK_F10, "F10"),
            Key::F11 => (winuser::VK_F11, "F11"),
            Key::F12 => (winuser::VK_F12, "F12"),
            Key::F13 => (winuser::VK_F13, "F14"),
            Key::F14 => (winuser::VK_F14, "F14"),
            Key::F15 => (winuser::VK_F15, "F15"),

            Key::Down => (winuser::VK_DOWN, "Down"),
            Key::Left => (winuser::VK_LEFT, "Left"),
            Key::Right => (winuser::VK_RIGHT, "Right"),
            Key::Up => (winuser::VK_UP, "Up"),

            Key::Backslash => (winuser::VK_OEM_102, "Backslash"),
            Key::Comma => (winuser::VK_OEM_COMMA, ","),
            Key::Minus => (winuser::VK_OEM_MINUS, "-"),
            Key::Period => (winuser::VK_OEM_PERIOD, "."),

            Key::Backspace => (winuser::VK_BACK, "Back"),
            Key::Delete => (winuser::VK_DELETE, "Delete"),
            Key::End => (winuser::VK_END, "End"),
            Key::Enter => (winuser::VK_RETURN, "Enter"),

            Key::Escape => (winuser::VK_ESCAPE, "Esc"),

            Key::Home => (winuser::VK_HOME, "Home"),
            Key::Insert => (winuser::VK_INSERT, "Insert"),
            Key::Menu => (winuser::VK_MENU, "Menu"),

            Key::PageDown => (winuser::VK_NEXT, "PageDown"),
            Key::PageUp => (winuser::VK_PRIOR, "PageUp"),

            Key::Pause => (winuser::VK_PAUSE, "Pause"),
            Key::Space => (winuser::VK_SPACE, "Space"),
            Key::Tab => (winuser::VK_TAB, "Tab"),
            Key::NumLock => (winuser::VK_NUMLOCK, "NumLock"),
            Key::CapsLock => (winuser::VK_CAPITAL, "CapsLock"),
            Key::ScrollLock => (winuser::VK_SCROLL, "Scroll"),

            Key::LeftShift => (winuser::VK_LSHIFT, "LeftShift"),
            Key::RightShift => (winuser::VK_RSHIFT, "RightShift"),
            Key::LeftCtrl => (winuser::VK_CONTROL, "Ctrl"),
            Key::RightCtrl => (winuser::VK_CONTROL, "Ctrl"),

            Key::NumPad0 => (winuser::VK_NUMPAD0, "NumPad0"),
            Key::NumPad1 => (winuser::VK_NUMPAD1, "NumPad1"),
            Key::NumPad2 => (winuser::VK_NUMPAD2, "NumPad2"),
            Key::NumPad3 => (winuser::VK_NUMPAD3, "NumPad3"),
            Key::NumPad4 => (winuser::VK_NUMPAD4, "NumPad4"),
            Key::NumPad5 => (winuser::VK_NUMPAD5, "NumPad5"),
            Key::NumPad6 => (winuser::VK_NUMPAD6, "NumPad6"),
            Key::NumPad7 => (winuser::VK_NUMPAD7, "NumPad7"),
            Key::NumPad8 => (winuser::VK_NUMPAD8, "NumPad8"),
            Key::NumPad9 => (winuser::VK_NUMPAD9, "NumPad9"),

            Key::LeftAlt => (winuser::VK_MENU, "Alt"),
            Key::RightAlt => (winuser::VK_MENU, "Alt"),

            Key::LeftSuper => (winuser::VK_LWIN, "LeftWin"),
            Key::RightSuper => (winuser::VK_RWIN, "RightWin"),

            _ => (0, "Unsupported"),
        }
    }

    pub fn add_sub_menu(&mut self, name: &str, menu: &Menu) {
        unsafe {
            let menu_name = to_wstring(name);
            winuser::AppendMenuW(
                self.menu_handle,
                0x10,
                menu.menu_handle as basetsd::UINT_PTR,
                menu_name.as_ptr(),
            );
            self.accel_table
                .extend_from_slice(menu.accel_table.as_slice());
        }
    }

    fn format_name(menu_item: &MenuItem, key_name: &'static str) -> String {
        let mut name = menu_item.label.clone();

        name.push_str("\t");

        if (menu_item.modifier & MENU_KEY_WIN) == MENU_KEY_WIN {
            name.push_str("Win-");
        }

        if (menu_item.modifier & MENU_KEY_SHIFT) == MENU_KEY_SHIFT {
            name.push_str("Shift-");
        }

        if (menu_item.modifier & MENU_KEY_CTRL) == MENU_KEY_CTRL {
            name.push_str("Ctrl-");
        }

        if (menu_item.modifier & MENU_KEY_ALT) == MENU_KEY_ALT {
            name.push_str("Alt-");
        }

        name.push_str(key_name);

        name
    }

    fn is_key_virtual_range(_key: raw::c_int) -> u32 {
        /*
        if (key >= 0x30 && key <= 0x30) ||
           (key >= 0x41 && key <= 0x5a) {
            0
           } else {
            1
        }
        */

        1
    }

    fn get_virt_key(menu_item: &MenuItem, key: raw::c_int) -> u32 {
        let mut virt = Self::is_key_virtual_range(key);

        if (menu_item.modifier & MENU_KEY_ALT) == MENU_KEY_ALT {
            virt |= 0x10;
        }

        if (menu_item.modifier & MENU_KEY_CTRL) == MENU_KEY_CTRL {
            virt |= 0x8;
        }

        if (menu_item.modifier & MENU_KEY_SHIFT) == MENU_KEY_SHIFT {
            virt |= 0x4;
        }

        virt
    }

    fn add_accel(&mut self, vk: raw::c_int, menu_item: &MenuItem) {
        let vk_accel = Self::map_key_to_vk_accel(menu_item.key);
        let virt = Self::get_virt_key(menu_item, vk);
        let accel = winuser::ACCEL {
            fVirt: virt as minwindef::BYTE,
            cmd: menu_item.id as minwindef::WORD,
            key: vk_accel.0 as minwindef::WORD,
        };

        self.accel_table.push(accel);
    }

    pub fn add_menu_item(&mut self, menu_item: &MenuItem) -> MenuItemHandle {
        let vk_accel = Self::map_key_to_vk_accel(menu_item.key);

        unsafe {
            match vk_accel.0 {
                0 => {
                    let item_name = to_wstring(&menu_item.label);
                    winuser::AppendMenuW(
                        self.menu_handle,
                        0x10,
                        menu_item.id as basetsd::UINT_PTR,
                        item_name.as_ptr(),
                    );
                }
                _ => {
                    let menu_name = Self::format_name(menu_item, vk_accel.1);
                    let w_name = to_wstring(&menu_name);
                    winuser::AppendMenuW(
                        self.menu_handle,
                        0x10,
                        menu_item.id as basetsd::UINT_PTR,
                        w_name.as_ptr(),
                    );
                    self.add_accel(vk_accel.0, menu_item);
                }
            }
        }

        // TODO: This is not correct and needs to be fixed if remove_item is added. The
        // issue here is that AppendMenuW doesn't return a handle so it's hard to track
        // in an easy way :(

        MenuItemHandle(0)
    }

    pub fn remove_item(&mut self, _item: &MenuItemHandle) {
        panic!("remove item hasn't been implemented");
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            if self.dc.is_some() {
                winuser::ReleaseDC(self.window.unwrap(), self.dc.unwrap());
            }

            if self.window.is_some() {
                winuser::DestroyWindow(self.window.unwrap());
            }
        }
    }
}
