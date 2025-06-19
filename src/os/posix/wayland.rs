use std::{
    collections::HashMap,
    ffi::c_void,
    fs::File,
    os::unix::io::{AsFd, AsRawFd},
    ptr::NonNull,
    sync::mpsc,
    time::Duration,
};

use super::common::{
    image_center, image_resize_linear, image_resize_linear_aspect_fill, image_upper_left, Menu,
};
use crate::{
    check_buffer_size, key_handler::KeyHandler, rate::UpdateRate, CursorStyle, Error,
    InputCallback, Key, KeyRepeat, MenuHandle, MouseButton, MouseMode, Result, Scale,
    ScaleMode, UnixMenu, WindowOptions,
};
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, WindowHandle,
    WaylandDisplayHandle, WaylandWindowHandle,
};

use wayland_client::{
    protocol::{
        wl_buffer::{self, WlBuffer},
        wl_compositor::WlCompositor,
        wl_keyboard::{self, WlKeyboard},
        wl_pointer::{self, WlPointer},
        wl_registry::{self, WlRegistry},
        wl_seat::{self, WlSeat},
        wl_shm::{self, Format, WlShm},
        wl_shm_pool::WlShmPool,
        wl_surface::WlSurface,
    },
    Connection, Dispatch, EventQueue, Proxy, QueueHandle, WEnum,
};

use wayland_protocols::xdg::{
    decoration::zv1::client::{
        zxdg_decoration_manager_v1::{self, ZxdgDecorationManagerV1},
        zxdg_toplevel_decoration_v1::{self, ZxdgToplevelDecorationV1},
    },
    shell::client::{
        xdg_surface::{self, XdgSurface},
        xdg_toplevel::{self, XdgToplevel},
        xdg_wm_base::{self, XdgWmBase},
    },
};

use wayland_cursor::{CursorTheme, CursorImageBuffer};


const BUFFER_COUNT: usize = 2;

// Key mapping from Linux keycodes to minifb Keys
fn linux_keycode_to_key(keycode: u32) -> Key {
    match keycode {
        // Letters
        30 => Key::A, 48 => Key::B, 46 => Key::C, 32 => Key::D, 18 => Key::E,
        33 => Key::F, 34 => Key::G, 35 => Key::H, 23 => Key::I, 36 => Key::J,
        37 => Key::K, 38 => Key::L, 50 => Key::M, 49 => Key::N, 24 => Key::O,
        25 => Key::P, 16 => Key::Q, 19 => Key::R, 31 => Key::S, 20 => Key::T,
        22 => Key::U, 47 => Key::V, 17 => Key::W, 45 => Key::X, 21 => Key::Y,
        44 => Key::Z,
        
        // Numbers
        11 => Key::Key0, 2 => Key::Key1, 3 => Key::Key2, 4 => Key::Key3, 5 => Key::Key4,
        6 => Key::Key5, 7 => Key::Key6, 8 => Key::Key7, 9 => Key::Key8, 10 => Key::Key9,
        
        // Function keys
        59 => Key::F1, 60 => Key::F2, 61 => Key::F3, 62 => Key::F4, 63 => Key::F5,
        64 => Key::F6, 65 => Key::F7, 66 => Key::F8, 67 => Key::F9, 68 => Key::F10,
        87 => Key::F11, 88 => Key::F12,
        
        // Special keys  
        108 => Key::Down, 105 => Key::Left, 106 => Key::Right, 103 => Key::Up,
        1 => Key::Escape, 14 => Key::Backspace, 111 => Key::Delete, 107 => Key::End,
        28 => Key::Enter, 102 => Key::Home, 110 => Key::Insert, 104 => Key::PageUp,
        109 => Key::PageDown, 119 => Key::Pause, 57 => Key::Space, 15 => Key::Tab,
        
        // Keypad
        82 => Key::NumPad0, 79 => Key::NumPad1, 80 => Key::NumPad2, 81 => Key::NumPad3,
        75 => Key::NumPad4, 76 => Key::NumPad5, 77 => Key::NumPad6, 71 => Key::NumPad7,
        72 => Key::NumPad8, 73 => Key::NumPad9, 83 => Key::NumPadDot, 98 => Key::NumPadSlash,
        55 => Key::NumPadAsterisk, 74 => Key::NumPadMinus, 78 => Key::NumPadPlus,
        96 => Key::NumPadEnter,
        
        // Modifiers
        42 | 54 => Key::LeftShift, 29 | 97 => Key::LeftCtrl, 56 | 100 => Key::LeftAlt,
        125 => Key::LeftSuper, 58 => Key::CapsLock,
        
        // Punctuation
        12 => Key::Minus, 13 => Key::Equal, 26 => Key::LeftBracket, 27 => Key::RightBracket,
        43 => Key::Backslash, 39 => Key::Semicolon, 40 => Key::Apostrophe,
        51 => Key::Comma, 52 => Key::Period, 53 => Key::Slash,
        
        _ => Key::Unknown,
    }
}

struct Buffer {
    file: File,
    len: usize,
    data: *mut u8,
    pool: Option<WlShmPool>,
    buffer: Option<WlBuffer>,
    busy: bool,
}

impl Buffer {
    fn new(size: usize) -> Result<Buffer> {
        let file = tempfile::tempfile().map_err(|e| Error::WindowCreate(format!("Failed to create temp file: {}", e)))?;
        file.set_len(size as u64).map_err(|e| Error::WindowCreate(format!("Failed to set file size: {}", e)))?;
        
        let data = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                file.as_raw_fd(),
                0,
            )
        };

        if data == libc::MAP_FAILED {
            return Err(Error::WindowCreate("Failed to mmap buffer".to_string()));
        }

        Ok(Buffer {
            file,
            len: size,
            data: data as *mut u8,
            pool: None,
            buffer: None,
            busy: false,
        })
    }

    fn get_data(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.data, self.len) }
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        if !self.data.is_null() {
            unsafe {
                libc::munmap(self.data as *mut c_void, self.len);
            }
        }
    }
}

struct BufferPool {
    buffers: Vec<Buffer>,
    width: i32,
    height: i32,
}

impl Default for BufferPool {
    fn default() -> Self {
        Self::new()
    }
}

impl BufferPool {
    fn new() -> Self {
        Self {
            buffers: Vec::new(),
            width: 0,
            height: 0,
        }
    }

    fn get_buffer(&mut self, width: i32, height: i32, shm: &WlShm, qh: &QueueHandle<WaylandState>) -> Result<&mut Buffer> {
        if self.width != width || self.height != height {
            self.resize(width, height, shm, qh)?;
        }

        // Find a non-busy buffer first
        for i in 0..self.buffers.len() {
            if !self.buffers[i].busy {
                return Ok(&mut self.buffers[i]);
            }
        }

        // If all buffers are busy, force release the oldest one
        // This should not happen in a well-behaved compositor, but provides fallback
        if !self.buffers.is_empty() {
            self.buffers[0].busy = false;
            return Ok(&mut self.buffers[0]);
        }

        Err(Error::WindowCreate("No available buffers".to_string()))
    }

    fn resize(&mut self, width: i32, height: i32, shm: &WlShm, qh: &QueueHandle<WaylandState>) -> Result<()> {
        self.width = width;
        self.height = height;
        
        // Clear existing buffers
        self.buffers.clear();

        let stride = width * 4;
        let size = (stride * height) as usize;

        // Create new buffers
        for _ in 0..BUFFER_COUNT {
            let mut buffer = Buffer::new(size)?;
            
            let pool = shm.create_pool(buffer.file.as_fd(), size as i32, qh, ());
            let wl_buffer = pool.create_buffer(0, width, height, stride, Format::Argb8888, qh, ());
            
            buffer.pool = Some(pool);
            buffer.buffer = Some(wl_buffer);
            
            self.buffers.push(buffer);
        }

        Ok(())
    }
}

// State struct that implements all the Dispatch traits
#[derive(Default)]
struct WaylandState {
    registry: Option<WlRegistry>,
    compositor: Option<WlCompositor>,
    shm: Option<WlShm>,
    seat: Option<WlSeat>,
    keyboard: Option<WlKeyboard>,
    pointer: Option<WlPointer>,
    xdg_wm_base: Option<XdgWmBase>,
    decoration_manager: Option<ZxdgDecorationManagerV1>,
    
    // Window state
    surface: Option<WlSurface>,
    xdg_surface: Option<XdgSurface>,
    xdg_toplevel: Option<XdgToplevel>,
    decoration: Option<ZxdgToplevelDecorationV1>,
    
    // Input state
    key_sender: Option<mpsc::Sender<(Key, bool)>>,
    mouse_sender: Option<mpsc::Sender<(MouseButton, bool)>>,
    scroll_sender: Option<mpsc::Sender<(f32, f32)>>,
    mouse_pos_sender: Option<mpsc::Sender<(f32, f32)>>,
    
    // Window state
    width: i32,
    height: i32,
    scale_factor: i32,
    configured: bool,
    closed: bool,
    active: bool,
    
    // Cursor state
    cursor_theme: Option<CursorTheme>,
    cursor_surface: Option<WlSurface>,
    
    // Buffer management
    buffer_pool: BufferPool,
}

impl WaylandState {
    fn new() -> Self {
        Self {
            buffer_pool: BufferPool::new(),
            scale_factor: 1,
            ..Default::default()
        }
    }
}

// Registry dispatch - handles global objects
impl Dispatch<WlRegistry, ()> for WaylandState {
    fn event(
        state: &mut Self,
        registry: &WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_registry::Event::Global { name, interface, version } => {
                match interface.as_str() {
                    "wl_compositor" => {
                        let compositor = registry.bind::<WlCompositor, _, _>(name, version.min(4), qh, ());
                        state.compositor = Some(compositor);
                    }
                    "wl_shm" => {
                        let shm = registry.bind::<WlShm, _, _>(name, version.min(1), qh, ());
                        state.shm = Some(shm);
                    }
                    "wl_seat" => {
                        let seat = registry.bind::<WlSeat, _, _>(name, version.min(7), qh, ());
                        state.seat = Some(seat);
                    }
                    "xdg_wm_base" => {
                        let xdg_wm_base = registry.bind::<XdgWmBase, _, _>(name, version.min(2), qh, ());
                        state.xdg_wm_base = Some(xdg_wm_base);
                    }
                    "zxdg_decoration_manager_v1" => {
                        let decoration_manager = registry.bind::<ZxdgDecorationManagerV1, _, _>(name, version.min(1), qh, ());
                        state.decoration_manager = Some(decoration_manager);
                    }
                    _ => {}
                }
            }
            wl_registry::Event::GlobalRemove { .. } => {}
            _ => {}
        }
    }
}

// Compositor dispatch
impl Dispatch<WlCompositor, ()> for WaylandState {
    fn event(
        _: &mut Self,
        _: &WlCompositor,
        _: <WlCompositor as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

// Shm dispatch
impl Dispatch<WlShm, ()> for WaylandState {
    fn event(
        _: &mut Self,
        _: &WlShm,
        _: wl_shm::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

// Shm pool dispatch
impl Dispatch<WlShmPool, ()> for WaylandState {
    fn event(
        _: &mut Self,
        _: &WlShmPool,
        _: <WlShmPool as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

// Buffer dispatch
impl Dispatch<WlBuffer, ()> for WaylandState {
    fn event(
        state: &mut Self,
        buffer: &WlBuffer,
        event: <WlBuffer as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            wl_buffer::Event::Release => {
                // Find and mark the specific buffer as not busy
                for buf in &mut state.buffer_pool.buffers {
                    if let Some(ref wl_buf) = buf.buffer {
                        if wl_buf == buffer {
                            buf.busy = false;
                            break;
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

// Surface dispatch
impl Dispatch<WlSurface, ()> for WaylandState {
    fn event(
        _: &mut Self,
        _: &WlSurface,
        _: <WlSurface as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

// Seat dispatch
impl Dispatch<WlSeat, ()> for WaylandState {
    fn event(
        state: &mut Self,
        seat: &WlSeat,
        event: wl_seat::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_seat::Event::Capabilities { capabilities } => {
                if let WEnum::Value(caps) = capabilities {
                    if caps.contains(wl_seat::Capability::Keyboard) && state.keyboard.is_none() {
                        let keyboard = seat.get_keyboard(qh, ());
                        state.keyboard = Some(keyboard);
                    }
                    if caps.contains(wl_seat::Capability::Pointer) && state.pointer.is_none() {
                        let pointer = seat.get_pointer(qh, ());
                        state.pointer = Some(pointer);
                    }
                }
            }
            _ => {}
        }
    }
}

// Keyboard dispatch
impl Dispatch<WlKeyboard, ()> for WaylandState {
    fn event(
        state: &mut Self,
        _: &WlKeyboard,
        event: wl_keyboard::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            wl_keyboard::Event::Key { key, state: key_state, .. } => {
                if let Some(sender) = &state.key_sender {
                    let pressed = matches!(key_state, WEnum::Value(wl_keyboard::KeyState::Pressed));
                    let key_enum = linux_keycode_to_key(key);
                    let _ = sender.send((key_enum, pressed));
                }
            }
            wl_keyboard::Event::Enter { .. } => {
                state.active = true;
            }
            wl_keyboard::Event::Leave { .. } => {
                state.active = false;
            }
            wl_keyboard::Event::Keymap { format: _, fd: _, size: _ } => {
                // Handle keymap - for now just acknowledge it
            }
            wl_keyboard::Event::RepeatInfo { rate: _, delay: _ } => {
                // Handle key repeat configuration
            }
            _ => {}
        }
    }
}

// Pointer dispatch
impl Dispatch<WlPointer, ()> for WaylandState {
    fn event(
        state: &mut Self,
        _: &WlPointer,
        event: wl_pointer::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            wl_pointer::Event::Button { button, state: button_state, .. } => {
                if let Some(sender) = &state.mouse_sender {
                    let pressed = matches!(button_state, WEnum::Value(wl_pointer::ButtonState::Pressed));
                    let mouse_button = match button {
                        272 => MouseButton::Left,
                        273 => MouseButton::Right,
                        274 => MouseButton::Middle,
                        _ => return,
                    };
                    let _ = sender.send((mouse_button, pressed));
                }
            }
            wl_pointer::Event::Axis { axis, value, .. } => {
                if let Some(sender) = &state.scroll_sender {
                    match axis {
                        WEnum::Value(wl_pointer::Axis::VerticalScroll) => {
                            let _ = sender.send((0.0, value as f32));
                        }
                        WEnum::Value(wl_pointer::Axis::HorizontalScroll) => {
                            let _ = sender.send((value as f32, 0.0));
                        }
                        _ => {}
                    }
                }
            }
            wl_pointer::Event::Motion { surface_x, surface_y, .. } => {
                if let Some(sender) = &state.mouse_pos_sender {
                    let _ = sender.send((surface_x as f32, surface_y as f32));
                }
            }
            wl_pointer::Event::Enter { surface_x, surface_y, .. } => {
                state.active = true;
                if let Some(sender) = &state.mouse_pos_sender {
                    let _ = sender.send((surface_x as f32, surface_y as f32));
                }
            }
            wl_pointer::Event::Leave { .. } => {
                state.active = false;
            }
            _ => {}
        }
    }
}

// XDG WM Base dispatch
impl Dispatch<XdgWmBase, ()> for WaylandState {
    fn event(
        _: &mut Self,
        xdg_wm_base: &XdgWmBase,
        event: xdg_wm_base::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            xdg_wm_base::Event::Ping { serial } => {
                xdg_wm_base.pong(serial);
            }
            _ => {}
        }
    }
}

// XDG Surface dispatch
impl Dispatch<XdgSurface, ()> for WaylandState {
    fn event(
        state: &mut Self,
        xdg_surface: &XdgSurface,
        event: xdg_surface::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            xdg_surface::Event::Configure { serial } => {
                xdg_surface.ack_configure(serial);
                state.configured = true;
            }
            _ => {}
        }
    }
}

// XDG Toplevel dispatch
impl Dispatch<XdgToplevel, ()> for WaylandState {
    fn event(
        state: &mut Self,
        _: &XdgToplevel,
        event: xdg_toplevel::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            xdg_toplevel::Event::Configure { width, height, .. } => {
                if width > 0 && height > 0 {
                    state.width = width;
                    state.height = height;
                }
            }
            xdg_toplevel::Event::Close => {
                state.closed = true;
            }
            _ => {}
        }
    }
}

// Decoration manager dispatch
impl Dispatch<ZxdgDecorationManagerV1, ()> for WaylandState {
    fn event(
        _: &mut Self,
        _: &ZxdgDecorationManagerV1,
        _: zxdg_decoration_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

// Decoration dispatch
impl Dispatch<ZxdgToplevelDecorationV1, ()> for WaylandState {
    fn event(
        _: &mut Self,
        _: &ZxdgToplevelDecorationV1,
        _: zxdg_toplevel_decoration_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

pub struct Window {
    connection: Connection,
    event_queue: EventQueue<WaylandState>,
    state: WaylandState,
    
    // Input channels
    key_receiver: mpsc::Receiver<(Key, bool)>,
    mouse_receiver: mpsc::Receiver<(MouseButton, bool)>,
    scroll_receiver: mpsc::Receiver<(f32, f32)>,
    mouse_pos_receiver: mpsc::Receiver<(f32, f32)>,
    
    // Window properties
    width: i32,        // Actual window width (scaled)
    height: i32,       // Actual window height (scaled)
    buf_width: i32,    // Original buffer width (unscaled)
    buf_height: i32,   // Original buffer height (unscaled)
    scale_factor: i32, // Scale factor applied to window
    scale_mode: ScaleMode,
    
    // Input state
    keys: [bool; 512],
    key_states: std::collections::HashMap<Key, bool>,
    mouse_buttons: [bool; 8],
    scroll_x: f32,
    scroll_y: f32,
    mouse_x: f32,
    mouse_y: f32,
    
    // Callbacks
    key_handler: KeyHandler,
}

impl Window {
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

    pub fn new(name: &str, width: usize, height: usize, opts: WindowOptions) -> Result<Window> {
        let connection = Connection::connect_to_env()
            .map_err(|e| Error::WindowCreate(format!("Failed to connect to Wayland: {}", e)))?;
        
        let display = connection.display();
        let mut event_queue = connection.new_event_queue();
        let qh = event_queue.handle();
        
        // Apply scale factor to window dimensions like X11 does
        // For Wayland, use reasonable default screen dimensions (1920x1080) for scale calculation
        // since we don't have easy access to actual screen size during initial setup
        let scale_factor = Self::get_scale_factor(width, height, 1920, 1080, opts.scale);
        let scaled_width = width * scale_factor;
        let scaled_height = height * scale_factor;
        
        // Create channels for input events
        let (key_sender, key_receiver) = mpsc::channel();
        let (mouse_sender, mouse_receiver) = mpsc::channel();
        let (scroll_sender, scroll_receiver) = mpsc::channel();
        let (mouse_pos_sender, mouse_pos_receiver) = mpsc::channel();
        
        let mut state = WaylandState::new();
        state.key_sender = Some(key_sender);
        state.mouse_sender = Some(mouse_sender);
        state.scroll_sender = Some(scroll_sender);
        state.mouse_pos_sender = Some(mouse_pos_sender);
        state.width = scaled_width as i32;
        state.height = scaled_height as i32;
        
        // Get registry and bind globals
        let registry = display.get_registry(&qh, ());
        state.registry = Some(registry);
        
        // Initial roundtrip to get globals
        event_queue.roundtrip(&mut state)
            .map_err(|e| Error::WindowCreate(format!("Failed to roundtrip: {}", e)))?;
        
        // Create surface and XDG surface
        if let (Some(compositor), Some(xdg_wm_base)) = (&state.compositor, &state.xdg_wm_base) {
            let surface = compositor.create_surface(&qh, ());
            let xdg_surface = xdg_wm_base.get_xdg_surface(&surface, &qh, ());
            let xdg_toplevel = xdg_surface.get_toplevel(&qh, ());
            
            xdg_toplevel.set_title(name.to_string());
            xdg_toplevel.set_min_size(scaled_width as i32, scaled_height as i32);
            
            // Set up decorations if available
            if let Some(decoration_manager) = &state.decoration_manager {
                let decoration = decoration_manager.get_toplevel_decoration(&xdg_toplevel, &qh, ());
                decoration.set_mode(zxdg_toplevel_decoration_v1::Mode::ServerSide);
                state.decoration = Some(decoration);
            }
            
            state.surface = Some(surface);
            state.xdg_surface = Some(xdg_surface);
            state.xdg_toplevel = Some(xdg_toplevel);
            
            // Surface must be committed after creating XDG surface
            if let Some(surface) = &state.surface {
                surface.commit();
            }
        }
        
        // Wait for initial configure
        while !state.configured {
            event_queue.blocking_dispatch(&mut state)
                .map_err(|e| Error::WindowCreate(format!("Failed to dispatch events: {}", e)))?;
        }
        
        // Additional roundtrip to ensure everything is set up
        event_queue.roundtrip(&mut state)
            .map_err(|e| Error::WindowCreate(format!("Failed to complete setup: {}", e)))?;
        
        Ok(Window {
            connection,
            event_queue,
            state,
            key_receiver,
            mouse_receiver,
            scroll_receiver,
            mouse_pos_receiver,
            width: scaled_width as i32,
            height: scaled_height as i32,
            buf_width: width as i32,
            buf_height: height as i32,
            scale_factor: scale_factor as i32,
            scale_mode: opts.scale_mode,
            keys: [false; 512],
            key_states: HashMap::new(),
            mouse_buttons: [false; 8],
            scroll_x: 0.0,
            scroll_y: 0.0,
            mouse_x: 0.0,
            mouse_y: 0.0,
            key_handler: KeyHandler::new(),
        })
    }
    
    pub fn update_with_buffer(&mut self, buffer: &[u32]) -> Result<()> {
        self.update_with_buffer_stride(buffer, self.buf_width as usize, self.buf_height as usize, self.buf_width as usize)
    }
    
    
    pub fn is_open(&self) -> bool {
        !self.state.closed
    }
    
    pub fn get_keys(&self) -> Vec<Key> {
        let mut keys = Vec::new();
        for (i, &pressed) in self.keys.iter().enumerate() {
            if pressed {
                // Convert index back to Key - this is a simple approximation
                if i < 256 {
                    keys.push(unsafe { std::mem::transmute(i as u8) });
                }
            }
        }
        keys
    }
    
    pub fn get_keys_pressed(&self, _repeat: KeyRepeat) -> Vec<Key> {
        // For now, just return currently pressed keys
        self.get_keys()
    }
    
    pub fn is_key_down(&self, key: Key) -> bool {
        self.key_states.get(&key).copied().unwrap_or(false)
    }
    
    pub fn get_mouse_down(&self, button: MouseButton) -> bool {
        let index = match button {
            MouseButton::Left => 0,
            MouseButton::Right => 1,
            MouseButton::Middle => 2,
        };
        self.mouse_buttons[index]
    }
    
    pub fn get_mouse_pos(&self, _mode: MouseMode) -> Option<(f32, f32)> {
        Some((self.mouse_x, self.mouse_y))
    }
    
    pub fn get_scroll_wheel(&self) -> Option<(f32, f32)> {
        Some((self.scroll_x, self.scroll_y))
    }
    
    pub fn set_cursor_style(&mut self, _cursor: CursorStyle) {
        // TODO: Implement cursor style changes
    }
    
    pub fn get_window_handle(&self) -> *mut c_void {
        if let Some(surface) = &self.state.surface {
            surface as *const _ as *mut c_void
        } else {
            std::ptr::null_mut()
        }
    }
}

impl HasWindowHandle for Window {
    fn window_handle(&self) -> std::result::Result<WindowHandle, HandleError> {
        if let Some(surface) = &self.state.surface {
            let handle = WaylandWindowHandle::new(unsafe {
                NonNull::new_unchecked(surface as *const _ as *mut c_void)
            });
            Ok(unsafe { WindowHandle::borrow_raw(handle.into()) })
        } else {
            Err(HandleError::Unavailable)
        }
    }
}

impl HasDisplayHandle for Window {
    fn display_handle(&self) -> std::result::Result<DisplayHandle, HandleError> {
        let display = self.connection.display();
        let handle = WaylandDisplayHandle::new(unsafe {
            NonNull::new_unchecked(&display as *const _ as *mut c_void)
        });
        Ok(unsafe { DisplayHandle::borrow_raw(handle.into()) })
    }
}

// Additional required implementations for the Window struct
impl Window {
    pub fn set_title(&mut self, title: &str) {
        if let Some(xdg_toplevel) = &self.state.xdg_toplevel {
            xdg_toplevel.set_title(title.to_string());
        }
    }
    
    pub fn set_input_callback(&mut self, callback: Box<dyn InputCallback>) {
        self.key_handler.set_input_callback(callback);
    }
    
    pub fn is_active(&self) -> bool {
        self.state.active
    }
    
    pub fn get_size(&self) -> (usize, usize) {
        (self.width as usize, self.height as usize)
    }
    
    pub fn get_position(&self) -> (isize, isize) {
        // TODO: Implement position tracking
        (0, 0)
    }
    
    pub fn set_position(&mut self, x: isize, y: isize) {
        // TODO: Implement position setting
    }
    
    pub fn topmost(&mut self, topmost: bool) {
        // TODO: Implement topmost functionality
    }
    
    pub fn set_background_color(&mut self, color: u32) {
        // TODO: Implement background color
    }
    
    pub fn add_menu(&mut self, _menu: &Menu) -> MenuHandle {
        // TODO: Implement menu support
        MenuHandle(0)
    }
    
    pub fn get_posix_menus(&self) -> Option<&Vec<UnixMenu>> {
        // TODO: Implement menu support
        None
    }
    
    pub fn remove_menu(&mut self, _handle: MenuHandle) {
        // TODO: Implement menu support
    }
    
    pub fn is_menu_pressed(&mut self) -> Option<usize> {
        // TODO: Implement menu support
        None
    }
    
    pub fn update_with_buffer_stride(&mut self, buffer: &[u32], buf_width: usize, buf_height: usize, _buf_stride: usize) -> Result<()> {
        check_buffer_size(buffer, buf_width, buf_height, buf_width)?;
        
        // Don't override window dimensions with buffer dimensions
        // The window size should be controlled by the compositor, not the buffer size
        
        // Process events first to handle input and buffer releases
        // Use roundtrip every few frames to ensure we read new events
        static mut FRAME_COUNT: u32 = 0;
        unsafe {
            FRAME_COUNT += 1;
            if FRAME_COUNT % 3 == 0 {
                // Every 3rd frame, do a full roundtrip to read new events
                self.event_queue.roundtrip(&mut self.state)
                    .map_err(|e| Error::UpdateFailed(format!("Failed to roundtrip: {}", e)))?;
            } else {
                // Other frames, just dispatch what's already queued
                self.event_queue.dispatch_pending(&mut self.state)
                    .map_err(|e| Error::UpdateFailed(format!("Failed to dispatch events: {}", e)))?;
            }
        }
        
        // Check if the window was resized by the compositor and update our dimensions
        if self.state.width != self.width || self.state.height != self.height {
            self.width = self.state.width;
            self.height = self.state.height;
        }
        
        // Process input events
        while let Ok((key, pressed)) = self.key_receiver.try_recv() {
            self.key_states.insert(key, pressed);
            let key_index = key as usize;
            if key_index < self.keys.len() {
                self.keys[key_index] = pressed;
            }
        }
        
        while let Ok((button, pressed)) = self.mouse_receiver.try_recv() {
            let index = match button {
                MouseButton::Left => 0,
                MouseButton::Right => 1,
                MouseButton::Middle => 2,
            };
            self.mouse_buttons[index] = pressed;
        }
        
        while let Ok((scroll_x, scroll_y)) = self.scroll_receiver.try_recv() {
            self.scroll_x = scroll_x;
            self.scroll_y = scroll_y;
        }
        
        while let Ok((mouse_x, mouse_y)) = self.mouse_pos_receiver.try_recv() {
            self.mouse_x = mouse_x;
            self.mouse_y = mouse_y;
        }
        
        // Check if we have necessary objects and update buffer
        let has_surface_and_shm = self.state.surface.is_some() && self.state.shm.is_some();
        if has_surface_and_shm {
            let qh = self.event_queue.handle();
            
            // Use the actual window dimensions for the buffer
            let window_width = self.width;
            let window_height = self.height;
            
            // Retry buffer allocation if the first attempt fails
            let mut retries = 0;
            while retries < 5 {
                match self.state.buffer_pool.get_buffer(window_width, window_height, 
                                                        self.state.shm.as_ref().unwrap(), &qh) {
                    Ok(buffer_obj) => {
                        let buffer_data = buffer_obj.get_data();
                        
                        // Create intermediate RGBA buffer for scaling operations
                        let window_width_usize = window_width as usize;
                        let window_height_usize = window_height as usize;
                        let mut draw_buffer: Vec<u32> = vec![0; window_width_usize * window_height_usize];
                        
                        // Apply scaling based on scale mode (like X11's raw_blit_buffer)
                        unsafe {
                            match self.scale_mode {
                                ScaleMode::Stretch => {
                                    image_resize_linear(
                                        draw_buffer.as_mut_ptr(),
                                        window_width as u32,
                                        window_height as u32,
                                        buffer.as_ptr(),
                                        buf_width as u32,
                                        buf_height as u32,
                                        buf_width as u32,
                                    );
                                }
                                ScaleMode::AspectRatioStretch => {
                                    image_resize_linear_aspect_fill(
                                        draw_buffer.as_mut_ptr(),
                                        window_width as u32,
                                        window_height as u32,
                                        buffer.as_ptr(),
                                        buf_width as u32,
                                        buf_height as u32,
                                        buf_width as u32,
                                        0, // background color
                                    );
                                }
                                ScaleMode::Center => {
                                    image_center(
                                        draw_buffer.as_mut_ptr(),
                                        window_width as u32,
                                        window_height as u32,
                                        buffer.as_ptr(),
                                        buf_width as u32,
                                        buf_height as u32,
                                        buf_width as u32,
                                        0, // background color
                                    );
                                }
                                ScaleMode::UpperLeft => {
                                    image_upper_left(
                                        draw_buffer.as_mut_ptr(),
                                        window_width as u32,
                                        window_height as u32,
                                        buffer.as_ptr(),
                                        buf_width as u32,
                                        buf_height as u32,
                                        buf_width as u32,
                                        0, // background color
                                    );
                                }
                            }
                        }
                        
                        // Convert scaled RGBA buffer to BGRA format for Wayland
                        for (i, &pixel) in draw_buffer.iter().enumerate() {
                            let offset = i * 4;
                            if offset + 3 < buffer_data.len() {
                                // Extract RGB components from scaled buffer (0x00RRGGBB)
                                let r = ((pixel >> 16) & 0xFF) as u8;
                                let g = ((pixel >> 8) & 0xFF) as u8;
                                let b = (pixel & 0xFF) as u8;
                                
                                // Write as ARGB8888 (little endian: BGRA)
                                buffer_data[offset] = b;     // Blue
                                buffer_data[offset + 1] = g; // Green
                                buffer_data[offset + 2] = r; // Red
                                buffer_data[offset + 3] = 0xFF; // Alpha (fully opaque)
                            }
                        }
                        
                        // Attach and commit
                        if let Some(wl_buffer) = &buffer_obj.buffer {
                            let surface = self.state.surface.as_ref().unwrap();
                            surface.attach(Some(wl_buffer), 0, 0);
                            surface.damage_buffer(0, 0, window_width, window_height);
                            surface.commit();
                            buffer_obj.busy = true;
                            
                            // Ensure compositor processes the frame
                            let _ = self.connection.flush();
                        }
                        break;
                    }
                    Err(_) => {
                        // Try to process more events to release buffers
                        let _ = self.event_queue.dispatch_pending(&mut self.state);
                        retries += 1;
                        
                        // If we're out of retries, just continue - we'll try again next frame
                        if retries >= 5 {
                            break;
                        }
                    }
                }
            }
        }
        
        // Additional dispatch to ensure frame is sent to compositor
        let _ = self.event_queue.dispatch_pending(&mut self.state);
        
        Ok(())
    }
    
    pub fn update(&mut self) {
        // Process events
        let _ = self.event_queue.dispatch_pending(&mut self.state);
        
        // Process input events
        while let Ok((key, pressed)) = self.key_receiver.try_recv() {
            self.key_states.insert(key, pressed);
            let key_index = key as usize;
            if key_index < self.keys.len() {
                self.keys[key_index] = pressed;
            }
        }
        
        while let Ok((button, pressed)) = self.mouse_receiver.try_recv() {
            let index = match button {
                MouseButton::Left => 0,
                MouseButton::Right => 1,
                MouseButton::Middle => 2,
            };
            self.mouse_buttons[index] = pressed;
        }
        
        while let Ok((scroll_x, scroll_y)) = self.scroll_receiver.try_recv() {
            self.scroll_x = scroll_x;
            self.scroll_y = scroll_y;
        }
        
        while let Ok((mouse_x, mouse_y)) = self.mouse_pos_receiver.try_recv() {
            self.mouse_x = mouse_x;
            self.mouse_y = mouse_y;
        }
    }
    
    pub fn set_rate(&mut self, _rate: Option<Duration>) {
        // TODO: Implement rate limiting
    }
    
    pub fn update_rate(&mut self) {
        // TODO: Implement update rate
    }
    
    pub fn set_key_repeat_delay(&mut self, _delay: f32) {
        // TODO: Implement key repeat delay
    }
    
    pub fn set_key_repeat_rate(&mut self, _rate: f32) {
        // TODO: Implement key repeat rate
    }
    
    pub fn is_key_pressed(&self, _key: Key, _repeat: KeyRepeat) -> bool {
        // TODO: Implement key pressed detection
        false
    }
    
    pub fn is_key_released(&self, _key: Key) -> bool {
        // TODO: Implement key released detection
        false
    }
    
    pub fn set_cursor_visibility(&mut self, _visibility: bool) {
        // TODO: Implement cursor visibility
    }
    
    pub fn get_unscaled_mouse_pos(&self, _mode: MouseMode) -> Option<(f32, f32)> {
        // TODO: Implement unscaled mouse position
        Some((self.mouse_x, self.mouse_y))
    }
    
    pub fn limit_update_rate(&mut self, _rate: Option<Duration>) {
        // TODO: Implement update rate limiting
    }
    
    pub fn get_keys_released(&self) -> Vec<Key> {
        // TODO: Implement key released detection
        Vec::new()
    }
    
    pub fn set_target_fps(&mut self, _fps: u64) {
        // TODO: Implement target FPS setting
    }
}