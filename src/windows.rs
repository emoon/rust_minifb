#![cfg(target_os = "windows")]

extern crate user32;
extern crate kernel32;
extern crate winapi;
extern crate gdi32;

use Scale;
use Vsync;
use Key;

use std::ptr;
use std::os::windows::ffi::OsStrExt;
use std::ffi::OsStr;
use std::mem;

use self::winapi::windef::HWND;
use self::winapi::windef::HDC;
use self::winapi::winuser::WS_OVERLAPPEDWINDOW;
use self::winapi::winuser::WNDCLASSW;
use self::winapi::wingdi::BITMAPINFOHEADER;
use self::winapi::wingdi::RGBQUAD;

// Wrap this so we can have a proper numbef of bmiColors to write in
#[repr(C)]
struct BitmapInfo {
    pub bmi_header: BITMAPINFOHEADER,
    pub bmi_colors: [RGBQUAD; 3],
}

fn update_key_state(window: &mut Window, wparam: u32, state: bool) {
    match wparam & 0x1ff {
        0x00B => window.keys[Key::Key0 as usize] = state,
        0x002 => window.keys[Key::Key1 as usize] = state,
        0x003 => window.keys[Key::Key2 as usize] = state,
        0x004 => window.keys[Key::Key3 as usize] = state,
        0x005 => window.keys[Key::Key4 as usize] = state,
        0x006 => window.keys[Key::Key5 as usize] = state,
        0x007 => window.keys[Key::Key6 as usize] = state,
        0x008 => window.keys[Key::Key7 as usize] = state,
        0x009 => window.keys[Key::Key8 as usize] = state,
        0x00A => window.keys[Key::Key9 as usize] = state,
        0x01E => window.keys[Key::A as usize] = state,
        0x030 => window.keys[Key::B as usize] = state,
        0x02E => window.keys[Key::C as usize] = state,
        0x020 => window.keys[Key::D as usize] = state,
        0x012 => window.keys[Key::E as usize] = state,
        0x021 => window.keys[Key::F as usize] = state,
        0x022 => window.keys[Key::G as usize] = state,
        0x023 => window.keys[Key::H as usize] = state,
        0x017 => window.keys[Key::I as usize] = state,
        0x024 => window.keys[Key::J as usize] = state,
        0x025 => window.keys[Key::K as usize] = state,
        0x026 => window.keys[Key::L as usize] = state,
        0x032 => window.keys[Key::M as usize] = state,
        0x031 => window.keys[Key::N as usize] = state,
        0x018 => window.keys[Key::O as usize] = state,
        0x019 => window.keys[Key::P as usize] = state,
        0x010 => window.keys[Key::Q as usize] = state,
        0x013 => window.keys[Key::R as usize] = state,
        0x01F => window.keys[Key::S as usize] = state,
        0x014 => window.keys[Key::T as usize] = state,
        0x016 => window.keys[Key::U as usize] = state,
        0x02F => window.keys[Key::V as usize] = state,
        0x011 => window.keys[Key::W as usize] = state,
        0x02D => window.keys[Key::X as usize] = state,
        0x015 => window.keys[Key::Y as usize] = state,
        0x02C => window.keys[Key::Z as usize] = state,
        0x03B => window.keys[Key::F1 as usize] = state,
        0x03C => window.keys[Key::F2 as usize] = state,
        0x03D => window.keys[Key::F3 as usize] = state,
        0x03E => window.keys[Key::F4 as usize] = state,
        0x03F => window.keys[Key::F5 as usize] = state,
        0x040 => window.keys[Key::F6 as usize] = state,
        0x041 => window.keys[Key::F7 as usize] = state,
        0x042 => window.keys[Key::F8 as usize] = state,
        0x043 => window.keys[Key::F9 as usize] = state,
        0x044 => window.keys[Key::F10 as usize] = state,
        0x057 => window.keys[Key::F11 as usize] = state,
        0x058 => window.keys[Key::F12 as usize] = state,
        0x150 => window.keys[Key::Down as usize] = state,
        0x14B => window.keys[Key::Left as usize] = state,
        0x14D => window.keys[Key::Right as usize] = state,
        0x148 => window.keys[Key::Up as usize] = state,
        0x028 => window.keys[Key::Apostrophe as usize] = state,
        0x02B => window.keys[Key::Backslash as usize] = state,
        0x033 => window.keys[Key::Comma as usize] = state,
        0x00D => window.keys[Key::Equal as usize] = state,
        0x01A => window.keys[Key::LeftBracket as usize] = state,
        0x00C => window.keys[Key::Minus as usize] = state,
        0x034 => window.keys[Key::Period as usize] = state,
        0x01B => window.keys[Key::RightBracket as usize] = state,
        0x027 => window.keys[Key::Semicolon as usize] = state,
        0x035 => window.keys[Key::Slash as usize] = state,
        0x00E => window.keys[Key::Backspace as usize] = state,
        0x153 => window.keys[Key::Delete as usize] = state,
        0x14F => window.keys[Key::End as usize] = state,
        0x01C => window.keys[Key::Enter as usize] = state,
        0x001 => window.keys[Key::Escape as usize] = state,
        0x147 => window.keys[Key::Home as usize] = state,
        0x152 => window.keys[Key::Insert as usize] = state,
        0x15D => window.keys[Key::Menu as usize] = state,
        0x151 => window.keys[Key::PageDown as usize] = state,
        0x149 => window.keys[Key::PageUp as usize] = state,
        0x045 => window.keys[Key::Pause as usize] = state,
        0x039 => window.keys[Key::Space as usize] = state,
        0x00F => window.keys[Key::Tab as usize] = state,
        0x03A => window.keys[Key::CapsLock as usize] = state,
        _ => (),
    }
}


unsafe extern "system" fn wnd_proc(window: winapi::HWND,
                                   msg: winapi::UINT,
                                   wparam: winapi::WPARAM,
                                   lparam: winapi::LPARAM)
                                   -> winapi::LRESULT {
    // This make sure we actually don't do anything before the user data has been setup for the
    // window

    let user_data = user32::GetWindowLongPtrW(window, winapi::winuser::GWLP_USERDATA);

    if user_data == 0 {
        return user32::DefWindowProcW(window, msg, wparam, lparam);
    }

    let mut wnd: &mut Window = mem::transmute(user_data);

    match msg {
        winapi::winuser::WM_KEYDOWN => {
            update_key_state(wnd, (lparam as u32) >> 16, true);
            return 0;
        }

        winapi::winuser::WM_CLOSE => {
            wnd.is_open = false;
        }

        winapi::winuser::WM_KEYUP => {
            update_key_state(wnd, (lparam as u32) >> 16, false);
            return 0;
        }

        winapi::winuser::WM_PAINT => {
            let mut bitmap_info: BitmapInfo = mem::zeroed();

            bitmap_info.bmi_header.biSize = mem::size_of::<BITMAPINFOHEADER>() as u32;
            bitmap_info.bmi_header.biPlanes = 1;
            bitmap_info.bmi_header.biBitCount = 32;
            bitmap_info.bmi_header.biCompression = winapi::wingdi::BI_BITFIELDS;
            bitmap_info.bmi_header.biWidth = wnd.width;
            bitmap_info.bmi_header.biHeight = -wnd.height;
            bitmap_info.bmi_colors[0].rgbRed = 0xff;
            bitmap_info.bmi_colors[1].rgbGreen = 0xff;
            bitmap_info.bmi_colors[2].rgbBlue = 0xff;

            gdi32::StretchDIBits(wnd.dc.unwrap(),
                                 0,
                                 0,
                                 wnd.width * wnd.scale_factor,
                                 wnd.height * wnd.scale_factor,
                                 0,
                                 0,
                                 wnd.width,
                                 wnd.height,
                                 mem::transmute(wnd.buffer.as_ptr()),
                                 mem::transmute(&bitmap_info),
                                 winapi::wingdi::DIB_RGB_COLORS,
                                 winapi::wingdi::SRCCOPY);

            user32::ValidateRect(window, ptr::null_mut());

            return 0;
        }

        _ => (),
    }

    return user32::DefWindowProcW(window, msg, wparam, lparam);
}

pub enum MinifbError {
    UnableToCreateWindow,
}

fn to_wstring(str: &str) -> Vec<u16> {
    let mut v: Vec<u16> = OsStr::new(str).encode_wide().chain(Some(0).into_iter()).collect();
    v.push(0u16);
    v
}

pub struct Window {
    dc: Option<HDC>,
    window: Option<HWND>,
    keys: [bool; 512],
    buffer: Vec<u32>,
    is_open : bool,
    scale_factor: i32,
    width: i32,
    height: i32,
}

impl Window {
    fn open_window(name: &str, width: usize, height: usize, scale: Scale, _: Vsync) -> Option<HWND> {
        unsafe {
            let class_name = to_wstring("minifb_window");
            let class = WNDCLASSW {
                style: winapi::CS_HREDRAW | winapi::CS_VREDRAW | winapi::CS_OWNDC,
                lpfnWndProc: Some(wnd_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: kernel32::GetModuleHandleA(ptr::null()),
                hIcon: ptr::null_mut(),
                hCursor: ptr::null_mut(),
                hbrBackground: ptr::null_mut(),
                lpszMenuName: ptr::null(),
                lpszClassName: class_name.as_ptr(),
            };

            if user32::RegisterClassW(&class) == 0 {
                println!("Unable to register class, error {}", kernel32::GetLastError() as u32);
                return None;
            }

            let scale_factor = Self::get_scale_factor(scale);
            let new_width = width * scale_factor as usize;
            let new_height = height * scale_factor as usize;

            let mut rect = winapi::RECT {
                left: 0,
                right: new_width as winapi::LONG,
                top: 0,
                bottom: new_height as winapi::LONG,
            };

            user32::AdjustWindowRect(&mut rect,
                                     winapi::WS_POPUP | winapi::WS_SYSMENU | winapi::WS_CAPTION,
                                     0);

            rect.right -= rect.left;
            rect.bottom -= rect.top;

            let window_name = to_wstring(name);

            let handle = user32::CreateWindowExW(0,
                                                 class_name.as_ptr(), 
                                                 window_name.as_ptr(),
                                                 winapi::WS_OVERLAPPEDWINDOW &
                                                 !winapi::WS_MAXIMIZEBOX &
                                                 !winapi::WS_THICKFRAME,
                                                 winapi::CW_USEDEFAULT,
                                                 winapi::CW_USEDEFAULT,
                                                 rect.right,
                                                 rect.bottom,
                                                 ptr::null_mut(),
                                                 ptr::null_mut(),
                                                 ptr::null_mut(),
                                                 ptr::null_mut());
            if handle.is_null() {
                println!("Unable to create window, error {}", kernel32::GetLastError() as u32);
                return None;
            }

            user32::ShowWindow(handle, winapi::SW_NORMAL);

            return Some(handle);
        }
    }

    pub fn new(name: &str,
               width: usize,
               height: usize,
               scale: Scale,
               vsync: Vsync)
               -> Result<Window, &str> {
        unsafe {
            let handle = Self::open_window(name, width, height, scale, vsync);

            if handle.is_none() {
                return Err("Unable to create Window");
            }

            let window = Window {
                dc: Some(user32::GetDC(handle.unwrap())),
                window: Some(handle.unwrap()),
                keys: [false; 512],
                buffer: Vec::new(),
                is_open: true,
                scale_factor: Self::get_scale_factor(scale),
                width: width as i32,
                height: height as i32,
            };

            Ok(window)
        }
    }

    #[inline]
    pub fn is_key_down(&self, key: Key) -> bool {
        return self.keys[key as usize];
    }

    #[inline]
    pub fn is_open(&self) -> bool {
        return self.is_open
    }

    pub fn update(&mut self, buffer: &[u32]) {
        unsafe {
            let mut msg = mem::uninitialized();
            let window = self.window.unwrap();

            // TODO: Optimize

            self.buffer = buffer.iter().cloned().collect();

            user32::SetWindowLongPtrW(window, winapi::winuser::GWLP_USERDATA, mem::transmute(self));
            user32::InvalidateRect(window, ptr::null_mut(), winapi::TRUE);

            while user32::PeekMessageW(&mut msg, window, 0, 0, winapi::winuser::PM_REMOVE) != 0 {
                user32::TranslateMessage(&mut msg);
                user32::DispatchMessageW(&mut msg);
            }
        }
    }

    fn get_scale_factor(scale: Scale) -> i32 {
        // TODO: Implement best fit
        let factor: i32 = match scale {
            Scale::X1 => 1,
            Scale::X2 => 2,
            Scale::X4 => 4,
            Scale::X8 => 8,
            Scale::X16 => 16,
            Scale::X32 => 32,
            _ => 1,
        };

        return factor;
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            if self.dc.is_some() {
                user32::ReleaseDC(self.window.unwrap(), self.dc.unwrap());
            }

            if self.window.is_some() {
                user32::CloseWindow(self.window.unwrap());
            }
        }
    }
}
