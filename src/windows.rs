extern crate user32;
extern crate kernel32;
extern crate winapi;
extern crate gdi32;

use std::ffi::CString;
use std::ptr;
use std::os::windows::ffi::OsStrExt;
use std::ffi::OsStr;
use std::mem;

use self::winapi::windef::HWND;
use self::winapi::winuser::WS_OVERLAPPEDWINDOW;
use self::winapi::winuser::WNDCLASSW;
use self::winapi::wingdi::BITMAPINFOHEADER;
use self::winapi::wingdi::RGBQUAD;

static mut CLOSE_APP: bool = false;

// Wrap this so we can have a proper numbef of bmiColors to write in
#[repr(C)]
struct BitmapInfo {
    pub bmi_header: BITMAPINFOHEADER,
    pub bmi_colors: [RGBQUAD; 3],
}

unsafe extern "system" fn wnd_proc(window: winapi::HWND,
                                   msg: winapi::UINT,
                                   wparam: winapi::WPARAM,
                                   lparam: winapi::LPARAM)
                                   -> winapi::LRESULT {
    match msg {
        winapi::winuser::WM_KEYDOWN => {
            if (wparam & 0xff) == 27 {
                CLOSE_APP = true;
            }
        }

        winapi::winuser::WM_PAINT => {
            let mut rect: winapi::RECT = mem::uninitialized();
            let buffer = user32::GetWindowLongPtrW(window, winapi::winuser::GWLP_USERDATA);

            user32::GetClientRect(window, &mut rect);

            let mut bitmap_info: BitmapInfo = mem::zeroed();
            let width = rect.right - rect.left;
            let height = rect.bottom - rect.top;

            bitmap_info.bmi_header.biSize = mem::size_of::<BITMAPINFOHEADER>() as u32;
            bitmap_info.bmi_header.biPlanes = 1;
            bitmap_info.bmi_header.biBitCount = 32;
            bitmap_info.bmi_header.biCompression = winapi::wingdi::BI_BITFIELDS;
            bitmap_info.bmi_header.biWidth = width;
            bitmap_info.bmi_header.biHeight = -height;
            bitmap_info.bmi_colors[0].rgbRed = 0xff;
            bitmap_info.bmi_colors[1].rgbGreen = 0xff;
            bitmap_info.bmi_colors[2].rgbBlue = 0xff;

            let dc = user32::GetDC(window);

            gdi32::StretchDIBits(dc,
                                 0,
                                 0,
                                 width,
                                 height,
                                 0,
                                 0,
                                 width,
                                 height,
                                 mem::transmute(buffer),
                                 mem::transmute(&bitmap_info),
                                 winapi::wingdi::DIB_RGB_COLORS,
                                 winapi::wingdi::SRCCOPY);

            user32::ValidateRect(window, ptr::null_mut());

            user32::ReleaseDC(window, dc);
        }
        _ => (),
    }

    return user32::DefWindowProcW(window, msg, wparam, lparam);
}

pub enum MinifbError {
    UnableToCreateWindow,
}

fn to_wstring(str: &str) -> *const u16 {
    let v: Vec<u16> = OsStr::new(str).encode_wide().chain(Some(0).into_iter()).collect();
    v.as_ptr()
}

pub struct Minifb {
    window: HWND,
}

impl Minifb {
    fn open_window(name: &str, width: usize, height: usize) -> HWND {
        unsafe {
            let class_name = to_wstring("minifb_window");
            let s = CString::new(name).unwrap();

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
                lpszClassName: class_name,
            };

            user32::RegisterClassW(&class);

            let mut rect = winapi::RECT {
                left: 0,
                right: width as winapi::LONG,
                top: 0,
                bottom: height as winapi::LONG,
            };

            user32::AdjustWindowRect(&mut rect,
                                     winapi::WS_POPUP | winapi::WS_SYSMENU | winapi::WS_CAPTION,
                                     0);

            rect.right -= rect.left;
            rect.bottom -= rect.top;

            let handle = user32::CreateWindowExA(0,
                                                 "minifb_window".as_ptr() as *mut _,
                                                 s.as_ptr(),
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

            if !handle.is_null() {
                user32::ShowWindow(handle, winapi::SW_NORMAL);
            }

            return handle;
        }
    }

    pub fn new(name: &str, width: usize, height: usize) -> Option<Minifb> {
        let handle = Minifb::open_window(name, width, height);

        match handle.is_null() {
            true => None,
            false => Some(Minifb { window: handle }), 
        }
    }

    pub fn update(&mut self, buffer: &[u32]) -> bool {
        unsafe {
            let mut msg = mem::uninitialized();

            user32::SetWindowLongPtrW(self.window,
                                      winapi::winuser::GWLP_USERDATA,
                                      buffer.as_ptr() as i64);
            user32::InvalidateRect(self.window, ptr::null_mut(), winapi::TRUE);

            while user32::PeekMessageW(&mut msg, self.window, 0, 0, winapi::winuser::PM_REMOVE) != 0 {
                user32::TranslateMessage(&mut msg);
                user32::DispatchMessageW(&mut msg);
            }
        }

        unsafe {
            return !CLOSE_APP;
        }
    }
}
