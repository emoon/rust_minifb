use crate::buffer_helper;
use crate::key_handler::KeyHandler;
use crate::mouse_handler;
use crate::rate::UpdateRate;
use crate::{CursorStyle, MenuHandle, UnixMenu};
use crate::{Error, Result};
use crate::{
    InputCallback, Key, KeyRepeat, MouseButton, MouseMode, Scale, ScaleMode, WindowOptions,
};

use super::common::Menu;

use wayland_client::protocol::wl_buffer::WlBuffer;
use wayland_client::protocol::wl_compositor::WlCompositor;
use wayland_client::protocol::wl_display::WlDisplay;
use wayland_client::protocol::wl_keyboard::{KeymapFormat, WlKeyboard};
use wayland_client::protocol::wl_pointer::WlPointer;
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::protocol::wl_shm::{Format, WlShm};
use wayland_client::protocol::wl_shm_pool::WlShmPool;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::protocol::{wl_keyboard, wl_pointer};
use wayland_client::{Attached, Display, EventQueue, GlobalManager, Main};
use wayland_protocols::unstable::xdg_decoration::v1::client::zxdg_decoration_manager_v1::ZxdgDecorationManagerV1;
use wayland_protocols::xdg_shell::client::xdg_surface::XdgSurface;
use wayland_protocols::xdg_shell::client::xdg_toplevel::XdgToplevel;
use wayland_protocols::xdg_shell::client::xdg_wm_base::XdgWmBase;
use xkb::keymap::Keymap;

use std::cell::RefCell;
use std::ffi::c_void;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::mem;
use std::os::unix::io::{AsRawFd, FromRawFd, RawFd};
use std::rc::Rc;
use std::slice;
use std::sync::mpsc;
use std::time::Duration;

const KEY_XKB_OFFSET: u32 = 8;
const KEY_MOUSE_BTN1: u32 = 272;
const KEY_MOUSE_BTN2: u32 = 273;
const KEY_MOUSE_BTN3: u32 = 274;

type ToplevelResolution = Rc<RefCell<Option<(i32, i32)>>>;
type ToplevelClosed = Rc<RefCell<bool>>;

// These functions are implemented in C in order to always have
// optimizations on (`-O3`), allowing debug builds to run fast as well.
extern "C" {
    fn Image_upper_left(
        target: *mut u32,
        source: *const u32,
        source_w: u32,
        source_h: u32,
        source_stride: u32,
        dest_width: u32,
        dest_height: u32,
        bg_color: u32,
    );

    fn Image_center(
        target: *mut u32,
        source: *const u32,
        source_w: u32,
        source_h: u32,
        source_stride: u32,
        dest_width: u32,
        dest_height: u32,
        bg_color: u32,
    );

    fn Image_resize_linear_aspect_fill_c(
        target: *mut u32,
        source: *const u32,
        source_w: u32,
        source_h: u32,
        source_stride: u32,
        dest_width: u32,
        dest_height: u32,
        bg_color: u32,
    );

    fn Image_resize_linear_c(
        target: *mut u32,
        source: *const u32,
        source_w: u32,
        source_h: u32,
        source_stride: u32,
        dest_width: u32,
        dest_height: u32,
    );
}

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
            size.0 * mem::size_of::<u32>() as i32,
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
        let size_bytes = size.0 * size.1 * mem::size_of::<u32>() as i32;

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
                let old_buffer = mem::replace(&mut self.pool[idx].buffer, new_buffer.0);
                old_buffer.destroy();
                self.pool[idx].fb_size = size;
            }

            Ok((self.pool[idx].fd.try_clone()?, &self.pool[idx].buffer))
        } else {
            let tempfile = tempfile::tempfile()?;
            let shm_pool = self.shm.create_pool(
                tempfile.as_raw_fd(),
                size.0 * size.1 * mem::size_of::<u32>() as i32,
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
            slice::from_raw_parts(
                frame[..].as_ptr() as *const u8,
                frame.len() * mem::size_of::<u32>(),
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

    fn set_geometry(&self, pos: (i32, i32), size: (i32, i32)) {
        self.xdg_surface
            .set_window_geometry(pos.0, pos.1, size.0, size.1);
    }

    fn set_title(&self, title: &str) {
        self.toplevel.set_title(title.to_owned());
    }

    fn set_no_resize(&self, size: (i32, i32)) {
        self.toplevel.set_max_size(size.0, size.1);
        self.toplevel.set_min_size(size.0, size.1);
    }

    // Sets a specific cursor style
    fn update_cursor(&mut self, cursor: &str) -> std::result::Result<(), ()> {
        let cursor = self.cursor.get_cursor(cursor);
        if let Some(cursor) = cursor {
            let img = &cursor[0];
            self.cursor_surface.attach(Some(&*img), 0, 0);
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
            slice::from_raw_parts(
                buffer[..].as_ptr() as *const u8,
                buffer.len() * mem::size_of::<u32>(),
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

    fn get_pointer(&self) -> &Main<WlPointer> {
        &self.pointer
    }

    fn iter_keyboard_events(&self) -> mpsc::TryIter<wl_keyboard::Event> {
        self.kb_events.try_iter()
    }

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
    // Option because MaybeUninit's get_ref() is nightly-only
    keymap: Option<Keymap>,
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
            keymap: None,
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

    pub fn set_title(&mut self, title: &str) {
        self.display.set_title(title);
    }

    pub fn set_background_color(&mut self, bg_color: u32) {
        self.bg_color = bg_color;
    }

    pub fn set_cursor_visibility(&mut self, visibility: bool) {
        self.pointer_visibility = visibility;
    }

    pub fn is_open(&self) -> bool {
        !self.should_close
    }

    pub fn get_window_handle(&self) -> *mut c_void {
        self.display.surface.as_ref().c_ptr() as *mut c_void
    }

    pub fn get_size(&self) -> (usize, usize) {
        (self.width as usize, self.height as usize)
    }

    pub fn get_keys(&self) -> Vec<Key> {
        self.key_handler.get_keys()
    }

    pub fn get_keys_pressed(&self, repeat: KeyRepeat) -> Vec<Key> {
        self.key_handler.get_keys_pressed(repeat)
    }

    pub fn get_keys_released(&self) -> Vec<Key> {
        self.key_handler.get_keys_released()
    }

    pub fn get_mouse_pos(&self, mode: MouseMode) -> Option<(f32, f32)> {
        mouse_handler::get_pos(
            mode,
            self.mouse_x,
            self.mouse_y,
            self.scale as f32,
            self.width as f32,
            self.height as f32,
        )
    }

    pub fn get_mouse_down(&self, button: MouseButton) -> bool {
        match button {
            MouseButton::Left => self.buttons[0],
            MouseButton::Right => self.buttons[1],
            MouseButton::Middle => self.buttons[2],
        }
    }

    pub fn get_unscaled_mouse_pos(&self, mode: MouseMode) -> Option<(f32, f32)> {
        mouse_handler::get_pos(
            mode,
            self.mouse_x,
            self.mouse_y,
            1.0,
            self.width as f32,
            self.height as f32,
        )
    }

    pub fn get_scroll_wheel(&self) -> Option<(f32, f32)> {
        if self.scroll_x.abs() > 0.0 || self.scroll_y.abs() > 0.0 {
            Some((self.scroll_x, self.scroll_y))
        } else {
            None
        }
    }

    pub fn is_key_down(&self, key: Key) -> bool {
        self.key_handler.is_key_down(key)
    }

    pub fn set_position(&mut self, x: isize, y: isize) {
        self.display
            .set_geometry((x as i32, y as i32), (self.width, self.height));
    }

    pub fn get_position(&self) -> (isize, isize) {
        let (mut x, mut y) = (0, 0);

        unsafe {
            todo!("get_position");
        }

        (x as isize, y as isize)
    }

    pub fn set_rate(&mut self, rate: Option<Duration>) {
        self.update_rate.set_rate(rate);
    }

    pub fn set_key_repeat_rate(&mut self, rate: f32) {
        self.key_handler.set_key_repeat_delay(rate);
    }

    pub fn set_key_repeat_delay(&mut self, delay: f32) {
        self.key_handler.set_key_repeat_delay(delay);
    }

    pub fn set_input_callback(&mut self, callback: Box<dyn InputCallback>) {
        self.key_handler.set_input_callback(callback);
    }

    pub fn is_key_pressed(&self, key: Key, repeat: KeyRepeat) -> bool {
        self.key_handler.is_key_pressed(key, repeat)
    }

    pub fn is_key_released(&self, key: Key) -> bool {
        !self.key_handler.is_key_released(key)
    }

    pub fn update_rate(&mut self) {
        self.update_rate.update();
    }

    pub fn is_active(&self) -> bool {
        self.active
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
        //FIXME
        unimplemented!()
    }

    pub fn remove_menu(&mut self, handle: MenuHandle) {
        self.menus.retain(|menu| menu.handle != handle);
    }

    pub fn is_menu_pressed(&mut self) -> Option<usize> {
        //FIXME
        unimplemented!()
    }

    fn try_dispatch_events(&mut self) {
        // as seen in https://docs.rs/wayland-client/0.28/wayland_client/struct.EventQueue.html
        if let Err(e) = self.display.event_queue.display().flush() {
            if e.kind() != io::ErrorKind::WouldBlock {
                eprintln!("Error while trying to flush the wayland socket: {:?}", e);
            }
        }

        if let Some(guard) = self.display.event_queue.prepare_read() {
            if let Err(e) = guard.read_events() {
                if e.kind() != io::ErrorKind::WouldBlock {
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
                    self.keymap = Some(Self::handle_keymap(format, fd, size).unwrap());
                }
                Event::Enter { .. } => {
                    self.active = true;
                }
                Event::Leave { .. } => {
                    self.active = false;
                }
                Event::Key { key, state, .. } => {
                    if let Some(ref keymap) = self.keymap {
                        Self::handle_key(
                            keymap,
                            key + KEY_XKB_OFFSET,
                            state,
                            &mut self.key_handler,
                        );
                    }

                    if state == wl_keyboard::KeyState::Pressed {
                        let keysym = xkb::Keysym(key);
                        let code_point = keysym.utf32();
                        if code_point != 0 {
                            // Taken from GLFW
                            if !(code_point < 32 || (code_point > 126 && code_point < 160)) {
                                if let Some(ref mut callback) = self.key_handler.key_callback {
                                    callback.add_char(code_point);
                                }
                            }
                        }
                    }
                }
                Event::Modifiers {
                    mods_depressed,
                    mods_latched,
                    mods_locked,
                    group,
                    ..
                } => {
                    if let Some(ref keymap) = self.keymap {
                        let mut state = keymap.state();
                        let mut update = state.update();
                        update.mask(mods_depressed, mods_latched, mods_locked, 0, 0, group);
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
        keymap: &Keymap,
        key: u32,
        state: wl_keyboard::KeyState,
        key_handler: &mut KeyHandler,
    ) {
        let is_down = state == wl_keyboard::KeyState::Pressed;
        let state = keymap.state();
        let key_xkb = state.key(key);

        if let Some(keysym) = key_xkb.sym() {
            use xkb::key;

            let key_i = match keysym {
                key::_0 => Key::Key0,
                key::_1 => Key::Key1,
                key::_2 => Key::Key2,
                key::_3 => Key::Key3,
                key::_4 => Key::Key4,
                key::_5 => Key::Key5,
                key::_6 => Key::Key6,
                key::_7 => Key::Key7,
                key::_8 => Key::Key8,
                key::_9 => Key::Key9,

                key::a => Key::A,
                key::b => Key::B,
                key::c => Key::C,
                key::d => Key::D,
                key::e => Key::E,
                key::f => Key::F,
                key::g => Key::G,
                key::h => Key::H,
                key::i => Key::I,
                key::j => Key::J,
                key::k => Key::K,
                key::l => Key::L,
                key::m => Key::M,
                key::n => Key::N,
                key::o => Key::O,
                key::p => Key::P,
                key::q => Key::Q,
                key::r => Key::R,
                key::s => Key::S,
                key::t => Key::T,
                key::u => Key::U,
                key::v => Key::V,
                key::w => Key::W,
                key::x => Key::X,
                key::y => Key::Y,
                key::z => Key::Z,

                key::apostrophe => Key::Apostrophe,
                key::grave => Key::Backquote,
                key::backslash => Key::Backslash,
                key::comma => Key::Comma,
                key::equal => Key::Equal,
                key::bracketleft => Key::LeftBracket,
                key::bracketright => Key::RightBracket,
                key::minus => Key::Minus,
                key::period => Key::Period,
                key::semicolon => Key::Semicolon,
                key::slash => Key::Slash,
                key::space => Key::Space,

                key::F1 => Key::F1,
                key::F2 => Key::F2,
                key::F3 => Key::F3,
                key::F4 => Key::F4,
                key::F5 => Key::F5,
                key::F6 => Key::F6,
                key::F7 => Key::F7,
                key::F8 => Key::F8,
                key::F9 => Key::F9,
                key::F10 => Key::F10,
                key::F11 => Key::F11,
                key::F12 => Key::F12,

                key::Down => Key::Down,
                key::Left => Key::Left,
                key::Right => Key::Right,
                key::Up => Key::Up,
                key::Escape => Key::Escape,
                key::BackSpace => Key::Backspace,
                key::Delete => Key::Delete,
                key::End => Key::End,
                key::Return => Key::Enter,
                key::Home => Key::Home,
                key::Insert => Key::Insert,
                key::Menu => Key::Menu,
                key::Page_Down => Key::PageDown,
                key::Page_Up => Key::PageUp,
                key::Pause => Key::Pause,
                key::Tab => Key::Tab,
                key::Num_Lock => Key::NumLock,
                key::Caps_Lock => Key::CapsLock,
                key::Scroll_Lock => Key::ScrollLock,
                key::Shift_L => Key::LeftShift,
                key::Shift_R => Key::RightShift,
                key::Alt_L => Key::LeftAlt,
                key::Alt_R => Key::RightAlt,
                key::Control_L => Key::LeftCtrl,
                key::Control_R => Key::RightCtrl,
                key::Super_L => Key::LeftSuper,
                key::Super_R => Key::RightSuper,

                key::KP_Insert => Key::NumPad0,
                key::KP_End => Key::NumPad1,
                key::KP_Down => Key::NumPad2,
                key::KP_Next => Key::NumPad3,
                key::KP_Left => Key::NumPad4,
                key::KP_Begin => Key::NumPad5,
                key::KP_Right => Key::NumPad6,
                key::KP_Home => Key::NumPad7,
                key::KP_Up => Key::NumPad8,
                key::KP_Prior => Key::NumPad9,
                key::KP_Decimal => Key::NumPadDot,
                key::KP_Divide => Key::NumPadSlash,
                key::KP_Multiply => Key::NumPadAsterisk,
                key::KP_Subtract => Key::NumPadMinus,
                key::KP_Add => Key::NumPadPlus,
                key::KP_Enter => Key::NumPadEnter,

                _ => {
                    // Ignore other keys
                    return;
                }
            };

            key_handler.set_key_state(key_i, is_down);
        }
    }

    fn handle_keymap(keymap: KeymapFormat, fd: RawFd, len: u32) -> std::io::Result<Keymap> {
        match keymap {
            KeymapFormat::XkbV1 => {
                unsafe {
                    // Read fd content into Vec
                    let mut file = File::from_raw_fd(fd);
                    let mut v = Vec::with_capacity(len as usize);
                    v.set_len(len as usize);
                    file.read_exact(&mut v)?;

                    let ctx = xkbcommon_sys::xkb_context_new(0);
                    let kb_map_ptr = xkbcommon_sys::xkb_keymap_new_from_string(
                        ctx,
                        v.as_ptr() as *const _ as *const std::os::raw::c_char,
                        xkbcommon_sys::xkb_keymap_format::XKB_KEYMAP_FORMAT_TEXT_V1,
                        0,
                    );

                    // Wrap keymap
                    Ok(Keymap::from_ptr(kb_map_ptr as *mut _ as *mut c_void))
                }
            }
            _ => unimplemented!("Only XKB keymaps are supported"),
        }
    }

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
        buffer_helper::check_buffer_size(buf_width, buf_height, buf_width, buffer)?;

        unsafe { self.scale_buffer(buffer, buf_width, buf_height, buf_stride) };

        self.display
            .update_framebuffer(&self.buffer[..], (self.width as i32, self.height as i32))
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
                Image_resize_linear_c(
                    self.buffer.as_mut_ptr(),
                    buffer.as_ptr(),
                    buf_width as u32,
                    buf_height as u32,
                    buf_stride as u32,
                    self.width as u32,
                    self.height as u32,
                );
            }

            ScaleMode::AspectRatioStretch => {
                Image_resize_linear_aspect_fill_c(
                    self.buffer.as_mut_ptr(),
                    buffer.as_ptr(),
                    buf_width as u32,
                    buf_height as u32,
                    buf_stride as u32,
                    self.width as u32,
                    self.height as u32,
                    self.bg_color,
                );
            }

            ScaleMode::Center => {
                Image_center(
                    self.buffer.as_mut_ptr(),
                    buffer.as_ptr(),
                    buf_width as u32,
                    buf_height as u32,
                    buf_stride as u32,
                    self.width as u32,
                    self.height as u32,
                    self.bg_color,
                );
            }

            ScaleMode::UpperLeft => {
                Image_upper_left(
                    self.buffer.as_mut_ptr(),
                    buffer.as_ptr(),
                    buf_width as u32,
                    buf_height as u32,
                    buf_stride as u32,
                    self.width as u32,
                    self.height as u32,
                    self.bg_color,
                );
            }
        }
    }
}

unsafe impl raw_window_handle::HasRawWindowHandle for Window {
    fn raw_window_handle(&self) -> raw_window_handle::RawWindowHandle {
        let mut handle = raw_window_handle::WaylandHandle::empty();
        handle.surface = self.display.surface.as_ref().c_ptr() as *mut _ as *mut c_void;
        handle.display = self
            .display
            .attached_display
            .clone()
            .detach()
            .as_ref()
            .c_ptr() as *mut _ as *mut c_void;

        raw_window_handle::RawWindowHandle::Wayland(handle)
    }
}
