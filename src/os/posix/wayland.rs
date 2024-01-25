use std::{
    cell::RefCell,
    ffi::c_void,
    fs::File,
    io::{Seek, SeekFrom, Write},
    os::unix::io::{AsRawFd, RawFd},
    ptr::NonNull,
    rc::Rc,
    sync::mpsc,
    time::Duration,
};

use super::common::{
    image_center, image_resize_linear, image_resize_linear_aspect_fill, image_upper_left, Menu,
};
use crate::{
    check_buffer_size, key_handler::KeyHandler, rate::UpdateRate, CursorStyle, Error,
    InputCallback, Key, KeyRepeat, MenuHandle, MouseButton, MouseMode, Result, Scale, ScaleMode,
    UnixMenu, WindowOptions,
};
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle, WindowHandle,
};
use wayland_client::{
    protocol::{
        wl_buffer::WlBuffer,
        wl_compositor::WlCompositor,
        wl_display::WlDisplay,
        wl_keyboard::{self, KeymapFormat, WlKeyboard},
        wl_pointer::{self, WlPointer},
        wl_seat::WlSeat,
        wl_shm::{Format, WlShm},
        wl_shm_pool::WlShmPool,
        wl_surface::WlSurface,
    },
    Attached, Display, EventQueue, GlobalManager, Main,
};
use wayland_protocols::{
    unstable::xdg_decoration::v1::client::zxdg_decoration_manager_v1::ZxdgDecorationManagerV1,
    xdg_shell::client::{
        xdg_surface::XdgSurface, xdg_toplevel::XdgToplevel, xdg_wm_base::XdgWmBase,
    },
};

use super::xkb_ffi;
#[cfg(feature = "dlopen")]
use super::xkb_ffi::XKBCOMMON_HANDLE as XKBH;
#[cfg(not(feature = "dlopen"))]
use super::xkb_ffi::*;

const KEY_XKB_OFFSET: u32 = 8;
const KEY_MOUSE_BTN1: u32 = 272;
const KEY_MOUSE_BTN2: u32 = 273;
const KEY_MOUSE_BTN3: u32 = 274;

type ToplevelResolution = Rc<RefCell<Option<(i32, i32)>>>;
type ToplevelClosed = Rc<RefCell<bool>>;

struct Buffer {
    fd: File,
    pool: Main<WlShmPool>,
    pool_size: i32,
    buffer: Main<WlBuffer>,
    buffer_state: Rc<RefCell<bool>>,
    fb_size: (i32, i32),
}

struct BufferPool {
    pool: Vec<Buffer>,
    shm: Main<WlShm>,
    format: Format,
}

impl BufferPool {
    fn new(shm: Main<WlShm>, format: Format) -> Self {
        Self {
            pool: Vec::new(),
            shm,
            format,
        }
    }

    fn create_shm_buffer(
        shm_pool: &Main<WlShmPool>,
        size: (i32, i32),
        format: Format,
    ) -> (Main<WlBuffer>, Rc<RefCell<bool>>) {
        let buf = shm_pool.create_buffer(
            0,
            size.0,
            size.1,
            size.0 * std::mem::size_of::<u32>() as i32,
            format,
        );

        // Whether or not the buffer has been released by the compositor
        let buf_released = Rc::new(RefCell::new(false));
        let buf_released_clone = buf_released.clone();

        buf.quick_assign(move |_, event, _| {
            use wayland_client::protocol::wl_buffer::Event;

            if let Event::Release = event {
                *buf_released_clone.borrow_mut() = true;
            }
        });

        (buf, buf_released)
    }

    fn get_buffer(&mut self, size: (i32, i32)) -> std::io::Result<(File, &Main<WlBuffer>)> {
        let pos = self.pool.iter().rposition(|e| *e.buffer_state.borrow());
        let size_bytes = size.0 * size.1 * std::mem::size_of::<u32>() as i32;

        // If possible, take an older shm_pool and create a new buffer in it
        if let Some(idx) = pos {
            // Shm_pool not allowed to be truncated
            if size_bytes > self.pool[idx].pool_size {
                self.pool[idx].pool.resize(size_bytes);
                self.pool[idx].pool_size = size_bytes;
            }

            // Different buffer size
            if self.pool[idx].fb_size != size {
                let new_buffer = Self::create_shm_buffer(&self.pool[idx].pool, size, self.format);
                let old_buffer = std::mem::replace(&mut self.pool[idx].buffer, new_buffer.0);
                old_buffer.destroy();
                self.pool[idx].fb_size = size;
            }

            Ok((self.pool[idx].fd.try_clone()?, &self.pool[idx].buffer))
        } else {
            let tempfile = tempfile::tempfile()?;
            let shm_pool = self.shm.create_pool(
                tempfile.as_raw_fd(),
                size.0 * size.1 * std::mem::size_of::<u32>() as i32,
            );
            let buffer = Self::create_shm_buffer(&shm_pool, size, self.format);

            self.pool.push(Buffer {
                fd: tempfile,
                pool: shm_pool,
                pool_size: size_bytes,
                buffer: buffer.0,
                buffer_state: buffer.1,
                fb_size: size,
            });

            Ok((
                self.pool[self.pool.len() - 1].fd.try_clone()?,
                &self.pool[self.pool.len() - 1].buffer,
            ))
        }
    }
}

struct DisplayInfo {
    attached_display: Attached<WlDisplay>,
    surface: Main<WlSurface>,
    xdg_surface: Main<XdgSurface>,
    toplevel: Main<XdgToplevel>,
    event_queue: EventQueue,
    xdg_config: Rc<RefCell<Option<u32>>>,
    cursor: wayland_cursor::CursorTheme,
    cursor_surface: Main<WlSurface>,
    _display: Display,
    buf_pool: BufferPool,
}

impl DisplayInfo {
    /// Accepts the size of the surface to be created, whether or not the alpha channel will be
    /// rendered, and whether or not server-side decorations will be used.
    fn new(size: (i32, i32), alpha: bool, decorate: bool) -> Result<(Self, WaylandInput)> {
        // Get the wayland display
        let display = Display::connect_to_env().map_err(|e| {
            Error::WindowCreate(format!("Failed to connect to the Wayland display: {:?}", e))
        })?;
        let mut event_queue = display.create_event_queue();

        // Access internal WlDisplay with a token
        let attached_display = (*display).clone().attach(event_queue.token());
        let globals = GlobalManager::new(&attached_display);

        // Wait for the Wayland server to process all events
        event_queue
            .sync_roundtrip(&mut (), |_, _, _| unreachable!())
            .map_err(|e| Error::WindowCreate(format!("Roundtrip failed: {:?}", e)))?;

        // Version 5 is required for scroll events
        let seat = globals
            .instantiate_exact::<WlSeat>(5)
            .map_err(|e| Error::WindowCreate(format!("Failed to retrieve the WlSeat: {:?}", e)))?;

        let input_devices = WaylandInput::new(&seat);
        let compositor = globals.instantiate_exact::<WlCompositor>(4).map_err(|e| {
            Error::WindowCreate(format!("Failed to retrieve the compositor: {:?}", e))
        })?;
        let shm = globals
            .instantiate_exact::<WlShm>(1)
            .map_err(|e| Error::WindowCreate(format!("Failed to create shared memory: {:?}", e)))?;

        let surface = compositor.create_surface();

        // Specify format
        let format = if alpha {
            Format::Argb8888
        } else {
            Format::Xrgb8888
        };

        // Retrive shm buffer for writing
        let mut buf_pool = BufferPool::new(shm.clone(), format);
        let (mut tempfile, buffer) = buf_pool
            .get_buffer(size)
            .map_err(|e| Error::WindowCreate(format!("Failed to retrieve Buffer: {:?}", e)))?;

        // Add a black canvas into the framebuffer
        let frame: Vec<u32> = vec![0xFF00_0000; (size.0 * size.1) as usize];
        let slice = unsafe {
            std::slice::from_raw_parts(
                frame.as_ptr() as *const u8,
                frame.len() * std::mem::size_of::<u32>(),
            )
        };
        tempfile
            .write_all(slice)
            .map_err(|e| Error::WindowCreate(format!("Io Error: {:?}", e)))?;
        tempfile
            .flush()
            .map_err(|e| Error::WindowCreate(format!("Io Error: {:?}", e)))?;

        let xdg_wm_base = globals.instantiate_exact::<XdgWmBase>(1).map_err(|e| {
            Error::WindowCreate(format!("Failed to retrieve the XdgWmBase: {:?}", e))
        })?;

        // Reply to ping event
        xdg_wm_base.quick_assign(|xdg_wm_base, event, _| {
            use wayland_protocols::xdg_shell::client::xdg_wm_base::Event;

            if let Event::Ping { serial } = event {
                xdg_wm_base.pong(serial);
            }
        });

        let xdg_surface = xdg_wm_base.get_xdg_surface(&surface);
        let surface_clone = surface.clone();

        // Handle configure event
        xdg_surface.quick_assign(move |xdg_surface, event, _| {
            use wayland_protocols::xdg_shell::client::xdg_surface::Event;

            if let Event::Configure { serial } = event {
                xdg_surface.ack_configure(serial);
                surface_clone.commit();
            }
        });

        // Assign the toplevel role and commit
        let xdg_toplevel = xdg_surface.get_toplevel();

        if decorate {
            if let Ok(decorations) = globals
                .instantiate_exact::<ZxdgDecorationManagerV1>(1)
                .map_err(|e| println!("Failed to create server-side surface decoration: {:?}", e))
            {
                decorations.get_toplevel_decoration(&xdg_toplevel);
                decorations.destroy();
            }
        }

        surface.commit();
        event_queue
            .sync_roundtrip(&mut (), |_, _, _| {})
            .map_err(|e| Error::WindowCreate(format!("Roundtrip failed: {:?}", e)))?;

        // Give the buffer to the surface and commit
        surface.attach(Some(buffer), 0, 0);
        surface.damage(0, 0, i32::max_value(), i32::max_value());
        surface.commit();

        let xdg_config = Rc::new(RefCell::new(None));
        let xdg_config_clone = xdg_config.clone();

        xdg_surface.quick_assign(move |_xdg_surface, event, _| {
            use wayland_protocols::xdg_shell::client::xdg_surface::Event;

            // Acknowledge only the last configure
            if let Event::Configure { serial } = event {
                *xdg_config_clone.borrow_mut() = Some(serial);
            }
        });

        let cursor = wayland_cursor::CursorTheme::load(16, &shm);
        let cursor_surface = compositor.create_surface();

        Ok((
            Self {
                _display: display,
                attached_display,
                surface,
                xdg_surface,
                toplevel: xdg_toplevel,
                event_queue,
                xdg_config,
                cursor,
                cursor_surface,
                buf_pool,
            },
            input_devices,
        ))
    }

    #[inline]
    fn set_geometry(&self, pos: (i32, i32), size: (i32, i32)) {
        self.xdg_surface
            .set_window_geometry(pos.0, pos.1, size.0, size.1);
    }

    #[inline]
    fn set_title(&self, title: &str) {
        self.toplevel.set_title(title.to_owned());
    }

    #[inline]
    fn set_no_resize(&self, size: (i32, i32)) {
        self.toplevel.set_max_size(size.0, size.1);
        self.toplevel.set_min_size(size.0, size.1);
    }

    // Sets a specific cursor style
    #[inline]
    fn update_cursor(&mut self, cursor: &str) -> std::result::Result<(), ()> {
        let cursor = self.cursor.get_cursor(cursor);
        if let Some(cursor) = cursor {
            let img = &cursor[0];
            self.cursor_surface.attach(Some(img), 0, 0);
            self.cursor_surface.damage(0, 0, 32, 32);
            self.cursor_surface.commit();
        }
        Ok(())
    }

    // Resizes when buffer is bigger or less
    fn update_framebuffer(&mut self, buffer: &[u32], size: (i32, i32)) -> std::io::Result<()> {
        let (mut fd, buf) = self.buf_pool.get_buffer(size)?;

        fd.seek(SeekFrom::Start(0))?;

        let slice = unsafe {
            std::slice::from_raw_parts(
                buffer.as_ptr() as *const u8,
                buffer.len() * std::mem::size_of::<u32>(),
            )
        };

        fd.write_all(slice)?;
        fd.flush()?;

        // Acknowledge the last configure event
        if let Some(serial) = (*self.xdg_config.borrow_mut()).take() {
            self.xdg_surface.ack_configure(serial);
        }

        self.surface.attach(Some(buf), 0, 0);
        self.surface
            .damage(0, 0, i32::max_value(), i32::max_value());
        self.surface.commit();

        Ok(())
    }

    fn get_toplevel_info(&self) -> (ToplevelResolution, ToplevelClosed) {
        let resolution = Rc::new(RefCell::new(None));
        let closed = Rc::new(RefCell::new(false));

        let resolution_clone = resolution.clone();
        let closed_clone = closed.clone();

        self.toplevel.quick_assign(move |_, event, _| {
            use wayland_protocols::xdg_shell::client::xdg_toplevel::Event;

            if let Event::Configure { width, height, .. } = event {
                *resolution_clone.borrow_mut() = Some((width, height));
            } else if let Event::Close = event {
                *closed_clone.borrow_mut() = true;
            }
        });

        (resolution, closed)
    }
}

struct WaylandInput {
    kb_events: mpsc::Receiver<wl_keyboard::Event>,
    pt_events: mpsc::Receiver<wl_pointer::Event>,
    _keyboard: Main<WlKeyboard>,
    pointer: Main<WlPointer>,
}

impl WaylandInput {
    fn new(seat: &Main<WlSeat>) -> Self {
        let (keyboard, pointer) = (seat.get_keyboard(), seat.get_pointer());
        let (kb_sender, kb_receiver) = mpsc::sync_channel(1024);

        keyboard.quick_assign(move |_, event, _| {
            kb_sender.send(event).unwrap();
        });

        let (pt_sender, pt_receiver) = mpsc::sync_channel(1024);

        pointer.quick_assign(move |_, event, _| {
            pt_sender.send(event).unwrap();
        });

        Self {
            kb_events: kb_receiver,
            pt_events: pt_receiver,
            _keyboard: keyboard,
            pointer,
        }
    }

    #[inline]
    fn get_pointer(&self) -> &Main<WlPointer> {
        &self.pointer
    }

    #[inline]
    fn iter_keyboard_events(&self) -> mpsc::TryIter<wl_keyboard::Event> {
        self.kb_events.try_iter()
    }

    #[inline]
    fn iter_pointer_events(&self) -> mpsc::TryIter<wl_pointer::Event> {
        self.pt_events.try_iter()
    }
}

pub struct Window {
    display: DisplayInfo,

    width: i32,
    height: i32,

    scale: i32,
    bg_color: u32,
    scale_mode: ScaleMode,

    mouse_x: f32,
    mouse_y: f32,
    scroll_x: f32,
    scroll_y: f32,
    buttons: [bool; 8], // Linux kernel defines 8 mouse buttons
    prev_cursor: CursorStyle,

    should_close: bool,
    active: bool,

    key_handler: KeyHandler,

    xkb_context: *mut xkb_ffi::xkb_context,
    xkb_keymap: *mut xkb_ffi::xkb_keymap,
    xkb_state: *mut xkb_ffi::xkb_state,

    update_rate: UpdateRate,
    menu_counter: MenuHandle,
    menus: Vec<UnixMenu>,
    input: WaylandInput,
    resizable: bool,
    // Temporary buffer
    buffer: Vec<u32>,
    // Resolution, closed
    toplevel_info: (ToplevelResolution, ToplevelClosed),
    pointer_visibility: bool,
}

impl Window {
    pub fn new(name: &str, width: usize, height: usize, opts: WindowOptions) -> Result<Self> {
        let scale: i32 = match opts.scale {
            // Relies on the fact that this is done by the server
            // https://docs.rs/winit/0.22.0/winit/dpi/index.html#how-is-the-scale-factor-calculated
            Scale::FitScreen => 1,

            Scale::X1 => 1,
            Scale::X2 => 2,
            Scale::X4 => 4,
            Scale::X8 => 8,
            Scale::X16 => 16,
            Scale::X32 => 32,
        };

        let (display, input) = DisplayInfo::new(
            (width as i32 * scale, height as i32 * scale),
            opts.transparency,
            !opts.borderless || opts.none,
        )?;

        if opts.title {
            display.set_title(name);
        }
        if !opts.resize || opts.none {
            display.set_no_resize((width as i32 * scale, height as i32 * scale));
        }

        let (resolution, closed) = display.get_toplevel_info();

        #[cfg(feature = "dlopen")]
        {
            if xkb_ffi::XKBCOMMON_OPTION.as_ref().is_none() {
                return Err(Error::WindowCreate(
                    "Could not load xkbcommon shared library.".to_owned(),
                ));
            }
        }
        let context = unsafe {
            ffi_dispatch!(
                XKBH,
                xkb_context_new,
                xkb_ffi::xkb_context_flags::XKB_CONTEXT_NO_FLAGS
            )
        };
        if context.is_null() {
            return Err(Error::WindowCreate(
                "Could not create xkb context.".to_owned(),
            ));
        }

        Ok(Self {
            display,

            width: width as i32 * scale,
            height: height as i32 * scale,

            scale,
            bg_color: 0,
            scale_mode: opts.scale_mode,

            mouse_x: 0.,
            mouse_y: 0.,
            scroll_x: 0.,
            scroll_y: 0.,
            buttons: [false; 8],
            prev_cursor: CursorStyle::Arrow,

            should_close: false,
            active: false,

            key_handler: KeyHandler::new(),

            xkb_context: context,
            xkb_keymap: std::ptr::null_mut(),
            xkb_state: std::ptr::null_mut(),

            update_rate: UpdateRate::new(),
            menu_counter: MenuHandle(0),
            menus: Vec::new(),
            input,
            resizable: opts.resize && !opts.none,
            buffer: Vec::with_capacity(width * height * scale as usize * scale as usize),
            toplevel_info: (resolution, closed),
            pointer_visibility: true,
        })
    }

    #[inline]
    pub fn set_title(&mut self, title: &str) {
        self.display.set_title(title);
    }

    #[inline]
    pub fn set_background_color(&mut self, bg_color: u32) {
        self.bg_color = bg_color;
    }

    #[inline]
    pub fn set_cursor_visibility(&mut self, visibility: bool) {
        self.pointer_visibility = visibility;
    }

    #[inline]
    pub fn is_open(&self) -> bool {
        !self.should_close
    }

    #[inline]
    pub fn get_window_handle(&self) -> *mut c_void {
        self.display.surface.as_ref().c_ptr() as *mut c_void
    }

    #[inline]
    pub fn get_size(&self) -> (usize, usize) {
        (self.width as usize, self.height as usize)
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
    pub fn get_mouse_pos(&self, mode: MouseMode) -> Option<(f32, f32)> {
        mode.get_pos(
            self.mouse_x,
            self.mouse_y,
            self.scale as f32,
            self.width as f32,
            self.height as f32,
        )
    }

    #[inline]
    pub fn get_mouse_down(&self, button: MouseButton) -> bool {
        match button {
            MouseButton::Left => self.buttons[0],
            MouseButton::Right => self.buttons[1],
            MouseButton::Middle => self.buttons[2],
        }
    }

    #[inline]
    pub fn get_unscaled_mouse_pos(&self, mode: MouseMode) -> Option<(f32, f32)> {
        mode.get_pos(
            self.mouse_x,
            self.mouse_y,
            1.0,
            self.width as f32,
            self.height as f32,
        )
    }

    #[inline]
    pub fn get_scroll_wheel(&self) -> Option<(f32, f32)> {
        if self.scroll_x.abs() > 0.0 || self.scroll_y.abs() > 0.0 {
            Some((self.scroll_x, self.scroll_y))
        } else {
            None
        }
    }

    #[inline]
    pub fn is_key_down(&self, key: Key) -> bool {
        self.key_handler.is_key_down(key)
    }

    #[inline]
    pub fn set_position(&mut self, x: isize, y: isize) {
        self.display
            .set_geometry((x as i32, y as i32), (self.width, self.height));
    }

    #[inline]
    pub fn get_position(&self) -> (isize, isize) {
        let (x, y) = (0, 0);
        // todo!("get_position");

        (x as isize, y as isize)
    }

    #[inline]
    pub fn set_rate(&mut self, rate: Option<Duration>) {
        self.update_rate.set_rate(rate);
    }

    #[inline]
    pub fn set_key_repeat_rate(&mut self, rate: f32) {
        self.key_handler.set_key_repeat_delay(rate);
    }

    #[inline]
    pub fn set_key_repeat_delay(&mut self, delay: f32) {
        self.key_handler.set_key_repeat_delay(delay);
    }

    #[inline]
    pub fn set_input_callback(&mut self, callback: Box<dyn InputCallback>) {
        self.key_handler.set_input_callback(callback);
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
    pub fn update_rate(&mut self) {
        self.update_rate.update();
    }

    #[inline]
    pub fn is_active(&self) -> bool {
        self.active
    }

    #[inline]
    fn next_menu_handle(&mut self) -> MenuHandle {
        let handle = self.menu_counter;
        self.menu_counter.0 += 1;
        handle
    }

    #[inline]
    pub fn add_menu(&mut self, menu: &Menu) -> MenuHandle {
        let handle = self.next_menu_handle();
        let mut menu = menu.internal.clone();
        menu.handle = handle;
        self.menus.push(menu);
        handle
    }

    #[inline]
    pub fn get_posix_menus(&self) -> Option<&Vec<UnixMenu>> {
        //FIXME
        unimplemented!()
    }

    #[inline]
    pub fn remove_menu(&mut self, handle: MenuHandle) {
        self.menus.retain(|menu| menu.handle != handle);
    }

    #[inline]
    pub fn is_menu_pressed(&mut self) -> Option<usize> {
        //FIXME
        unimplemented!()
    }

    fn try_dispatch_events(&mut self) {
        // as seen in https://docs.rs/wayland-client/0.28/wayland_client/struct.EventQueue.html
        if let Err(e) = self.display.event_queue.display().flush() {
            if e.kind() != std::io::ErrorKind::WouldBlock {
                eprintln!("Error while trying to flush the wayland socket: {:?}", e);
            }
        }

        if let Some(guard) = self.display.event_queue.prepare_read() {
            if let Err(e) = guard.read_events() {
                if e.kind() != std::io::ErrorKind::WouldBlock {
                    eprintln!(
                        "Error while trying to read from the wayland socket: {:?}",
                        e
                    );
                }
            }
        }

        self.display
            .event_queue
            .dispatch_pending(&mut (), |_, _, _| {})
            .map_err(|e| Error::WindowCreate(format!("Event dispatch failed: {:?}", e)))
            .unwrap();
    }

    pub fn update(&mut self) {
        self.try_dispatch_events();

        if let Some(resize) = (*self.toplevel_info.0.borrow_mut()).take() {
            // Don't try to resize to 0x0
            if self.resizable && resize != (0, 0) {
                self.width = resize.0;
                self.height = resize.1;
            }
        }
        if *self.toplevel_info.1.borrow() {
            self.should_close = true;
        }

        for event in self.input.iter_keyboard_events() {
            use wayland_client::protocol::wl_keyboard::Event;

            match event {
                Event::Keymap { format, fd, size } => {
                    let keymap = Self::handle_keymap(self.xkb_context, format, fd, size).unwrap();
                    self.xkb_keymap = keymap;
                    self.xkb_state = unsafe { ffi_dispatch!(XKBH, xkb_state_new, keymap) };
                }
                Event::Enter { .. } => {
                    self.active = true;
                }
                Event::Leave { .. } => {
                    self.active = false;
                }
                Event::Key { key, state, .. } => {
                    if !self.xkb_state.is_null() {
                        Self::handle_key(
                            self.xkb_state,
                            key + KEY_XKB_OFFSET,
                            state,
                            &mut self.key_handler,
                        );
                    }
                }
                Event::Modifiers {
                    mods_depressed,
                    mods_latched,
                    mods_locked,
                    group,
                    ..
                } => {
                    if !self.xkb_state.is_null() {
                        unsafe {
                            ffi_dispatch!(
                                XKBH,
                                xkb_state_update_mask,
                                self.xkb_state,
                                mods_depressed,
                                mods_latched,
                                mods_locked,
                                0,
                                0,
                                group
                            )
                        };
                    }
                }
                _ => {}
            }
        }

        self.scroll_x = 0.;
        self.scroll_y = 0.;

        for event in self.input.iter_pointer_events() {
            use wayland_client::protocol::wl_pointer::Event;

            match event {
                Event::Enter {
                    serial,
                    surface_x,
                    surface_y,
                    ..
                } => {
                    self.mouse_x = surface_x as f32;
                    self.mouse_y = surface_y as f32;

                    self.input.get_pointer().set_cursor(
                        serial,
                        Some(&self.display.cursor_surface),
                        0,
                        0,
                    );
                    self.display
                        .update_cursor(Self::decode_cursor(self.prev_cursor))
                        .unwrap();

                    if self.pointer_visibility {
                        self.input.get_pointer().set_cursor(
                            serial,
                            Some(&self.display.cursor_surface),
                            0,
                            0,
                        );
                    } else {
                        self.input.get_pointer().set_cursor(serial, None, 0, 0);
                    }
                }
                Event::Motion {
                    surface_x,
                    surface_y,
                    ..
                } => {
                    self.mouse_x = surface_x as f32;
                    self.mouse_y = surface_y as f32;
                }
                Event::Button {
                    button,
                    state,
                    serial,
                    ..
                } => {
                    use wayland_client::protocol::wl_pointer::ButtonState;

                    let pressed = state == ButtonState::Pressed;

                    match button {
                        // Left mouse button
                        KEY_MOUSE_BTN1 => self.buttons[0] = pressed,
                        // Right mouse button
                        KEY_MOUSE_BTN2 => self.buttons[1] = pressed,
                        // Middle mouse button
                        KEY_MOUSE_BTN3 => self.buttons[2] = pressed,
                        _ => {
                            // TODO: handle more mouse buttons (see: linux/input-event-codes.h from
                            // the Linux kernel)
                        }
                    }

                    if self.pointer_visibility {
                        self.input.get_pointer().set_cursor(
                            serial,
                            Some(&self.display.cursor_surface),
                            0,
                            0,
                        );
                    } else {
                        self.input.get_pointer().set_cursor(serial, None, 0, 0);
                    }
                }
                Event::Axis { axis, value, .. } => {
                    use wayland_client::protocol::wl_pointer::Axis;

                    match axis {
                        Axis::VerticalScroll => self.scroll_y = value as f32,
                        Axis::HorizontalScroll => self.scroll_x = value as f32,
                        _ => {}
                    }
                }
                Event::Frame {} => {
                    // TODO
                }
                Event::AxisSource { axis_source } => {
                    let _ = axis_source;
                    // TODO
                }
                Event::AxisStop { axis, .. } => {
                    use wayland_client::protocol::wl_pointer::Axis;

                    match axis {
                        Axis::VerticalScroll => self.scroll_y = 0.,
                        Axis::HorizontalScroll => self.scroll_x = 0.,
                        _ => {}
                    }
                }
                Event::AxisDiscrete { axis, discrete } => {
                    let _ = (axis, discrete);
                    // TODO
                }
                Event::Leave { serial, .. } => {
                    if self.pointer_visibility {
                        self.input.get_pointer().set_cursor(
                            serial,
                            Some(&self.display.cursor_surface),
                            0,
                            0,
                        );
                    } else {
                        self.input.get_pointer().set_cursor(serial, None, 0, 0);
                    }
                }
                _ => {}
            }
        }

        self.key_handler.update();
    }

    fn handle_key(
        keymap_state: *mut xkb_ffi::xkb_state,
        key: u32,
        state: wl_keyboard::KeyState,
        key_handler: &mut KeyHandler,
    ) {
        let is_down = state == wl_keyboard::KeyState::Pressed;
        let key_xkb = unsafe { ffi_dispatch!(XKBH, xkb_state_key_get_one_sym, keymap_state, key) };
        if key_xkb != 0 {
            use super::xkb_keysyms as key;

            if state == wl_keyboard::KeyState::Pressed {
                // Taken from GLFW
                let code_point = unsafe { ffi_dispatch!(XKBH, xkb_keysym_to_utf32, key_xkb) };
                if !(code_point < 32 || (code_point > 126 && code_point < 160)) {
                    if let Some(ref mut callback) = key_handler.key_callback {
                        callback.add_char(code_point);
                    }
                }
            }

            let key_i = match key_xkb {
                key::XKB_KEY_0 => Key::Key0,
                key::XKB_KEY_1 => Key::Key1,
                key::XKB_KEY_2 => Key::Key2,
                key::XKB_KEY_3 => Key::Key3,
                key::XKB_KEY_4 => Key::Key4,
                key::XKB_KEY_5 => Key::Key5,
                key::XKB_KEY_6 => Key::Key6,
                key::XKB_KEY_7 => Key::Key7,
                key::XKB_KEY_8 => Key::Key8,
                key::XKB_KEY_9 => Key::Key9,

                key::XKB_KEY_a => Key::A,
                key::XKB_KEY_b => Key::B,
                key::XKB_KEY_c => Key::C,
                key::XKB_KEY_d => Key::D,
                key::XKB_KEY_e => Key::E,
                key::XKB_KEY_f => Key::F,
                key::XKB_KEY_g => Key::G,
                key::XKB_KEY_h => Key::H,
                key::XKB_KEY_i => Key::I,
                key::XKB_KEY_j => Key::J,
                key::XKB_KEY_k => Key::K,
                key::XKB_KEY_l => Key::L,
                key::XKB_KEY_m => Key::M,
                key::XKB_KEY_n => Key::N,
                key::XKB_KEY_o => Key::O,
                key::XKB_KEY_p => Key::P,
                key::XKB_KEY_q => Key::Q,
                key::XKB_KEY_r => Key::R,
                key::XKB_KEY_s => Key::S,
                key::XKB_KEY_t => Key::T,
                key::XKB_KEY_u => Key::U,
                key::XKB_KEY_v => Key::V,
                key::XKB_KEY_w => Key::W,
                key::XKB_KEY_x => Key::X,
                key::XKB_KEY_y => Key::Y,
                key::XKB_KEY_z => Key::Z,

                key::XKB_KEY_apostrophe => Key::Apostrophe,
                key::XKB_KEY_grave => Key::Backquote,
                key::XKB_KEY_backslash => Key::Backslash,
                key::XKB_KEY_comma => Key::Comma,
                key::XKB_KEY_equal => Key::Equal,
                key::XKB_KEY_bracketleft => Key::LeftBracket,
                key::XKB_KEY_bracketright => Key::RightBracket,
                key::XKB_KEY_minus => Key::Minus,
                key::XKB_KEY_period => Key::Period,
                key::XKB_KEY_semicolon => Key::Semicolon,
                key::XKB_KEY_slash => Key::Slash,
                key::XKB_KEY_space => Key::Space,

                key::XKB_KEY_F1 => Key::F1,
                key::XKB_KEY_F2 => Key::F2,
                key::XKB_KEY_F3 => Key::F3,
                key::XKB_KEY_F4 => Key::F4,
                key::XKB_KEY_F5 => Key::F5,
                key::XKB_KEY_F6 => Key::F6,
                key::XKB_KEY_F7 => Key::F7,
                key::XKB_KEY_F8 => Key::F8,
                key::XKB_KEY_F9 => Key::F9,
                key::XKB_KEY_F10 => Key::F10,
                key::XKB_KEY_F11 => Key::F11,
                key::XKB_KEY_F12 => Key::F12,

                key::XKB_KEY_Down => Key::Down,
                key::XKB_KEY_Left => Key::Left,
                key::XKB_KEY_Right => Key::Right,
                key::XKB_KEY_Up => Key::Up,
                key::XKB_KEY_Escape => Key::Escape,
                key::XKB_KEY_BackSpace => Key::Backspace,
                key::XKB_KEY_Delete => Key::Delete,
                key::XKB_KEY_End => Key::End,
                key::XKB_KEY_Return => Key::Enter,
                key::XKB_KEY_Home => Key::Home,
                key::XKB_KEY_Insert => Key::Insert,
                key::XKB_KEY_Menu => Key::Menu,
                key::XKB_KEY_Page_Down => Key::PageDown,
                key::XKB_KEY_Page_Up => Key::PageUp,
                key::XKB_KEY_Pause => Key::Pause,
                key::XKB_KEY_Tab => Key::Tab,
                key::XKB_KEY_Num_Lock => Key::NumLock,
                key::XKB_KEY_Caps_Lock => Key::CapsLock,
                key::XKB_KEY_Scroll_Lock => Key::ScrollLock,
                key::XKB_KEY_Shift_L => Key::LeftShift,
                key::XKB_KEY_Shift_R => Key::RightShift,
                key::XKB_KEY_Alt_L => Key::LeftAlt,
                key::XKB_KEY_Alt_R => Key::RightAlt,
                key::XKB_KEY_Control_L => Key::LeftCtrl,
                key::XKB_KEY_Control_R => Key::RightCtrl,
                key::XKB_KEY_Super_L => Key::LeftSuper,
                key::XKB_KEY_Super_R => Key::RightSuper,

                key::XKB_KEY_KP_Insert => Key::NumPad0,
                key::XKB_KEY_KP_End => Key::NumPad1,
                key::XKB_KEY_KP_Down => Key::NumPad2,
                key::XKB_KEY_KP_Next => Key::NumPad3,
                key::XKB_KEY_KP_Left => Key::NumPad4,
                key::XKB_KEY_KP_Begin => Key::NumPad5,
                key::XKB_KEY_KP_Right => Key::NumPad6,
                key::XKB_KEY_KP_Home => Key::NumPad7,
                key::XKB_KEY_KP_Up => Key::NumPad8,
                key::XKB_KEY_KP_Prior => Key::NumPad9,
                key::XKB_KEY_KP_Decimal => Key::NumPadDot,
                key::XKB_KEY_KP_Divide => Key::NumPadSlash,
                key::XKB_KEY_KP_Multiply => Key::NumPadAsterisk,
                key::XKB_KEY_KP_Subtract => Key::NumPadMinus,
                key::XKB_KEY_KP_Add => Key::NumPadPlus,
                key::XKB_KEY_KP_Enter => Key::NumPadEnter,

                _ => {
                    // Ignore other keys
                    return;
                }
            };

            key_handler.set_key_state(key_i, is_down);
        }
    }

    fn handle_keymap(
        context: *mut xkb_ffi::xkb_context,
        keymap: KeymapFormat,
        fd: RawFd,
        len: u32,
    ) -> Result<*mut xkb_ffi::xkb_keymap> {
        match keymap {
            KeymapFormat::XkbV1 => {
                unsafe {
                    // The file descriptor must be memory-mapped (with MAP_PRIVATE).
                    let addr = libc::mmap(
                        std::ptr::null_mut(),
                        len as usize,
                        libc::PROT_READ,
                        libc::MAP_PRIVATE,
                        fd,
                        0,
                    );
                    if addr == libc::MAP_FAILED {
                        return Err(Error::WindowCreate(format!(
                            "Could not mmap keymap from compositor ({})",
                            std::io::Error::last_os_error()
                        )));
                    }

                    let keymap = ffi_dispatch!(
                        XKBH,
                        xkb_keymap_new_from_string,
                        context,
                        addr as *const _,
                        xkb_ffi::xkb_keymap_format::XKB_KEYMAP_FORMAT_TEXT_V1,
                        xkb_ffi::xkb_keymap_compile_flags::XKB_KEYMAP_COMPILE_NO_FLAGS
                    );

                    libc::munmap(addr, len as usize);

                    if keymap.is_null() {
                        Err(Error::WindowCreate(
                            "Received invalid keymap from compositor.".to_owned(),
                        ))
                    } else {
                        Ok(keymap)
                    }
                }
            }
            _ => unimplemented!("Only XKB keymaps are supported"),
        }
    }

    #[inline]
    fn decode_cursor(cursor: CursorStyle) -> &'static str {
        match cursor {
            CursorStyle::Arrow => "arrow",
            CursorStyle::Ibeam => "xterm",
            CursorStyle::Crosshair => "crosshair",
            CursorStyle::ClosedHand => "hand2",
            CursorStyle::OpenHand => "hand2",
            CursorStyle::ResizeLeftRight => "sb_h_double_arrow",
            CursorStyle::ResizeUpDown => "sb_v_double_arrow",
            CursorStyle::ResizeAll => "diamond_cross",
        }
    }

    #[inline]
    pub fn set_cursor_style(&mut self, cursor: CursorStyle) {
        if self.prev_cursor != cursor {
            self.display
                .update_cursor(Self::decode_cursor(cursor))
                .unwrap();
            self.prev_cursor = cursor;
        }
    }

    pub fn update_with_buffer_stride(
        &mut self,
        buffer: &[u32],
        buf_width: usize,
        buf_height: usize,
        buf_stride: usize,
    ) -> Result<()> {
        check_buffer_size(buffer, buf_width, buf_height, buf_stride)?;

        unsafe { self.scale_buffer(buffer, buf_width, buf_height, buf_stride) };

        self.display
            .update_framebuffer(&self.buffer, (self.width, self.height))
            .map_err(|e| Error::UpdateFailed(format!("Error updating framebuffer: {:?}", e)))?;
        self.update();

        Ok(())
    }

    unsafe fn scale_buffer(
        &mut self,
        buffer: &[u32],
        buf_width: usize,
        buf_height: usize,
        buf_stride: usize,
    ) {
        self.buffer.resize((self.width * self.height) as usize, 0);

        match self.scale_mode {
            ScaleMode::Stretch => {
                image_resize_linear(
                    self.buffer.as_mut_ptr(),
                    self.width as u32,
                    self.height as u32,
                    buffer.as_ptr(),
                    buf_width as u32,
                    buf_height as u32,
                    buf_stride as u32,
                );
            }

            ScaleMode::AspectRatioStretch => {
                image_resize_linear_aspect_fill(
                    self.buffer.as_mut_ptr(),
                    self.width as u32,
                    self.height as u32,
                    buffer.as_ptr(),
                    buf_width as u32,
                    buf_height as u32,
                    buf_stride as u32,
                    self.bg_color,
                );
            }

            ScaleMode::Center => {
                image_center(
                    self.buffer.as_mut_ptr(),
                    self.width as u32,
                    self.height as u32,
                    buffer.as_ptr(),
                    buf_width as u32,
                    buf_height as u32,
                    buf_stride as u32,
                    self.bg_color,
                );
            }

            ScaleMode::UpperLeft => {
                image_upper_left(
                    self.buffer.as_mut_ptr(),
                    self.width as u32,
                    self.height as u32,
                    buffer.as_ptr(),
                    buf_width as u32,
                    buf_height as u32,
                    buf_stride as u32,
                    self.bg_color,
                );
            }
        }
    }
}

impl HasWindowHandle for Window {
    fn window_handle(&self) -> std::result::Result<WindowHandle, HandleError> {
        let raw_display_surface = self.display.surface.as_ref().c_ptr() as *mut c_void;
        let display_surface = match NonNull::new(raw_display_surface) {
            Some(display_surface) => display_surface,
            None => unimplemented!("null display surface"),
        };

        let handle = WaylandWindowHandle::new(display_surface);
        let raw_handle = RawWindowHandle::Wayland(handle);
        unsafe { Ok(WindowHandle::borrow_raw(raw_handle)) }
    }
}

impl HasDisplayHandle for Window {
    fn display_handle(&self) -> std::result::Result<DisplayHandle, HandleError> {
        let raw_display = self
            .display
            .attached_display
            .clone()
            .detach()
            .as_ref()
            .c_ptr() as *mut c_void;
        let display = match NonNull::new(raw_display) {
            Some(display) => display,
            None => unimplemented!("null display"),
        };
        let handle = WaylandDisplayHandle::new(display);
        let raw_handle = RawDisplayHandle::Wayland(handle);
        unsafe { Ok(DisplayHandle::borrow_raw(raw_handle)) }
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            ffi_dispatch!(XKBH, xkb_state_unref, self.xkb_state);
            ffi_dispatch!(XKBH, xkb_keymap_unref, self.xkb_keymap);
            ffi_dispatch!(XKBH, xkb_context_unref, self.xkb_context);
        }
    }
}
