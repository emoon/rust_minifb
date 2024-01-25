use super::common::{
    image_center, image_resize_linear, image_resize_linear_aspect_fill, image_upper_left, Menu,
};
use crate::{
    check_buffer_size, error::Error, icon::Icon, key_handler::KeyHandler, rate::UpdateRate,
    CursorStyle, InputCallback, Key, KeyRepeat, MenuHandle, MouseButton, MouseMode, Result, Scale,
    ScaleMode, UnixMenu, WindowOptions,
};
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, WindowHandle, XlibDisplayHandle, XlibWindowHandle,
};
use std::{
    convert::TryFrom,
    ffi::{c_char, c_int, c_long, c_uchar, c_uint, c_ulong, c_void, CStr, CString},
    mem::MaybeUninit,
    ptr::NonNull,
    time::Duration,
};
use x11_dl::{
    keysym::*,
    xcursor,
    xlib::{
        self, KeyPressMask, KeyReleaseMask, KeySym, Status, XEvent, XIMPreeditNothing,
        XIMStatusNothing, XKeyEvent, XNClientWindow_0, XNFocusWindow_0, XNInputStyle_0,
        XWindowAttributes, XrmDatabase, XIC, XIM,
    },
};

// NOTE: the x11-dl crate does not define Button6 or Button7
const Button6: c_uint = xlib::Button5 + 1;
const Button7: c_uint = xlib::Button5 + 2;

#[repr(C)]
struct MwmHints {
    flags: c_ulong,
    functions: c_ulong,
    decorations: c_ulong,
    input_mode: c_long,
    status: c_ulong,
}

struct DisplayInfo {
    lib: x11_dl::xlib::Xlib,
    display: *mut xlib::Display,
    screen: i32,
    visual: *mut xlib::Visual,
    gc: xlib::GC,
    depth: i32,
    screen_width: usize,
    screen_height: usize,
    _context: xlib::XContext,
    cursor_lib: x11_dl::xcursor::Xcursor,
    cursors: [xlib::Cursor; 8],
    keyb_ext: bool,
    wm_delete_window: xlib::Atom,
}

impl DisplayInfo {
    fn new(transparency: bool) -> Result<DisplayInfo> {
        let mut display = Self::setup(transparency)?;

        display.check_formats()?;
        display.check_extensions()?;
        display.init_cursors();
        display.init_atoms();

        Ok(display)
    }

    fn setup(transparency: bool) -> Result<DisplayInfo> {
        unsafe {
            libc::setlocale(libc::LC_ALL, std::ptr::null()); //needed to make compose key work

            let lib = xlib::Xlib::open()
                .map_err(|e| Error::WindowCreate(format!("failed to load Xlib: {:?}", e)))?;

            if (lib.XInitThreads)() == 0 {
                panic!("failed to init X11 threads");
            }

            let cursor_lib = xcursor::Xcursor::open()
                .map_err(|e| Error::WindowCreate(format!("failed to load XCursor: {:?}", e)))?;

            let display = (lib.XOpenDisplay)(std::ptr::null());

            if display.is_null() {
                return Err(Error::WindowCreate("XOpenDisplay failed".to_owned()));
            }

            let mut supported = 0;
            (lib.XkbSetDetectableAutoRepeat)(display, 1, &mut supported);

            let screen;
            let visual;
            let depth;

            let mut vinfo: xlib::XVisualInfo = std::mem::zeroed();
            if transparency {
                (lib.XMatchVisualInfo)(
                    display,
                    (lib.XDefaultScreen)(display),
                    32,
                    xlib::TrueColor,
                    &mut vinfo as *mut _,
                );
                screen = vinfo.screen;
                visual = vinfo.visual;
                depth = vinfo.depth;
            } else {
                screen = (lib.XDefaultScreen)(display);
                visual = (lib.XDefaultVisual)(display, screen);
                depth = (lib.XDefaultDepth)(display, screen);
            }

            let gc = (lib.XDefaultGC)(display, screen);

            let screen_width = usize::try_from((lib.XDisplayWidth)(display, screen))
                .map_err(|e| Error::WindowCreate(format!("illegal width: {}", e)))?;
            let screen_height = usize::try_from((lib.XDisplayHeight)(display, screen))
                .map_err(|e| Error::WindowCreate(format!("illegal height: {}", e)))?;

            // andrewj: using this instead of XUniqueContext(), as the latter
            // seems to be erroneously feature guarded in the x11_dl crate.
            let context = (lib.XrmUniqueQuark)();

            Ok(DisplayInfo {
                lib,
                display,
                screen,
                visual,
                gc,
                depth,
                screen_width,
                screen_height,
                _context: context,
                cursor_lib,
                // the following are determined later...
                cursors: [0; 8],
                keyb_ext: false,
                wm_delete_window: 0,
            })
        }
    }

    fn check_formats(&mut self) -> Result<()> {
        // We only support 32-bit right now

        let mut conv_depth: i32 = -1;

        unsafe {
            let mut count: i32 = -1;

            let formats = (self.lib.XListPixmapFormats)(self.display, &mut count);

            for i in 0..count {
                let pix_fmt = *formats.offset(i as isize);

                if pix_fmt.depth == self.depth {
                    conv_depth = pix_fmt.bits_per_pixel;
                }
            }
        }

        if conv_depth != 32 {
            Err(Error::WindowCreate("No 32-bit format available".to_owned()))
        } else {
            Ok(())
        }
    }

    fn check_extensions(&mut self) -> Result<()> {
        // require version 1.0
        let mut major: i32 = 1;
        let mut minor: i32 = 0;

        // these values are out-only, and are ignored
        let mut opcode: i32 = 0;
        let mut event: i32 = 0;
        let mut error: i32 = 0;

        unsafe {
            if (self.lib.XkbQueryExtension)(
                self.display,
                &mut opcode,
                &mut event,
                &mut error,
                &mut major,
                &mut minor,
            ) != xlib::False
            {
                self.keyb_ext = true;
            }
        }

        Ok(())
    }

    fn init_cursors(&mut self) {
        self.cursors[0] = self.load_cursor(b"arrow\0");
        self.cursors[1] = self.load_cursor(b"xterm\0");
        self.cursors[2] = self.load_cursor(b"crosshair\0");
        self.cursors[3] = self.load_cursor(b"hand2\0");
        self.cursors[4] = self.load_cursor(b"hand2\0");
        self.cursors[5] = self.load_cursor(b"sb_h_double_arrow\0");
        self.cursors[6] = self.load_cursor(b"sb_v_double_arrow\0");
        self.cursors[7] = self.load_cursor(b"diamond_cross\0");
    }

    fn load_cursor(&mut self, name: &'static [u8]) -> xlib::Cursor {
        unsafe {
            let name = CStr::from_bytes_with_nul_unchecked(name);
            (self.cursor_lib.XcursorLibraryLoadCursor)(self.display, name.as_ptr())
        }
    }

    fn init_atoms(&mut self) {
        self.wm_delete_window = self.intern_atom(b"WM_DELETE_WINDOW\0", false);
    }

    fn intern_atom(&mut self, name: &'static [u8], only_if_exists: bool) -> xlib::Atom {
        unsafe {
            let name = CStr::from_bytes_with_nul_unchecked(name);
            (self.lib.XInternAtom)(
                self.display,
                name.as_ptr(),
                if only_if_exists {
                    xlib::True
                } else {
                    xlib::False
                },
            )
        }
    }
}

impl Drop for DisplayInfo {
    fn drop(&mut self) {
        unsafe {
            (self.lib.XCloseDisplay)(self.display);
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum ProcessEventResult {
    Ok,
    Termination,
}

pub struct Window {
    d: DisplayInfo,

    handle: xlib::Window,
    xim: XIM,
    xic: XIC,

    ximage: *mut xlib::XImage,
    draw_buffer: Vec<u32>,

    width: u32,  // this is the *scaled* size
    height: u32, //

    scale: i32,
    bg_color: u32,
    scale_mode: ScaleMode,

    mouse_x: f32,
    mouse_y: f32,
    scroll_x: f32,
    scroll_y: f32,
    buttons: [u8; 3],
    prev_cursor: CursorStyle,
    active: bool,

    should_close: bool, // received delete window message from X server

    key_handler: KeyHandler,
    update_rate: UpdateRate,
    menu_counter: MenuHandle,
    menus: Vec<UnixMenu>,
}

impl HasWindowHandle for Window {
    fn window_handle(&self) -> std::result::Result<WindowHandle, HandleError> {
        let handle = XlibWindowHandle::new(self.handle);
        let raw_handle = RawWindowHandle::Xlib(handle);
        unsafe { Ok(WindowHandle::borrow_raw(raw_handle)) }
    }
}

impl HasDisplayHandle for Window {
    fn display_handle(&self) -> std::result::Result<DisplayHandle, HandleError> {
        let raw_display = self.d.display as *mut c_void;
        let display = NonNull::new(raw_display);
        let handle = XlibDisplayHandle::new(display, self.d.screen);
        let raw_handle = RawDisplayHandle::Xlib(handle);
        unsafe { Ok(DisplayHandle::borrow_raw(raw_handle)) }
    }
}

impl Window {
    pub fn new(name: &str, width: usize, height: usize, opts: WindowOptions) -> Result<Window> {
        let name = match CString::new(name) {
            Err(_) => {
                println!("Unable to convert {} to c_string", name);
                return Err(Error::WindowCreate("Unable to set correct name".to_owned()));
            }
            Ok(n) => n,
        };

        // FIXME: this DisplayInfo should be a singleton, hence this code
        // is probably no good when using multiple windows.

        let mut d = DisplayInfo::new(opts.transparency)?;

        let scale =
            Self::get_scale_factor(width, height, d.screen_width, d.screen_height, opts.scale);

        let width = width * scale;
        let height = height * scale;

        unsafe {
            let mut attributes: xlib::XSetWindowAttributes = std::mem::zeroed();

            let root = (d.lib.XDefaultRootWindow)(d.display);

            attributes.border_pixel = (d.lib.XBlackPixel)(d.display, d.screen);
            attributes.background_pixel = attributes.border_pixel;
            if opts.transparency {
                attributes.colormap =
                    (d.lib.XCreateColormap)(d.display, root, d.visual, xlib::AllocNone);
            }

            attributes.backing_store = xlib::NotUseful;

            let x = if d.screen_width > width {
                (d.screen_width - width) / 2
            } else {
                0
            };
            let y = if d.screen_height > height {
                (d.screen_height - height) / 2
            } else {
                0
            };

            let handle = (d.lib.XCreateWindow)(
                d.display,
                root,
                x as i32,
                y as i32,
                width as u32,
                height as u32,
                0, /* border_width */
                d.depth,
                xlib::InputOutput as u32, /* class */
                d.visual,
                xlib::CWColormap | xlib::CWBackingStore | xlib::CWBackPixel | xlib::CWBorderPixel,
                &mut attributes,
            );

            let empty_string = b"\0";
            (d.lib.XSetLocaleModifiers)(empty_string.as_ptr() as _);

            let xim = (d.lib.XOpenIM)(
                d.display,
                0 as XrmDatabase,
                std::ptr::null_mut::<c_char>(),
                std::ptr::null_mut::<c_char>(),
            );
            if (xim as usize) == 0 {
                return Err(Error::WindowCreate(
                    "Failed to setup X IM via XOpenIM.".to_owned(),
                ));
            }

            let xn_input_style = CStr::from_bytes_with_nul_unchecked(XNInputStyle_0);
            let xn_client_window = CStr::from_bytes_with_nul_unchecked(XNClientWindow_0);
            let xn_focus_window = CStr::from_bytes_with_nul_unchecked(XNFocusWindow_0);
            let xic = (d.lib.XCreateIC)(
                xim,
                xn_input_style.as_ptr(),
                XIMPreeditNothing | XIMStatusNothing,
                xn_client_window.as_ptr(),
                handle as c_ulong,
                xn_focus_window.as_ptr(),
                handle as c_ulong,
                std::ptr::null_mut::<c_void>(),
            );
            if (xic as usize) == 0 {
                return Err(Error::WindowCreate(
                    "Failed to setup X IC via XCreateIC.".to_owned(),
                ));
            }

            (d.lib.XSetICFocus)(xic);
            (d.lib.XSelectInput)(d.display, handle, KeyPressMask | KeyReleaseMask);

            d.gc = (d.lib.XCreateGC)(d.display, handle, 0, std::ptr::null_mut());

            if handle == 0 {
                return Err(Error::WindowCreate("Unable to open Window".to_owned()));
            }

            Self::set_title_raw(&mut d, handle, &name).map_err(Error::WindowCreate)?;

            (d.lib.XSelectInput)(
                d.display,
                handle,
                xlib::StructureNotifyMask
                    | xlib::KeyPressMask
                    | xlib::KeyReleaseMask
                    | xlib::ButtonPressMask
                    | xlib::ButtonReleaseMask
                    | xlib::FocusChangeMask,
            );

            if !opts.resize || opts.none {
                let mut size_hints: xlib::XSizeHints = std::mem::zeroed();

                size_hints.flags = xlib::PMinSize | xlib::PMaxSize;
                size_hints.min_width = width as i32;
                size_hints.max_width = width as i32;
                size_hints.min_height = height as i32;
                size_hints.max_height = height as i32;

                (d.lib.XSetWMNormalHints)(
                    d.display,
                    handle,
                    &mut size_hints as *mut xlib::XSizeHints,
                );
            }

            if opts.borderless || opts.none {
                let hints_property = (d.lib.XInternAtom)(
                    d.display,
                    "_MOTIF_WM_HINTS\0" as *const _ as *const c_char,
                    0,
                );
                assert!(hints_property != 0);
                let mut hints: MwmHints = std::mem::zeroed();
                hints.flags = 2;
                hints.decorations = 0;
                (d.lib.XChangeProperty)(
                    d.display,
                    handle,
                    hints_property,
                    hints_property,
                    32,
                    xlib::PropModeReplace,
                    &hints as *const _ as *const c_uchar,
                    5,
                );
            }

            (d.lib.XClearWindow)(d.display, handle);
            (d.lib.XMapRaised)(d.display, handle);
            (d.lib.XSetWMProtocols)(d.display, handle, &mut d.wm_delete_window, 1);
            (d.lib.XFlush)(d.display);

            let mut draw_buffer: Vec<u32> = Vec::new();

            let ximage = match Self::alloc_image(&d, width, height, &mut draw_buffer) {
                Some(ximage) => ximage,
                None => {
                    (d.lib.XDestroyWindow)(d.display, handle);
                    return Err(Error::WindowCreate(
                        "Unable to create pixel buffer".to_owned(),
                    ));
                }
            };

            Ok(Window {
                d,
                handle,
                xim,
                xic,
                ximage,
                draw_buffer,
                width: width as u32,
                height: height as u32,
                scale: scale as i32,
                mouse_x: 0.0,
                mouse_y: 0.0,
                scroll_x: 0.0,
                scroll_y: 0.0,
                bg_color: 0,
                scale_mode: opts.scale_mode,
                buttons: [0, 0, 0],
                prev_cursor: CursorStyle::Arrow,
                should_close: false,
                active: false,
                key_handler: KeyHandler::new(),
                update_rate: UpdateRate::new(),
                menu_counter: MenuHandle(0),
                menus: Vec::new(),
            })
        }
    }

    unsafe fn alloc_image(
        d: &DisplayInfo,
        width: usize,
        height: usize,
        draw_buffer: &mut Vec<u32>,
    ) -> Option<*mut xlib::XImage> {
        let bytes_per_line = (width as i32) * 4;

        draw_buffer.resize(width * height, 0);
        let image = (d.lib.XCreateImage)(
            d.display,
            d.visual, /* TODO: this was CopyFromParent in the C code */
            d.depth as u32,
            xlib::ZPixmap,
            0,
            draw_buffer[..].as_mut_ptr() as *mut c_char,
            width as u32,
            height as u32,
            32,
            bytes_per_line,
        );

        if image.is_null() {
            None
        } else {
            Some(image)
        }
    }

    unsafe fn free_image(&mut self) {
        (*self.ximage).data = std::ptr::null_mut();
        (self.d.lib.XDestroyImage)(self.ximage);
        self.ximage = std::ptr::null_mut();
    }

    unsafe fn set_title_raw(
        d: &mut DisplayInfo,
        handle: xlib::Window,
        name: &CStr,
    ) -> std::result::Result<(), String> {
        (d.lib.XStoreName)(d.display, handle, name.as_ptr());
        if let Ok(name_len) = c_int::try_from(name.to_bytes().len()) {
            let net_wm_name = d.intern_atom(b"_NET_WM_NAME\0", false);
            let utf8_string = d.intern_atom(b"UTF8_STRING\0", false);
            (d.lib.XChangeProperty)(
                d.display,
                handle,
                net_wm_name,
                utf8_string,
                8,
                xlib::PropModeReplace,
                name.as_ptr() as *const c_uchar,
                name_len,
            );
            Ok(())
        } else {
            Err("Window name too long".to_owned())
        }
    }

    pub fn set_title(&mut self, title: &str) {
        match CString::new(title) {
            Err(_) => {
                println!("Unable to convert {} to c_string", title);
            }

            Ok(t) => unsafe {
                if let Err(e) = Self::set_title_raw(&mut self.d, self.handle, &t) {
                    println!("Setting window name failed: {}", e);
                }
            },
        };
    }

    pub fn update_with_buffer_stride(
        &mut self,
        buffer: &[u32],
        buf_width: usize,
        buf_height: usize,
        buf_stride: usize,
    ) -> Result<()> {
        check_buffer_size(buffer, buf_width, buf_height, buf_stride)?;

        unsafe { self.raw_blit_buffer(buffer, buf_width, buf_height, buf_stride) };

        self.update();

        Ok(())
    }

    pub fn update(&mut self) {
        self.key_handler.update();

        // clear before processing new events
        self.scroll_x = 0.0;
        self.scroll_y = 0.0;

        unsafe {
            self.raw_get_mouse_pos();
            self.raw_process_events();
        }
    }

    #[inline]
    pub fn set_icon(&mut self, icon: Icon) {
        // XChangeProperty
        let net_string_ptr = b"_NET_WM_ICON\0".as_ptr() as _;
        let cardinal_ptr = b"CARDINAL\0".as_ptr() as _;

        unsafe {
            if let Icon::Buffer(ptr, len) = icon {
                let _ = (self.d.lib.XChangeProperty)(
                    self.d.display,
                    self.handle,
                    (self.d.lib.XInternAtom)(self.d.display, net_string_ptr, xlib::False),
                    (self.d.lib.XInternAtom)(self.d.display, cardinal_ptr, xlib::False),
                    32,
                    xlib::PropModeReplace,
                    ptr as *const u8,
                    len as c_int,
                );
            }
        }
    }

    #[inline]
    pub fn get_window_handle(&self) -> *mut c_void {
        self.handle as *mut c_void
    }

    #[inline]
    pub fn set_background_color(&mut self, bg_color: u32) {
        self.bg_color = bg_color;
    }

    #[inline]
    pub fn set_cursor_visibility(&mut self, visibility: bool) {
        unsafe {
            if visibility {
                (self.d.lib.XDefineCursor)(
                    self.d.display,
                    self.handle,
                    self.d.cursors[self.prev_cursor as usize],
                );
            } else {
                static empty: [c_char; 8] = [0; 8];
                let mut color = std::mem::zeroed();
                let pixmap = (self.d.lib.XCreateBitmapFromData)(
                    self.d.display,
                    self.handle,
                    empty.as_ptr(),
                    8,
                    8,
                );
                let cursor = (self.d.lib.XCreatePixmapCursor)(
                    self.d.display,
                    pixmap,
                    pixmap,
                    &mut color as *mut _,
                    &mut color as *mut _,
                    0,
                    0,
                );
                (self.d.lib.XDefineCursor)(self.d.display, self.handle, cursor);
            }
        }
    }

    #[inline]
    pub fn set_position(&mut self, x: isize, y: isize) {
        unsafe {
            (self.d.lib.XMoveWindow)(self.d.display, self.handle, x as i32, y as i32);
            (self.d.lib.XFlush)(self.d.display);
        }
    }

    #[inline]
    pub fn get_position(&self) -> (isize, isize) {
        let (x, y);
        let (mut nx, mut ny) = (0, 0);

        // Create dummy window for child_return value
        let mut dummy_window: MaybeUninit<Window> = MaybeUninit::uninit();
        let mut attributes: MaybeUninit<XWindowAttributes> = MaybeUninit::uninit();

        unsafe {
            let root = (self.d.lib.XDefaultRootWindow)(self.d.display);

            (self.d.lib.XGetWindowAttributes)(self.d.display, self.handle, attributes.as_mut_ptr());
            x = attributes.assume_init().x;
            y = attributes.assume_init().y;

            (self.d.lib.XTranslateCoordinates)(
                self.d.display,
                self.handle,
                root,
                x,
                y,
                &mut nx,
                &mut ny,
                dummy_window.as_mut_ptr() as *mut c_ulong,
            );
        }

        (nx as isize, ny as isize)
    }

    #[inline]
    pub fn get_size(&self) -> (usize, usize) {
        (self.width as usize, self.height as usize)
    }

    pub fn get_mouse_pos(&self, mode: MouseMode) -> Option<(f32, f32)> {
        let s = self.scale as f32;
        let w = self.width as f32;
        let h = self.height as f32;

        mode.get_pos(self.mouse_x, self.mouse_y, s, w, h)
    }

    pub fn get_unscaled_mouse_pos(&self, mode: MouseMode) -> Option<(f32, f32)> {
        let w = self.width as f32;
        let h = self.height as f32;

        mode.get_pos(self.mouse_x, self.mouse_y, 1.0, w, h)
    }

    pub fn get_mouse_down(&self, button: MouseButton) -> bool {
        match button {
            MouseButton::Left => self.buttons[0] > 0,
            MouseButton::Middle => self.buttons[1] > 0,
            MouseButton::Right => self.buttons[2] > 0,
        }
    }

    pub fn get_scroll_wheel(&self) -> Option<(f32, f32)> {
        if self.scroll_x.abs() > 0.0 || self.scroll_y.abs() > 0.0 {
            Some((self.scroll_x, self.scroll_y))
        } else {
            None
        }
    }

    #[inline]
    pub fn set_cursor_style(&mut self, cursor: CursorStyle) {
        if self.prev_cursor != cursor {
            unsafe {
                (self.d.lib.XDefineCursor)(
                    self.d.display,
                    self.handle,
                    self.d.cursors[cursor as usize],
                );
            }

            self.prev_cursor = cursor;
        }
    }

    #[inline]
    pub fn set_rate(&mut self, rate: Option<Duration>) {
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
    pub fn set_input_callback(&mut self, callback: Box<dyn InputCallback>) {
        self.key_handler.set_input_callback(callback)
    }

    #[inline]
    pub fn is_open(&self) -> bool {
        !self.should_close
    }

    #[inline]
    pub fn is_active(&mut self) -> bool {
        self.active
    }

    fn get_scale_factor(
        width: usize,
        height: usize,
        screen_width: usize,
        screen_height: usize,
        scale: Scale,
    ) -> usize {
        match scale {
            Scale::X1 => 1,
            Scale::X2 => 2,
            Scale::X4 => 4,
            Scale::X8 => 8,
            Scale::X16 => 16,
            Scale::X32 => 32,
            Scale::FitScreen => {
                let mut scale = 1;

                loop {
                    let w = width * (scale + 1);
                    let h = height * (scale + 1);

                    if w >= screen_width || h >= screen_height {
                        break;
                    }

                    scale *= 2;
                }

                if scale >= 32 {
                    32
                } else {
                    scale
                }
            }
        }
    }

    fn next_menu_handle(&mut self) -> MenuHandle {
        let handle = self.menu_counter;
        self.menu_counter.0 += 1;
        handle
    }

    pub fn add_menu(&mut self, menu: &Menu) -> MenuHandle {
        let handle = self.next_menu_handle();
        let mut menu = menu.internal.clone();
        menu.handle = handle;
        self.menus.push(menu);
        handle
    }

    pub fn get_posix_menus(&self) -> Option<&Vec<UnixMenu>> {
        Some(&self.menus)
    }

    pub fn remove_menu(&mut self, handle: MenuHandle) {
        self.menus.retain(|menu| menu.handle != handle);
    }

    pub fn is_menu_pressed(&mut self) -> Option<usize> {
        None
    }

    ////////////////////////////////////

    unsafe fn raw_blit_buffer(
        &mut self,
        buffer: &[u32],
        buf_width: usize,
        buf_height: usize,
        buf_stride: usize,
    ) {
        match self.scale_mode {
            ScaleMode::Stretch => {
                image_resize_linear(
                    self.draw_buffer.as_mut_ptr(),
                    self.width,
                    self.height,
                    buffer.as_ptr(),
                    buf_width as u32,
                    buf_height as u32,
                    buf_stride as u32,
                );
            }

            ScaleMode::AspectRatioStretch => {
                image_resize_linear_aspect_fill(
                    self.draw_buffer.as_mut_ptr(),
                    self.width,
                    self.height,
                    buffer.as_ptr(),
                    buf_width as u32,
                    buf_height as u32,
                    buf_stride as u32,
                    self.bg_color,
                );
            }

            ScaleMode::Center => {
                image_center(
                    self.draw_buffer.as_mut_ptr(),
                    self.width,
                    self.height,
                    buffer.as_ptr(),
                    buf_width as u32,
                    buf_height as u32,
                    buf_stride as u32,
                    self.bg_color,
                );
            }

            ScaleMode::UpperLeft => {
                image_upper_left(
                    self.draw_buffer.as_mut_ptr(),
                    self.width,
                    self.height,
                    buffer.as_ptr(),
                    buf_width as u32,
                    buf_height as u32,
                    buf_stride as u32,
                    self.bg_color,
                );
            }
        }

        (self.d.lib.XPutImage)(
            self.d.display,
            self.handle,
            self.d.gc,
            self.ximage,
            0,
            0,
            0,
            0,
            self.width,
            self.height,
        );
        (self.d.lib.XFlush)(self.d.display);
    }

    unsafe fn raw_get_mouse_pos(&mut self) {
        let mut root: xlib::Window = 0;
        let mut root_x: i32 = 0;
        let mut root_y: i32 = 0;

        let mut child: xlib::Window = 0;
        let mut child_x: i32 = 0;
        let mut child_y: i32 = 0;

        let mut mask: u32 = 0;

        if (self.d.lib.XQueryPointer)(
            self.d.display,
            self.handle,
            &mut root,
            &mut child,
            &mut root_x,
            &mut root_y,
            &mut child_x,
            &mut child_y,
            &mut mask,
        ) != xlib::False
        {
            self.mouse_x = child_x as f32;
            self.mouse_y = child_y as f32;
        }
    }

    unsafe fn raw_process_events(&mut self) {
        let count = (self.d.lib.XPending)(self.d.display);

        for _ in 0..count {
            let mut event: xlib::XEvent = std::mem::zeroed();

            (self.d.lib.XNextEvent)(self.d.display, &mut event);

            //skip any events that need to get eaten by X to do compose key, e.g. if the user types compose key + a + ' then all of these events need to get eaten and processed in xlib
            //XFilterEvent will do the processing for these cases, and returns whether or not it handled an event
            if (self.d.lib.XFilterEvent)(&mut event as *mut XEvent, 0) != 0 {
                continue;
            }

            // Don't process any more messages if we hit a termination event
            if self.raw_process_one_event(event) == ProcessEventResult::Termination {
                return;
            }
        }
    }

    unsafe fn raw_process_one_event(&mut self, mut ev: xlib::XEvent) -> ProcessEventResult {
        // FIXME: we cannot handle multiple windows here!
        if ev.any.window != self.handle {
            return ProcessEventResult::Ok;
        }

        match ev.type_ {
            xlib::ClientMessage => {
                // TODO : check for message_type == wm_protocols, as per x11-rs example
                if ev.client_message.format == 32 /* i.e. longs */ &&
                   ev.client_message.data.get_long(0) as xlib::Atom == self.d.wm_delete_window
                {
                    self.should_close = true;
                    return ProcessEventResult::Termination;
                }
            }

            xlib::KeyPress => {
                self.process_key(ev, true /* is_down */);
                self.emit_code_point_chars_to_callback(&mut ev.key);
            }

            xlib::KeyRelease => {
                /* After XkbSetDetectableAutoRepeat it looks like we don't
                   have to try to fix the x11 repeat issue this way, but code left as reference in one commit)
                let mut is_retriggered = false;
                let t = (self.d.lib.XEventsQueued)(self.d.display, 1 /*QueuedAfterReading*/);

                if t != 0 {
                    let mut nev: xlib::XEvent = mem::zeroed();
                    (self.d.lib.XPeekEvent)(self.d.display, &mut nev);

                    if nev.type_ == xlib::KeyPress
                        && nev.key.time == ev.key.time
                        && nev.key.keycode == ev.key.keycode
                    {
                        is_retriggered = true;
                        (self.d.lib.XNextEvent)(self.d.display, &mut ev);
                    }
                }

                if is_retriggered {
                    println!("retrigged");
                }
                 */

                self.process_key(ev, false /* is_down */);
            }

            xlib::ButtonPress => {
                self.process_button(ev, true /* is_down */);
            }

            xlib::ButtonRelease => {
                self.process_button(ev, false /* is_down */);
            }

            xlib::ConfigureNotify => {
                // TODO : pass this onto the application
                self.width = ev.configure.width as u32;
                self.height = ev.configure.height as u32;
                self.free_image();
                self.ximage = Self::alloc_image(
                    &self.d,
                    self.width as usize,
                    self.height as usize,
                    &mut self.draw_buffer,
                )
                .expect("todo");
            }
            xlib::FocusOut => {
                self.active = false;
            }
            xlib::FocusIn => {
                self.active = true;
            }

            _ => {}
        }

        ProcessEventResult::Ok
    }

    fn process_key(&mut self, mut ev: xlib::XEvent, is_down: bool) {
        // NOTE: need "mut" on ev due to dumbness in the X API

        // handle special keys...

        if self.d.keyb_ext {
            let sym: xlib::KeySym = unsafe {
                (self.d.lib.XkbKeycodeToKeysym)(
                    self.d.display,
                    ev.key.keycode as xlib::KeyCode,
                    0, /* group */
                    1, /* level */
                )
            };

            match sym as u32 {
                XK_KP_0 | XK_KP_1 | XK_KP_2 | XK_KP_3 | XK_KP_4 | XK_KP_5 | XK_KP_6 | XK_KP_7
                | XK_KP_8 | XK_KP_9 | XK_KP_Separator | XK_KP_Decimal | XK_KP_Equal
                | XK_KP_Enter => {
                    self.update_key_state(sym, is_down);
                    return;
                }

                _ => {}
            }
        }

        // normal keys...

        let sym: xlib::KeySym = unsafe {
            (self.d.lib.XLookupKeysym)(&mut ev.key, 0 /* index */)
        };

        if sym == xlib::NoSymbol as xlib::KeySym {
            return;
        }

        self.update_key_state(sym, is_down);
    }

    fn emit_code_point_chars_to_callback(&mut self, event: &mut XKeyEvent) {
        const BUFFER_SIZE: usize = 32;

        if let Some(callback) = &mut self.key_handler.key_callback {
            let mut buff: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];
            let str = unsafe {
                let mut keysym: KeySym = std::mem::zeroed();
                let mut status: Status = 0;
                let length_in_bytes = (self.d.lib.Xutf8LookupString)(
                    self.xic,
                    event as *mut XKeyEvent,
                    buff.as_mut_ptr() as *mut c_char,
                    (BUFFER_SIZE - 1) as c_int,
                    (&mut keysym) as *mut KeySym,
                    (&mut status) as *mut Status,
                );
                &buff[0..(length_in_bytes as usize + 1)]
            };

            if let Ok(cstr) = CStr::from_bytes_with_nul(str) {
                if let Ok(str) = cstr.to_str() {
                    for c in str.chars() {
                        callback.add_char(c as u32);
                    }
                }
            }
        }
    }

    unsafe fn process_button(&mut self, ev: xlib::XEvent, is_down: bool) {
        match ev.button.button {
            xlib::Button1 => {
                self.buttons[0] = if is_down { 1 } else { 0 };
                return;
            }
            xlib::Button2 => {
                self.buttons[1] = if is_down { 1 } else { 0 };
                return;
            }
            xlib::Button3 => {
                self.buttons[2] = if is_down { 1 } else { 0 };
                return;
            }

            _ => {}
        }

        // in X, the mouse wheel is usually mapped to Button4/5

        let scroll: (i32, i32) = match ev.button.button {
            xlib::Button4 => (0, 10),
            xlib::Button5 => (0, -10),

            Button6 => (10, 0),
            Button7 => (-10, 0),

            _ => {
                return;
            }
        };

        self.scroll_x += scroll.0 as f32 * 0.1;
        self.scroll_y += scroll.1 as f32 * 0.1;
    }

    fn update_key_state(&mut self, sym: xlib::KeySym, is_down: bool) {
        if sym > u32::max_value() as xlib::KeySym {
            return;
        }

        let key = match sym as u32 {
            XK_0 => Key::Key0,
            XK_1 => Key::Key1,
            XK_2 => Key::Key2,
            XK_3 => Key::Key3,
            XK_4 => Key::Key4,
            XK_5 => Key::Key5,
            XK_6 => Key::Key6,
            XK_7 => Key::Key7,
            XK_8 => Key::Key8,
            XK_9 => Key::Key9,

            XK_a => Key::A,
            XK_b => Key::B,
            XK_c => Key::C,
            XK_d => Key::D,
            XK_e => Key::E,
            XK_f => Key::F,
            XK_g => Key::G,
            XK_h => Key::H,
            XK_i => Key::I,
            XK_j => Key::J,
            XK_k => Key::K,
            XK_l => Key::L,
            XK_m => Key::M,
            XK_n => Key::N,
            XK_o => Key::O,
            XK_p => Key::P,
            XK_q => Key::Q,
            XK_r => Key::R,
            XK_s => Key::S,
            XK_t => Key::T,
            XK_u => Key::U,
            XK_v => Key::V,
            XK_w => Key::W,
            XK_x => Key::X,
            XK_y => Key::Y,
            XK_z => Key::Z,

            XK_apostrophe => Key::Apostrophe,
            XK_grave => Key::Backquote,
            XK_backslash => Key::Backslash,
            XK_comma => Key::Comma,
            XK_equal => Key::Equal,
            XK_bracketleft => Key::LeftBracket,
            XK_minus => Key::Minus,
            XK_period => Key::Period,
            XK_bracketright => Key::RightBracket,
            XK_semicolon => Key::Semicolon,
            XK_slash => Key::Slash,
            XK_space => Key::Space,

            XK_F1 => Key::F1,
            XK_F2 => Key::F2,
            XK_F3 => Key::F3,
            XK_F4 => Key::F4,
            XK_F5 => Key::F5,
            XK_F6 => Key::F6,
            XK_F7 => Key::F7,
            XK_F8 => Key::F8,
            XK_F9 => Key::F9,
            XK_F10 => Key::F10,
            XK_F11 => Key::F11,
            XK_F12 => Key::F12,

            XK_Down => Key::Down,
            XK_Left => Key::Left,
            XK_Right => Key::Right,
            XK_Up => Key::Up,
            XK_Escape => Key::Escape,
            XK_BackSpace => Key::Backspace,
            XK_Delete => Key::Delete,
            XK_End => Key::End,
            XK_Return => Key::Enter,
            XK_Home => Key::Home,
            XK_Insert => Key::Insert,
            XK_Menu => Key::Menu,
            XK_Page_Down => Key::PageDown,
            XK_Page_Up => Key::PageUp,
            XK_Pause => Key::Pause,
            XK_Tab => Key::Tab,
            XK_Num_Lock => Key::NumLock,
            XK_Caps_Lock => Key::CapsLock,
            XK_Scroll_Lock => Key::ScrollLock,
            XK_Shift_L => Key::LeftShift,
            XK_Shift_R => Key::RightShift,
            XK_Alt_L => Key::LeftAlt,
            XK_Alt_R => Key::RightAlt,
            XK_Control_L => Key::LeftCtrl,
            XK_Control_R => Key::RightCtrl,
            XK_Super_L => Key::LeftSuper,
            XK_Super_R => Key::RightSuper,

            XK_KP_0 => Key::NumPad0,
            XK_KP_1 => Key::NumPad1,
            XK_KP_2 => Key::NumPad2,
            XK_KP_3 => Key::NumPad3,
            XK_KP_4 => Key::NumPad4,
            XK_KP_5 => Key::NumPad5,
            XK_KP_6 => Key::NumPad6,
            XK_KP_7 => Key::NumPad7,
            XK_KP_8 => Key::NumPad8,
            XK_KP_9 => Key::NumPad9,
            XK_KP_Decimal => Key::NumPadDot,
            XK_KP_Divide => Key::NumPadSlash,
            XK_KP_Multiply => Key::NumPadAsterisk,
            XK_KP_Subtract => Key::NumPadMinus,
            XK_KP_Add => Key::NumPadPlus,
            XK_KP_Enter => Key::NumPadEnter,

            _ => {
                // ignore other keys
                return;
            }
        };

        self.key_handler.set_key_state(key, is_down);
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            self.free_image();

            // TODO  [ andrewj: right now DisplayInfo is not shared, so doing this is
            //                  probably pointless ]
            // XSaveContext(s_display, info->window, s_context, (XPointer)0);

            (self.d.lib.XDestroyIC)(self.xic);
            (self.d.lib.XCloseIM)(self.xim);
            (self.d.lib.XDestroyWindow)(self.d.display, self.handle);
        }
    }
}
