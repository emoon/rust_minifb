use crate::key_handler::KeyHandler;
use crate::mouse_handler;
use crate::rate::UpdateRate;
use crate::{
    os::unix::x11::Menu, InputCallback, Key, KeyRepeat, MouseButton, MouseMode, Scale, ScaleMode,
    WindowOptions,
};
use crate::{CursorStyle, MenuHandle, UnixMenu};
use crate::{Error, Result};

use std::ffi::c_void;
use std::io::Write;
use wayland_client::protocol::{
    wl_buffer::WlBuffer,
    wl_compositor::WlCompositor,
    wl_display::WlDisplay,
    wl_keyboard::{KeymapFormat, WlKeyboard},
    wl_pointer::WlPointer,
    wl_seat::WlSeat,
    wl_shm::{Format, WlShm},
    wl_shm_pool::WlShmPool,
    wl_surface::WlSurface,
    {wl_keyboard, wl_pointer},
};
use wayland_client::{Attached, Main};
use wayland_client::{EventQueue, GlobalManager};
use wayland_protocols::xdg_shell::client::{
    xdg_surface::XdgSurface, xdg_toplevel::XdgToplevel, xdg_wm_base::XdgWmBase,
};

use std::cell::RefCell;
use std::os::unix::io::RawFd;
use std::rc::Rc;
use std::sync::mpsc;

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

struct SingleBuffer {
    fd: std::fs::File,
    // Shm pool and size
    pool: Main<WlShmPool>,
    pool_size: i32,
    // Buffer and state
    buffer: Main<WlBuffer>,
    buffer_state: Rc<RefCell<bool>>,
    fb_size: (i32, i32),
}

struct BufferPool {
    pool: Vec<SingleBuffer>,
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
        let buf_not_needed = Rc::new(RefCell::new(false));
        {
            let buf_not_needed = buf_not_needed.clone();

            buf.quick_assign(move |_, event, _| {
                use wayland_client::protocol::wl_buffer::Event;

                if let Event::Release = event {
                    *buf_not_needed.borrow_mut() = true;
                }
            });
        }

        (buf, buf_not_needed)
    }

    fn get_buffer(&mut self, size: (i32, i32)) -> (std::fs::File, &Main<WlBuffer>) {
        use std::os::unix::io::AsRawFd;

        let pos = self.pool.iter().rposition(|e| *e.buffer_state.borrow());
        let size_bytes = size.0 * size.1 * std::mem::size_of::<u32>() as i32;
        // If possible take an older shm_pool and create a new buffer onto it
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

            (
                self.pool[idx].fd.try_clone().unwrap(),
                &self.pool[idx].buffer,
            )
        } else {
            let file = tempfile::tempfile().unwrap();
            let shm_pool = self.shm.create_pool(
                file.as_raw_fd(),
                size.0 * size.1 * std::mem::size_of::<u32>() as i32,
            );
            let buffer = Self::create_shm_buffer(&shm_pool, size, self.format);

            self.pool.push(SingleBuffer {
                fd: file,
                pool: shm_pool,
                pool_size: size_bytes,
                buffer: buffer.0,
                buffer_state: buffer.1,
                fb_size: size,
            });

            (
                self.pool[self.pool.len() - 1].fd.try_clone().unwrap(),
                &self.pool[self.pool.len() - 1].buffer,
            )
        }
    }
}

struct DisplayInfo {
    wl_display: Attached<WlDisplay>,
    _compositor: Main<WlCompositor>,
    _base: Main<XdgWmBase>,
    surface: Main<WlSurface>,
    xdg_surface: Main<XdgSurface>,
    toplevel: Main<XdgToplevel>,
    _shm: Main<WlShm>,
    event_queue: EventQueue,
    _seat: Main<WlSeat>,
    xdg_config: Rc<RefCell<Option<u32>>>,
    cursor: wayland_cursor::CursorTheme,
    cursor_surface: Main<WlSurface>,
    _display: wayland_client::Display,
    buf_pool: BufferPool,
}

impl DisplayInfo {
    // size: size of the surface to be created
    // alpha: whether the alpha channel shall be rendered
    // decoration: whether server-side window decoration shall be created
    fn new(size: (i32, i32), alpha: bool, decoration: bool) -> Result<(Self, WaylandInput)> {
        // Get the wayland display
        let display = wayland_client::Display::connect_to_env().map_err(|e| {
            Error::WindowCreate(format!("Failed connecting to the Wayland Display: {:?}", e))
        })?;
        let mut event_queue = display.create_event_queue();

        // Access internal WlDisplay with a token
        let wl_display = (*display).clone().attach(event_queue.token());
        let globals = GlobalManager::new(&wl_display);

        // Wait the wayland server to process all events
        event_queue
            .sync_roundtrip(&mut (), |_, _, _| unreachable!())
            .map_err(|e| Error::WindowCreate(format!("Roundtrip failed: {:?}", e)))?;

        // Requires version 5 for the scroll events
        let seat = globals
            .instantiate_exact::<WlSeat>(5)
            .map_err(|e| Error::WindowCreate(format!("Failed retrieving the WlSeat: {:?}", e)))?;

        // Create the input devices already at this point
        let input = WaylandInput::new(&seat);

        // Retrieve some types from globals
        let compositor = globals.instantiate_exact::<WlCompositor>(4).map_err(|e| {
            Error::WindowCreate(format!("Failed retrieving the compositor: {:?}", e))
        })?;
        let shm = globals.instantiate_exact::<WlShm>(1).map_err(|e| {
            Error::WindowCreate(format!("Failed creating the shared memory: {:?}", e))
        })?;

        let surface = compositor.create_surface();

        // Specify format
        let format = if alpha {
            Format::Argb8888
        } else {
            Format::Xrgb8888
        };

        // Retrive file to write to
        let mut buf_pool = BufferPool::new(shm.clone(), format);
        let (mut tempfile, buffer) = buf_pool.get_buffer(size);

        // Add a black canvas into the framebuffer
        let frame: Vec<u32> = vec![0xFF00_0000; (size.0 * size.1) as usize];
        let slice = unsafe {
            std::slice::from_raw_parts(
                frame[..].as_ptr() as *const u8,
                frame.len() * std::mem::size_of::<u32>(),
            )
        };
        tempfile.write_all(&slice[..]).unwrap();
        tempfile.flush().unwrap();

        let xdg_wm_base = globals.instantiate_exact::<XdgWmBase>(1).map_err(|e| {
            Error::WindowCreate(format!("Failed retrieving the XdgWmBase: {:?}", e))
        })?;

        // Reply to Ping
        xdg_wm_base.quick_assign(|xdg_wm_base, event, _| {
            use wayland_protocols::xdg_shell::client::xdg_wm_base::Event;

            if let Event::Ping { serial } = event {
                xdg_wm_base.pong(serial);
            }
        });

        let xdg_surface = xdg_wm_base.get_xdg_surface(&surface);
        let surface_clone = surface.clone();
        // Handle Ping
        xdg_surface.quick_assign(move |xdg_surface, event, _| {
            use wayland_protocols::xdg_shell::client::xdg_surface::Event;

            if let Event::Configure { serial } = event {
                xdg_surface.ack_configure(serial);
                surface_clone.commit();
            }
        });

        // Assigns the toplevel role and commit
        let xdg_toplevel = xdg_surface.get_toplevel();
        if decoration {
            use wayland_protocols::unstable::xdg_decoration::v1::client::zxdg_decoration_manager_v1::ZxdgDecorationManagerV1;

            if let Ok(decorations) = globals
                .instantiate_exact::<ZxdgDecorationManagerV1>(1)
                .map_err(|e| {
                    Error::WindowCreate(format!(
                        "Failed creating server-side surface decoration: {:?}",
                        e
                    ))
                })
                .map_err(|e| println!("{:?}", e))
            {
                decorations.get_toplevel_decoration(&xdg_toplevel);
                decorations.destroy();
            }
        }
        surface.commit();

        event_queue
            .sync_roundtrip(&mut (), |_, _, _| {})
            .map_err(|e| Error::WindowCreate(format!("Roundtrip failed: {:?}", e)))?;

        // Give the surface the buffer and commit
        surface.attach(Some(&buffer), 0, 0);
        surface.damage(0, 0, i32::max_value(), i32::max_value());
        surface.commit();

        let xdg_config = Rc::new(RefCell::new(None));
        {
            let xdg_config = xdg_config.clone();
            xdg_surface.quick_assign(move |_xdg_surface, event, _| {
                use wayland_protocols::xdg_shell::client::xdg_surface::Event;

                // Acknowledge only the last configure
                if let Event::Configure { serial } = event {
                    *xdg_config.borrow_mut() = Some(serial);
                }
            });
        }

        let cursor = wayland_cursor::load_theme(None, 16, &shm);
        let cursor_surface = compositor.create_surface();

        Ok((
            Self {
                _display: display,
                wl_display,
                _compositor: compositor,
                _base: xdg_wm_base,
                surface,
                xdg_surface,
                toplevel: xdg_toplevel,
                _shm: shm,
                event_queue: event_queue,
                _seat: seat,
                xdg_config,
                cursor,
                cursor_surface,
                buf_pool,
            },
            input,
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
    fn update_cursor(&mut self, cursor: &str) {
        let csr = self.cursor.get_cursor(cursor).unwrap();
        let img = csr.frame_buffer(0).unwrap();
        self.cursor_surface.attach(Some(&*img), 0, 0);
        self.cursor_surface.damage(0, 0, 32, 32);
        self.cursor_surface.commit();
    }

    // Resizes when buffer is bigger or less
    fn update_framebuffer(&mut self, buffer: &[u32], size: (i32, i32)) {
        use std::io::{Seek, SeekFrom};

        let (mut fd, buf) = self.buf_pool.get_buffer(size);

        fd.seek(SeekFrom::Start(0)).unwrap();
        let slice = unsafe {
            std::slice::from_raw_parts(
                buffer[..].as_ptr() as *const u8,
                buffer.len() * std::mem::size_of::<u32>(),
            )
        };
        fd.write_all(&slice[..]).unwrap();
        fd.flush().unwrap();

        // Acknowledge the last configure event
        if let Some(serial) = (*self.xdg_config.borrow_mut()).take() {
            self.xdg_surface.ack_configure(serial);
        }

        self.surface.attach(Some(buf), 0, 0);
        self.surface
            .damage(0, 0, i32::max_value(), i32::max_value());
        self.surface.commit();
    }

    fn get_toplevel_info(&self) -> (Rc<RefCell<Option<(i32, i32)>>>, Rc<RefCell<bool>>) {
        let configure = Rc::new(RefCell::new(None));
        let close = Rc::new(RefCell::new(false));

        {
            let configure = configure.clone();
            let close = close.clone();

            self.toplevel.quick_assign(move |_, event, _| {
                use wayland_protocols::xdg_shell::client::xdg_toplevel::Event;

                if let Event::Configure {
                    width,
                    height,
                    states: _,
                } = event
                {
                    *configure.borrow_mut() = Some((width, height));
                } else if let Event::Close = event {
                    *close.borrow_mut() = true;
                }
            });
        }

        (configure, close)
    }

    // Keyboard, Pointer, Touch
    fn _check_capabilities(
        seat: &Main<WlSeat>,
        event_queue: &mut EventQueue,
    ) -> (bool, bool, bool) {
        let keyboard_fl = Rc::new(RefCell::new(false));
        let pointer_fl = Rc::new(RefCell::new(false));
        let touch_fl = Rc::new(RefCell::new(false));

        {
            let keyboard_fl = keyboard_fl.clone();
            let pointer_fl = pointer_fl.clone();
            let touch_fl = touch_fl.clone();

            // Check pointer and mouse capability
            seat.quick_assign(move |_, event, _| {
                use wayland_client::protocol::wl_seat::{Capability, Event};

                if let Event::Capabilities { capabilities } = event {
                    if !*pointer_fl.borrow() && capabilities.contains(Capability::Pointer) {
                        *pointer_fl.borrow_mut() = true;
                    }
                    if !*keyboard_fl.borrow() && capabilities.contains(Capability::Keyboard) {
                        *keyboard_fl.borrow_mut() = true;
                    }
                    if !*touch_fl.borrow() && capabilities.contains(Capability::Touch) {
                        *touch_fl.borrow_mut() = true;
                    }
                }
            });
        }

        event_queue
            .sync_roundtrip(&mut (), |_, _, _| {})
            .map_err(|e| Error::WindowCreate(format!("Roundtrip failed: {:?}", e)))
            .unwrap();

        let ret = (
            *keyboard_fl.borrow(),
            *pointer_fl.borrow(),
            *touch_fl.borrow(),
        );

        ret
    }
}

struct WaylandInput {
    kb_events: mpsc::Receiver<wl_keyboard::Event>,
    pt_events: mpsc::Receiver<wl_pointer::Event>,
    keyboard: Main<WlKeyboard>,
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
            keyboard,
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
    buttons: [u8; 3],
    prev_cursor: CursorStyle,

    should_close: bool,
    active: bool,

    key_handler: KeyHandler,
    // Option because MaybeUninit's get_ref() is nightly-only
    keymap: Option<xkb::keymap::Keymap>,
    update_rate: UpdateRate,
    menu_counter: MenuHandle,
    menus: Vec<UnixMenu>,
    input: WaylandInput,
    resizable: bool,
    // Temporary buffer
    buffer: Vec<u32>,
    // Configure, close
    toplevel_info: (Rc<RefCell<Option<(i32, i32)>>>, Rc<RefCell<bool>>),
}

impl Window {
    pub fn new(name: &str, width: usize, height: usize, opts: WindowOptions) -> Result<Self> {
        let scale: i32 = match opts.scale {
            // STUB: currently not working in Wayland
            Scale::FitScreen => 1,

            Scale::X1 => 1,
            Scale::X2 => 2,
            Scale::X4 => 4,
            Scale::X8 => 8,
            Scale::X16 => 16,
            Scale::X32 => 32,
        };

        let (dsp, input) = DisplayInfo::new(
            (width as i32 * scale, height as i32 * scale),
            false,
            !opts.borderless,
        )?;
        if opts.title {
            dsp.set_title(name);
        }
        if !opts.resize {
            dsp.set_no_resize((width as i32 * scale, height as i32 * scale));
        }

        let (configure, close) = dsp.get_toplevel_info();

        Ok(Self {
            display: dsp,

            width: width as i32 * scale,
            height: height as i32 * scale,

            scale,
            bg_color: 0,
            scale_mode: opts.scale_mode,

            mouse_x: 0.,
            mouse_y: 0.,
            scroll_x: 0.,
            scroll_y: 0.,
            buttons: [0; 3],
            prev_cursor: CursorStyle::Arrow,

            should_close: false,
            active: false,

            key_handler: KeyHandler::new(),
            keymap: None,
            update_rate: UpdateRate::new(),
            menu_counter: MenuHandle(0),
            menus: Vec::new(),
            input,
            resizable: opts.resize,
            buffer: Vec::with_capacity(width * height * scale as usize * scale as usize),
            toplevel_info: (configure, close),
        })
    }

    pub fn set_title(&mut self, title: &str) {
        self.display.set_title(title);
    }

    pub fn set_background_color(&mut self, bg_color: u32) {
        self.bg_color = bg_color;
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

    pub fn get_keys(&self) -> Option<Vec<Key>> {
        self.key_handler.get_keys()
    }

    pub fn get_keys_pressed(&self, repeat: KeyRepeat) -> Option<Vec<Key>> {
        self.key_handler.get_keys_pressed(repeat)
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
            MouseButton::Left => self.buttons[0] > 0,
            MouseButton::Right => self.buttons[1] > 0,
            MouseButton::Middle => self.buttons[2] > 0,
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

    pub fn set_rate(&mut self, rate: Option<std::time::Duration>) {
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

    pub fn get_unix_menus(&self) -> Option<&Vec<UnixMenu>> {
        Some(&self.menus)
    }

    pub fn remove_menu(&mut self, handle: MenuHandle) {
        self.menus.retain(|ref menu| menu.handle != handle);
    }

    pub fn is_menu_pressed(&mut self) -> Option<usize> {
        None
    }

    pub fn update(&mut self) {
        self.display
            .event_queue
            .dispatch(&mut (), |_, _, _| {})
            .map_err(|e| Error::WindowCreate(format!("Event dispatch failed: {:?}", e)))
            .unwrap();

        if let Some(resize) = (*self.toplevel_info.0.borrow_mut()).take() {
            //Dont try to resize to 0x0
            if self.resizable && resize != (0, 0) {
                self.width = resize.0;
                self.height = resize.1;
            }
        }
        if *self.toplevel_info.1.borrow() {
            self.should_close = true;
        }

        const KEY_XKB_OFFSET: u32 = 8;

        for event in self.input.iter_keyboard_events() {
            use wayland_client::protocol::wl_keyboard::Event;
            match event {
                Event::Keymap { format, fd, size } => {
                    self.keymap = Some(Self::handle_keymap(format, fd, size));
                }
                Event::Enter {
                    serial: _,
                    surface: _,
                    keys: _,
                } => {
                    self.active = true;
                }
                Event::Leave {
                    serial: _,
                    surface: _,
                } => {
                    self.active = false;
                }
                Event::Key {
                    serial: _,
                    time: _,
                    key,
                    state,
                } => {
                    if let Some(ref keymap) = self.keymap {
                        Self::handle_key(
                            keymap,
                            key + KEY_XKB_OFFSET,
                            state,
                            &mut self.key_handler,
                        );
                    }
                }
                Event::Modifiers {
                    serial: _,
                    mods_depressed,
                    mods_latched,
                    mods_locked,
                    group,
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
                    surface: _,
                    surface_x,
                    surface_y,
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
                        .update_cursor(Self::decode_cursor(self.prev_cursor));
                }
                Event::Leave {
                    serial: _,
                    surface: _,
                } => {
                    //TODO
                }
                Event::Motion {
                    time: _,
                    surface_x,
                    surface_y,
                } => {
                    self.mouse_x = surface_x as f32;
                    self.mouse_y = surface_y as f32;
                }
                Event::Button {
                    serial: _,
                    time: _,
                    button,
                    state,
                } => {
                    use wayland_client::protocol::wl_pointer::ButtonState;

                    let st = (state == ButtonState::Pressed) as u8;

                    match button {
                        //Left
                        272 => self.buttons[0] = st,
                        //Right
                        273 => self.buttons[1] = st,
                        //Middle
                        274 => self.buttons[2] = st,
                        _ => {}
                    }
                }
                Event::Axis {
                    time: _,
                    axis,
                    value,
                } => {
                    use wayland_client::protocol::wl_pointer::Axis;

                    match axis {
                        Axis::VerticalScroll => self.scroll_y = value as f32,
                        Axis::HorizontalScroll => self.scroll_x = value as f32,
                        _ => {}
                    }
                }
                //Event::Frame {} => {}
                //Event::AxisSource { axis_source } => {}
                Event::AxisStop { time: _, axis } => {
                    use wayland_client::protocol::wl_pointer::Axis;

                    match axis {
                        Axis::VerticalScroll => self.scroll_y = 0.,
                        Axis::HorizontalScroll => self.scroll_x = 0.,
                        _ => {}
                    }
                }
                //Event::AxisDiscrete { axis, discrete } => {}
                _ => {}
            }
        }

        self.key_handler.update();
    }

    fn handle_key(
        keymap: &xkb::keymap::Keymap,
        key: u32,
        state: wayland_client::protocol::wl_keyboard::KeyState,
        key_handler: &mut KeyHandler,
    ) {
        use wayland_client::protocol::wl_keyboard::KeyState;

        let is_down = state == KeyState::Pressed;

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
                key::minus => Key::Minus,
                key::period => Key::Period,
                key::braceright => Key::RightBracket,
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
                    // ignore other keys
                    return;
                }
            };
            key_handler.set_key_state(key_i, is_down);
        }
    }

    fn handle_keymap(keymap: KeymapFormat, fd: RawFd, len: u32) -> xkb::keymap::Keymap {
        use std::io::Read;
        use std::os::unix::io::FromRawFd;

        match keymap {
            KeymapFormat::XkbV1 => {
                use xkb::keymap::Keymap;

                unsafe {
                    //read in fd content into vec
                    let mut file = std::fs::File::from_raw_fd(fd);
                    let mut v = Vec::with_capacity(len as usize);
                    v.set_len(len as usize);
                    file.read_exact(&mut v).unwrap();

                    let ctx = xkbcommon_sys::xkb_context_new(0);
                    //create keymap from string
                    let kb_map_ptr = xkbcommon_sys::xkb_keymap_new_from_string(
                        ctx,
                        v.as_ptr() as *const _ as *const std::os::raw::c_char,
                        xkbcommon_sys::xkb_keymap_format::XKB_KEYMAP_FORMAT_TEXT_v1,
                        0,
                    );
                    //wrap keymap
                    let kb_map = Keymap::from_ptr(kb_map_ptr as *mut _ as *mut c_void);
                    kb_map
                }
            }
            _ => unimplemented!("Only XKB keymaps supported"),
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
            self.display.update_cursor(Self::decode_cursor(cursor));
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
        crate::buffer_helper::check_buffer_size(buf_width, buf_height, buf_width, buffer)?;

        unsafe { self.scale_buffer(buffer, buf_width, buf_height, buf_stride) };

        self.display
            .update_framebuffer(&self.buffer[..], (self.width as i32, self.height as i32));

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
        let mut handle = raw_window_handle::unix::WaylandHandle::empty();
        handle.surface = self.display.surface.as_ref().c_ptr() as *mut _ as *mut c_void;
        handle.display =
            self.display.wl_display.clone().detach().as_ref().c_ptr() as *mut _ as *mut c_void;
        raw_window_handle::RawWindowHandle::Wayland(handle)
    }
}
