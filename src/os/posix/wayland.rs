use std::{
    convert::TryFrom,
    ffi::c_void,
    fs::File,
    io::{Seek, SeekFrom, Write},
    os::unix::io::{AsFd, AsRawFd, OwnedFd},
    ptr::NonNull,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use super::common::{
    image_center, image_resize_linear, image_resize_linear_aspect_fill, image_upper_left, Menu,
};
use crate::{
    check_buffer_size, key_handler::KeyHandler, rate::UpdateRate, CursorStyle, Error,
    InputCallback, Key, KeyRepeat, MenuHandle, MouseButton, MouseMode, Result, Scale, ScaleMode,
    UnixMenu, UseGPU, WindowOptions,
};

use super::gl;
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle, WindowHandle,
};
use wayland_client::{
    backend::WaylandError,
    delegate_noop,
    globals::{registry_queue_init, GlobalList, GlobalListContents},
    protocol::{
        wl_buffer::{self, WlBuffer},
        wl_compositor::WlCompositor,
        wl_keyboard::{self, KeymapFormat, WlKeyboard},
        wl_pointer::{self, WlPointer},
        wl_registry::WlRegistry,
        wl_seat::WlSeat,
        wl_shm::{Format, WlShm},
        wl_shm_pool::WlShmPool,
        wl_surface::WlSurface,
    },
    Connection, Dispatch, EventQueue, Proxy, QueueHandle, WEnum,
};
use wayland_protocols::xdg::{
    decoration::zv1::client::{
        zxdg_decoration_manager_v1::ZxdgDecorationManagerV1,
        zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1,
    },
    shell::client::{
        xdg_surface::{self, XdgSurface},
        xdg_toplevel::{self, XdgToplevel},
        xdg_wm_base::{self, XdgWmBase},
    },
};

use super::xkb_ffi;
#[cfg(feature = "dlopen")]
use super::xkb_ffi::XKBCOMMON_HANDLE as XKBH;
#[cfg(not(feature = "dlopen"))]
use super::xkb_ffi::*;

const KEY_XKB_OFFSET: u32 = 8;
/// Covers the evdev keycode range (`KEY_MAX` 767) plus `KEY_XKB_OFFSET`.
const KEYCODE_SLOTS: usize = 776;
const KEY_MOUSE_BTN1: u32 = 272;
const KEY_MOUSE_BTN2: u32 = 273;
const KEY_MOUSE_BTN3: u32 = 274;
const KEY_MOUSE_BTN8: u32 = 275;
const KEY_MOUSE_BTN9: u32 = 276;

/// A surface size with both axes positive and an ARGB byte count that fits the
/// `i32` a `wl_shm_pool` size is sent as. `new` is the only constructor, so
/// nothing downstream re-validates.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct SurfaceSize {
    width: i32,
    height: i32,
    bytes: i32,
}

impl SurfaceSize {
    fn new(width: i32, height: i32) -> Option<Self> {
        if width <= 0 || height <= 0 {
            return None;
        }
        let bytes = i64::from(width)
            .checked_mul(i64::from(height))
            .and_then(|px| px.checked_mul(std::mem::size_of::<u32>() as i64))
            .and_then(|bytes| i32::try_from(bytes).ok())?;
        Some(Self {
            width,
            height,
            bytes,
        })
    }

    fn scaled(width: usize, height: usize, scale: i32) -> Option<Self> {
        let axis = |v: usize| i32::try_from(v).ok()?.checked_mul(scale);
        Self::new(axis(width)?, axis(height)?)
    }

    /// A zero axis in an `xdg_toplevel::configure` means the compositor is
    /// leaving that dimension to us, so the current value stands.
    fn reconfigured(self, width: i32, height: i32) -> Option<Self> {
        Self::new(
            if width == 0 { self.width } else { width },
            if height == 0 { self.height } else { height },
        )
    }

    /// Cannot overflow: the whole buffer already fits an i32.
    fn stride(self) -> i32 {
        self.width * std::mem::size_of::<u32>() as i32
    }

    fn pixels(self) -> usize {
        self.width as usize * self.height as usize
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Slot {
    Reuse(usize),
    Grow,
    /// Every buffer is still held and the pool is at its cap; drop the frame.
    Wait,
}

/// Split out of `get_buffer` so the policy is testable without a compositor.
fn select_slot(released: &[bool], max: usize) -> Slot {
    match released.iter().position(|&r| r) {
        Some(idx) => Slot::Reuse(idx),
        None if released.len() < max => Slot::Grow,
        None => Slot::Wait,
    }
}

/// Past this, a frame with no released buffer to write into is dropped rather
/// than presented. A well-behaved compositor settles the pool at two.
const MAX_POOLED_BUFFERS: usize = 4;

struct Buffer {
    file: File,
    pool: WlShmPool,
    pool_size: i32,
    buffer: WlBuffer,
    /// Set by the compositor's `wl_buffer::Release`, cleared when we attach the
    /// buffer to the surface. Only a released buffer may be written to.
    released: Arc<AtomicBool>,
    fb_size: SurfaceSize,
}

struct BufferPool {
    pool: Vec<Buffer>,
    shm: WlShm,
    format: Format,
}

impl BufferPool {
    fn new(shm: WlShm, format: Format) -> Self {
        Self {
            pool: Vec::new(),
            shm,
            format,
        }
    }

    fn create_shm_buffer(
        shm_pool: &WlShmPool,
        size: SurfaceSize,
        format: Format,
        qh: &QueueHandle<WaylandState>,
    ) -> (WlBuffer, Arc<AtomicBool>) {
        // Doubles as the buffer's dispatch user data, so a release lands on it
        // directly without searching the pool.
        let released = Arc::new(AtomicBool::new(true));

        let buffer = shm_pool.create_buffer(
            0,
            size.width,
            size.height,
            size.stride(),
            format,
            qh,
            released.clone(),
        );

        (buffer, released)
    }

    /// `Ok(None)` means every buffer is still held and the pool is at its cap;
    /// the caller must drop the frame rather than write to a held buffer.
    fn get_buffer(
        &mut self,
        size: SurfaceSize,
        qh: &QueueHandle<WaylandState>,
    ) -> std::io::Result<Option<(&mut File, &WlBuffer)>> {
        let size_bytes = size.bytes;

        let released: Vec<bool> = self
            .pool
            .iter()
            .map(|e| e.released.load(Ordering::Acquire))
            .collect();

        let idx = match select_slot(&released, MAX_POOLED_BUFFERS) {
            Slot::Reuse(idx) => idx,
            Slot::Wait => return Ok(None),
            Slot::Grow => {
                let file = tempfile::tempfile()?;
                // The compositor mmaps the pool on creation; a mapping past
                // EOF is a SIGBUS.
                file.set_len(size_bytes as u64)?;
                let shm_pool = self.shm.create_pool(file.as_fd(), size_bytes, qh, ());
                let (buffer, released) = Self::create_shm_buffer(&shm_pool, size, self.format, qh);

                self.pool.push(Buffer {
                    file,
                    pool: shm_pool,
                    pool_size: size_bytes,
                    buffer,
                    released,
                    fb_size: size,
                });

                self.pool.len() - 1
            }
        };

        let entry = &mut self.pool[idx];

        // A wl_shm_pool may only grow, and the file has to lead it.
        if size_bytes > entry.pool_size {
            entry.file.set_len(size_bytes as u64)?;
            entry.pool.resize(size_bytes);
            entry.pool_size = size_bytes;
        }

        if entry.fb_size != size {
            let (buffer, released) = Self::create_shm_buffer(&entry.pool, size, self.format, qh);
            let old = std::mem::replace(&mut entry.buffer, buffer);
            old.destroy();
            entry.released = released;
            entry.fb_size = size;
        }

        Ok(Some((&mut entry.file, &entry.buffer)))
    }

    fn mark_attached(&self, buffer: &WlBuffer) {
        if let Some(entry) = self.pool.iter().find(|e| &e.buffer == buffer) {
            entry.released.store(false, Ordering::Release);
        }
    }
}

/// Event callbacks write here; `Window::update` drains it each frame.
#[derive(Default)]
struct WaylandState {
    kb_events: Vec<wl_keyboard::Event>,
    pt_events: Vec<wl_pointer::Event>,

    /// Latest unacknowledged `xdg_surface::Configure` serial. Acknowledged when
    /// the next frame is presented, so the ack is paired with actual content.
    xdg_config: Option<u32>,
    /// Size from an `xdg_toplevel::Configure` that no `xdg_surface::Configure`
    /// has closed yet, so it is not part of a configure transaction and must
    /// not be applied.
    pending_resolution: Option<(i32, i32)>,
    /// Size of the most recent complete configure transaction.
    resolution: Option<(i32, i32)>,
    closed: bool,
}

impl Dispatch<WlRegistry, GlobalListContents> for WaylandState {
    fn event(
        _: &mut Self,
        _: &WlRegistry,
        _: <WlRegistry as Proxy>::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlBuffer, Arc<AtomicBool>> for WaylandState {
    fn event(
        _: &mut Self,
        _: &WlBuffer,
        event: wl_buffer::Event,
        released: &Arc<AtomicBool>,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_buffer::Event::Release = event {
            released.store(true, Ordering::Release);
        }
    }
}

impl Dispatch<WlKeyboard, ()> for WaylandState {
    fn event(
        state: &mut Self,
        _: &WlKeyboard,
        event: wl_keyboard::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        state.kb_events.push(event);
    }
}

impl Dispatch<WlPointer, ()> for WaylandState {
    fn event(
        state: &mut Self,
        _: &WlPointer,
        event: wl_pointer::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        state.pt_events.push(event);
    }
}

impl Dispatch<XdgWmBase, ()> for WaylandState {
    fn event(
        _: &mut Self,
        xdg_wm_base: &XdgWmBase,
        event: xdg_wm_base::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_wm_base::Event::Ping { serial } = event {
            xdg_wm_base.pong(serial);
        }
    }
}

impl Dispatch<XdgSurface, ()> for WaylandState {
    fn event(
        state: &mut Self,
        _: &XdgSurface,
        event: xdg_surface::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        // Ends a configure sequence, so it is what seals the size the
        // preceding xdg_toplevel::Configure proposed; publishing that earlier
        // lets a frame carry one transaction's size under another's serial.
        if let xdg_surface::Event::Configure { serial } = event {
            if let Some(size) = state.pending_resolution.take() {
                state.resolution = Some(size);
            }
            state.xdg_config = Some(serial);
        }
    }
}

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
                state.pending_resolution = Some((width, height));
            }
            xdg_toplevel::Event::Close => {
                state.closed = true;
            }
            _ => {}
        }
    }
}

delegate_noop!(WaylandState: ignore WlCompositor);
delegate_noop!(WaylandState: ignore WlSurface);
delegate_noop!(WaylandState: ignore WlShm);
delegate_noop!(WaylandState: ignore WlShmPool);
delegate_noop!(WaylandState: ignore WlSeat);
delegate_noop!(WaylandState: ignore ZxdgDecorationManagerV1);
delegate_noop!(WaylandState: ignore ZxdgToplevelDecorationV1);

struct DisplayInfo {
    conn: Connection,
    event_queue: EventQueue<WaylandState>,
    qh: QueueHandle<WaylandState>,
    state: WaylandState,
    surface: WlSurface,
    xdg_surface: XdgSurface,
    toplevel: XdgToplevel,
    cursor: wayland_cursor::CursorTheme,
    cursor_surface: WlSurface,
    buf_pool: BufferPool,
}

impl DisplayInfo {
    /// Accepts the size of the surface to be created, whether or not the alpha channel will be
    /// rendered, and whether or not server-side decorations will be used.
    fn new(
        size: SurfaceSize,
        alpha: bool,
        decorate: bool,
    ) -> Result<(Self, WlKeyboard, WlPointer)> {
        let conn = Connection::connect_to_env().map_err(|e| {
            Error::WindowCreate(format!("Failed to connect to the Wayland display: {:?}", e))
        })?;

        let (globals, mut event_queue): (GlobalList, EventQueue<WaylandState>) =
            registry_queue_init(&conn).map_err(|e| {
                Error::WindowCreate(format!("Failed to retrieve the Wayland globals: {:?}", e))
            })?;
        let qh = event_queue.handle();
        let mut state = WaylandState::default();

        // Version 5 is required for scroll events
        let seat: WlSeat = globals
            .bind(&qh, 5..=5, ())
            .map_err(|e| Error::WindowCreate(format!("Failed to retrieve the WlSeat: {:?}", e)))?;

        let keyboard = seat.get_keyboard(&qh, ());
        let pointer = seat.get_pointer(&qh, ());

        let compositor: WlCompositor = globals.bind(&qh, 4..=4, ()).map_err(|e| {
            Error::WindowCreate(format!("Failed to retrieve the compositor: {:?}", e))
        })?;
        let shm: WlShm = globals
            .bind(&qh, 1..=1, ())
            .map_err(|e| Error::WindowCreate(format!("Failed to create shared memory: {:?}", e)))?;

        let surface = compositor.create_surface(&qh, ());

        let format = if alpha {
            Format::Argb8888
        } else {
            Format::Xrgb8888
        };

        let mut buf_pool = BufferPool::new(shm.clone(), format);
        let (file, buffer) = buf_pool
            .get_buffer(size, &qh)
            .map_err(|e| Error::WindowCreate(format!("Failed to retrieve Buffer: {:?}", e)))?
            .ok_or_else(|| Error::WindowCreate("Failed to allocate a Buffer".to_owned()))?;
        let buffer = buffer.clone();

        // Add a black canvas into the framebuffer
        let frame: Vec<u32> = vec![0xFF00_0000; size.pixels()];
        let slice = unsafe {
            std::slice::from_raw_parts(
                frame.as_ptr() as *const u8,
                frame.len() * std::mem::size_of::<u32>(),
            )
        };
        file.write_all(slice)
            .map_err(|e| Error::WindowCreate(format!("Io Error: {:?}", e)))?;
        file.flush()
            .map_err(|e| Error::WindowCreate(format!("Io Error: {:?}", e)))?;

        let xdg_wm_base: XdgWmBase = globals.bind(&qh, 1..=1, ()).map_err(|e| {
            Error::WindowCreate(format!("Failed to retrieve the XdgWmBase: {:?}", e))
        })?;

        let xdg_surface = xdg_wm_base.get_xdg_surface(&surface, &qh, ());
        let xdg_toplevel = xdg_surface.get_toplevel(&qh, ());

        if decorate {
            match globals.bind::<ZxdgDecorationManagerV1, _, _>(&qh, 1..=1, ()) {
                Ok(decorations) => {
                    decorations.get_toplevel_decoration(&xdg_toplevel, &qh, ());
                    decorations.destroy();
                }
                Err(e) => println!("Failed to create server-side surface decoration: {:?}", e),
            }
        }

        surface.commit();
        event_queue
            .roundtrip(&mut state)
            .map_err(|e| Error::WindowCreate(format!("Roundtrip failed: {:?}", e)))?;

        // The first configure has to be acknowledged before any content is
        // attached; later ones are acknowledged as frames are presented.
        if let Some(serial) = state.xdg_config.take() {
            xdg_surface.ack_configure(serial);
        }

        // Give the buffer to the surface and commit
        surface.attach(Some(&buffer), 0, 0);
        buf_pool.mark_attached(&buffer);
        surface.damage(0, 0, i32::MAX, i32::MAX);
        surface.commit();

        let cursor = wayland_cursor::CursorTheme::load(&conn, shm.clone(), 16)
            .map_err(|e| Error::WindowCreate(format!("Failed to load cursor theme: {:?}", e)))?;
        let cursor_surface = compositor.create_surface(&qh, ());

        Ok((
            Self {
                conn,
                event_queue,
                qh,
                state,
                surface,
                xdg_surface,
                toplevel: xdg_toplevel,
                cursor,
                cursor_surface,
                buf_pool,
            },
            keyboard,
            pointer,
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
    fn set_no_resize(&self, size: SurfaceSize) {
        self.toplevel.set_max_size(size.width, size.height);
        self.toplevel.set_min_size(size.width, size.height);
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

    /// Acknowledge the pending configure, if any.
    ///
    /// xdg-shell requires this before the commit that consumes the configure.
    /// `eglSwapBuffers` is such a commit, so the GL path has to call this too
    /// -- `update_framebuffer` is not on that path.
    fn ack_pending_configure(&mut self) {
        if let Some(serial) = self.state.xdg_config.take() {
            self.xdg_surface.ack_configure(serial);
        }
    }

    fn update_framebuffer(&mut self, buffer: &[u32], size: SurfaceSize) -> std::io::Result<()> {
        // Every buffer is still held by the compositor: drop this frame
        // rather than write into one it may be scanning out. The previous
        // frame stays on screen and the next call presents normally.
        let (file, buf) = match self.buf_pool.get_buffer(size, &self.qh)? {
            Some(entry) => entry,
            None => return Ok(()),
        };
        let buf = buf.clone();

        file.seek(SeekFrom::Start(0))?;

        let slice = unsafe {
            std::slice::from_raw_parts(buffer.as_ptr() as *const u8, std::mem::size_of_val(buffer))
        };

        file.write_all(slice)?;
        file.flush()?;

        self.ack_pending_configure();

        self.surface.attach(Some(&buf), 0, 0);
        self.buf_pool.mark_attached(&buf);
        self.surface.damage(0, 0, i32::MAX, i32::MAX);
        self.surface.commit();

        Ok(())
    }
}

/// Where the GL path stands for this window. The context is built on the
/// first `update_with_buffer` rather than at window creation, so a caller that
/// only wants the `wl_surface` through `raw-window-handle` -- to drive it with
/// wgpu or glow -- never gets a competing EGL surface on it.
enum GlPath {
    /// No attempt made yet.
    Untried,
    Active(Box<gl::WaylandGl>),
    /// A frame exceeded `GL_MAX_TEXTURE_SIZE`. The context had to go -- shm
    /// cannot attach to a surface EGL owns -- but nothing was lost, so GL is
    /// rebuilt as soon as a buffer fits `max` again. Gating on the remembered
    /// limit is what keeps an app that stays oversized from paying for a fresh
    /// EGL context every frame.
    TooLarge {
        max: i32,
    },
    /// Tried and failed, or explicitly turned off. Never retried: a driver
    /// that could not give us a context once will not on the next frame, and
    /// retrying would stall every update.
    Unavailable,
}

pub struct Window {
    display: DisplayInfo,

    size: SurfaceSize,

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

    /// The `Key` each held keycode resolved to when it went down. A release
    /// clears what its own press set: the active layout group can change
    /// while a key is held, which would otherwise resolve the release to a
    /// different `Key` and leave the pressed one stuck.
    held: Box<[Option<Key>; KEYCODE_SLOTS]>,

    update_rate: UpdateRate,
    menu_counter: MenuHandle,
    menus: Vec<UnixMenu>,
    _keyboard: WlKeyboard,
    pointer: WlPointer,
    resizable: bool,
    // Temporary buffer, only used by the software path
    buffer: Vec<u32>,
    pointer_visibility: bool,

    transparent: bool,
    gl: GlPath,
    /// Size the `wl_egl_window` was last told about, so a resize is pushed to
    /// EGL only when it actually changed.
    gl_size: SurfaceSize,
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

        let size = SurfaceSize::scaled(width, height, scale).ok_or_else(|| {
            Error::WindowCreate(format!(
                "{}x{} at {}x scale is not a usable window size",
                width, height, scale
            ))
        })?;

        let (display, keyboard, pointer) =
            DisplayInfo::new(size, opts.transparency, !opts.borderless || opts.none)?;

        if opts.title {
            display.set_title(name);
        }
        if !opts.resize || opts.none {
            display.set_no_resize(size);
        }

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

        let window = Self {
            display,

            size,

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
            held: Box::new([None; KEYCODE_SLOTS]),

            update_rate: UpdateRate::new(),
            menu_counter: MenuHandle(0),
            menus: Vec::new(),
            _keyboard: keyboard,
            pointer,
            resizable: opts.resize && !opts.none,
            buffer: Vec::with_capacity(width * height * scale as usize * scale as usize),
            pointer_visibility: true,

            transparent: opts.transparency,
            gl: match opts.use_gpu {
                UseGPU::Disabled => GlPath::Unavailable,
                UseGPU::Auto => GlPath::Untried,
            },
            gl_size: size,
        };

        Ok(window)
    }

    /// Build the GL context for this window's surface.
    fn init_gl(&mut self) -> std::result::Result<(), gl::GlError> {
        let wl_display = self.display.conn.backend().display_ptr() as *mut c_void;
        let wl_surface = self.display.surface.id().as_ptr() as *mut c_void;

        let context = unsafe {
            gl::WaylandGl::new(
                wl_display,
                wl_surface,
                self.size.width,
                self.size.height,
                self.transparent,
            )?
        };

        self.gl_size = self.size;
        self.gl = GlPath::Active(Box::new(context));
        Ok(())
    }

    /// Present through GL, reporting whether it happened. `false` means the
    /// caller must run the software path.
    ///
    /// Once EGL owns the surface the shm path cannot attach to it any more, so
    /// anything that stops GL working has to destroy the context (and with it
    /// the `wl_egl_window`) before returning `false`. Assigning over
    /// `self.gl` is what does that -- the old `GlPath::Active` drops in place.
    fn try_present_gl(
        &mut self,
        buffer: &[u32],
        buf_width: usize,
        buf_height: usize,
        buf_stride: usize,
    ) -> bool {
        match self.gl {
            GlPath::Unavailable => return false,
            // Still too big for the context we would rebuild; do not pay for
            // one just to hit the same limit.
            GlPath::TooLarge { max } => {
                if buf_width as i32 > max || buf_height as i32 > max {
                    return false;
                }
            }
            GlPath::Untried | GlPath::Active(_) => {}
        }

        if matches!(self.gl, GlPath::Untried | GlPath::TooLarge { .. }) && self.init_gl().is_err() {
            self.gl = GlPath::Unavailable;
            return false;
        }

        let (size, scale_mode, bg_color) = (self.size, self.scale_mode, self.bg_color);
        let resized = self.gl_size != size;

        // The `wl_egl_window` carries its own size and will not pick up a
        // configure on its own.
        if resized {
            match &mut self.gl {
                GlPath::Active(context) => context.resize(size.width, size.height),
                _ => return false,
            }
        }

        // `eglSwapBuffers` inside `present` commits the surface, and xdg-shell
        // requires the configure to be acknowledged before that commit -- not
        // after it.
        self.display.ack_pending_configure();

        let context = match &mut self.gl {
            GlPath::Active(context) => context,
            _ => return false,
        };

        let result = unsafe {
            context.context().present(
                buffer,
                buf_width as i32,
                buf_height as i32,
                buf_stride as i32,
                size.width,
                size.height,
                scale_mode,
                bg_color,
            )
        };

        if let Err(e) = result {
            // Nothing reached the screen, and shm cannot attach while EGL owns
            // the surface -- so the context goes either way. Only an oversized
            // buffer is worth coming back from: the context was never touched,
            // and the next frame may well fit.
            self.gl = match e {
                gl::GlError::TextureTooLarge { max, .. } => GlPath::TooLarge { max },
                _ => GlPath::Unavailable,
            };
            return false;
        }

        if resized {
            self.gl_size = size;
        }

        true
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
        self.display.surface.id().as_ptr() as *mut c_void
    }

    #[inline]
    pub fn get_size(&self) -> (usize, usize) {
        (self.size.width as usize, self.size.height as usize)
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
            self.size.width as f32,
            self.size.height as f32,
        )
    }

    #[inline]
    pub fn get_mouse_down(&self, button: MouseButton) -> bool {
        match button {
            MouseButton::Left => self.buttons[0],
            MouseButton::Right => self.buttons[1],
            MouseButton::Middle => self.buttons[2],
            MouseButton::Back => self.buttons[3],
            MouseButton::Forward => self.buttons[4],
        }
    }

    #[inline]
    pub fn get_unscaled_mouse_pos(&self, mode: MouseMode) -> Option<(f32, f32)> {
        mode.get_pos(
            self.mouse_x,
            self.mouse_y,
            1.0,
            self.size.width as f32,
            self.size.height as f32,
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
            .set_geometry((x as i32, y as i32), (self.size.width, self.size.height));
    }

    #[inline]
    pub fn get_position(&self) -> (isize, isize) {
        // Wayland deliberately does not tell a client where its surface is on
        // screen, so there is nothing truthful to report here.
        (0, 0)
    }

    #[inline]
    pub fn set_rate(&mut self, rate: Option<Duration>) {
        self.update_rate.set_rate(rate);
    }

    #[inline]
    pub fn get_delta_time(&self) -> Option<Duration> {
        self.update_rate.get_delta_time()
    }

    #[inline]
    pub fn set_key_repeat_rate(&mut self, rate: f32) {
        self.key_handler.set_key_repeat_rate(rate);
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

    /// Pumps the socket. Returns `false` once the connection is gone, which
    /// the caller turns into a close request: a compositor that disconnects
    /// mid-run should end the window, not abort the process.
    fn try_dispatch_events(&mut self) -> bool {
        let display = &mut self.display;

        if let Err(e) = display.event_queue.flush() {
            if !is_would_block(&e) {
                eprintln!("Error while trying to flush the wayland socket: {:?}", e);
                return false;
            }
        }

        if let Err(e) = display.event_queue.dispatch_pending(&mut display.state) {
            eprintln!("Wayland event dispatch failed: {:?}", e);
            return false;
        }

        if let Some(guard) = display.event_queue.prepare_read() {
            if let Err(e) = guard.read() {
                if !is_would_block(&e) {
                    eprintln!(
                        "Error while trying to read from the wayland socket: {:?}",
                        e
                    );
                    return false;
                }
            }
        }

        if let Err(e) = display.event_queue.dispatch_pending(&mut display.state) {
            eprintln!("Wayland event dispatch failed: {:?}", e);
            return false;
        }

        true
    }

    pub fn update(&mut self) {
        if !self.try_dispatch_events() {
            self.should_close = true;
        }

        if let Some((width, height)) = self.display.state.resolution.take() {
            if self.resizable {
                match self.size.reconfigured(width, height) {
                    Some(size) => self.size = size,
                    None => eprintln!("Ignoring unusable configure size {}x{}", width, height),
                }
            }
        }
        if self.display.state.closed {
            self.should_close = true;
        }

        // Snapshot before applying this batch, advance after it -- see the
        // two methods' docs. This backend is the one that applies platform key
        // events inline, so it is the one that has to split the phases.
        self.key_handler.snapshot_prev();

        for event in std::mem::take(&mut self.display.state.kb_events) {
            use wayland_client::protocol::wl_keyboard::Event;

            match event {
                Event::Keymap { format, fd, size } => {
                    match Self::handle_keymap(self.xkb_context, format, fd, size) {
                        Ok(keymap) => {
                            // Drop the previous keymap/state; the compositor
                            // sends a fresh keymap whenever the layout changes.
                            unsafe {
                                ffi_dispatch!(XKBH, xkb_state_unref, self.xkb_state);
                                ffi_dispatch!(XKBH, xkb_keymap_unref, self.xkb_keymap);
                            }
                            self.xkb_keymap = keymap;
                            self.xkb_state = unsafe { ffi_dispatch!(XKBH, xkb_state_new, keymap) };
                        }
                        Err(e) => {
                            eprintln!("Failed to load the compositor keymap: {:?}", e);
                            // A keycode means nothing without the layout it
                            // came with, so translation stops here rather
                            // than continuing against the withdrawn one --
                            // which also means no release will arrive for a
                            // key held across this point.
                            unsafe {
                                ffi_dispatch!(XKBH, xkb_state_unref, self.xkb_state);
                                ffi_dispatch!(XKBH, xkb_keymap_unref, self.xkb_keymap);
                            }
                            self.xkb_state = std::ptr::null_mut();
                            self.xkb_keymap = std::ptr::null_mut();
                            self.release_held_keys();
                        }
                    }
                }
                Event::Enter { .. } => {
                    self.active = true;
                }
                Event::Leave { .. } => {
                    self.active = false;
                }
                Event::Key { key, state, .. } => {
                    // `key` is compositor-supplied, so the offset to xkb's
                    // numbering must not wrap into a valid-looking keycode.
                    let key = match key.checked_add(KEY_XKB_OFFSET) {
                        Some(key) => key,
                        None => continue,
                    };

                    if !self.xkb_state.is_null() && !self.xkb_keymap.is_null() {
                        if let WEnum::Value(state) = state {
                            Self::handle_key(
                                self.xkb_keymap,
                                self.xkb_state,
                                key,
                                state,
                                &mut self.key_handler,
                                &mut self.held,
                            );
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

        self.key_handler.advance_durations();

        self.scroll_x = 0.;
        self.scroll_y = 0.;

        for event in std::mem::take(&mut self.display.state.pt_events) {
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

                    self.display
                        .update_cursor(Self::decode_cursor(self.prev_cursor))
                        .unwrap();
                    self.apply_cursor(serial);
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

                    // An unknown state is not a release; treating it as one
                    // would clear a button still being held.
                    let pressed = match state {
                        WEnum::Value(ButtonState::Pressed) => true,
                        WEnum::Value(ButtonState::Released) => false,
                        _ => continue,
                    };

                    match button {
                        // Left mouse button
                        KEY_MOUSE_BTN1 => self.buttons[0] = pressed,
                        // Right mouse button
                        KEY_MOUSE_BTN2 => self.buttons[1] = pressed,
                        // Middle mouse button
                        KEY_MOUSE_BTN3 => self.buttons[2] = pressed,
                        // Back mouse button
                        KEY_MOUSE_BTN8 => self.buttons[3] = pressed,
                        // Forward mouse button
                        KEY_MOUSE_BTN9 => self.buttons[4] = pressed,
                        _ => (
                            // TODO: handle more mouse buttons (see: linux/input-event-codes.h from
                            // the Linux kernel)
                        ),
                    }

                    self.apply_cursor(serial);
                }
                Event::Axis { axis, value, .. } => {
                    use wayland_client::protocol::wl_pointer::Axis;

                    match axis {
                        WEnum::Value(Axis::VerticalScroll) => self.scroll_y = value as f32,
                        WEnum::Value(Axis::HorizontalScroll) => self.scroll_x = value as f32,
                        _ => {}
                    }
                }
                Event::AxisStop { axis, .. } => {
                    use wayland_client::protocol::wl_pointer::Axis;

                    match axis {
                        WEnum::Value(Axis::VerticalScroll) => self.scroll_y = 0.,
                        WEnum::Value(Axis::HorizontalScroll) => self.scroll_x = 0.,
                        _ => {}
                    }
                }
                Event::Leave { serial, .. } => {
                    self.apply_cursor(serial);
                }
                _ => {}
            }
        }
    }

    /// Clears every key this window still believes is down.
    fn release_held_keys(&mut self) {
        for slot in self.held.iter_mut() {
            if let Some(key) = slot.take() {
                self.key_handler.set_key_state(key, false);
            }
        }
    }

    /// Points the pointer at our cursor surface, or hides it entirely when the
    /// cursor has been turned off.
    #[inline]
    fn apply_cursor(&self, serial: u32) {
        let surface = if self.pointer_visibility {
            Some(&self.display.cursor_surface)
        } else {
            None
        };
        self.pointer.set_cursor(serial, surface, 0, 0);
    }

    /// The keysym `key` produces at shift level 0 of the currently active
    /// layout, or `None` if it produces nothing there.
    ///
    /// Level 0 is shift-invariant, unlike `xkb_state_key_get_one_sym`, which
    /// resolves through the active shift level.
    ///
    /// SAFETY: `keymap` and `keymap_state` must be live and non-null; the
    /// caller checks both before dispatching a key event.
    fn keysym_at_level_0(
        keymap: *mut xkb_ffi::xkb_keymap,
        keymap_state: *mut xkb_ffi::xkb_state,
        key: u32,
    ) -> Option<u32> {
        let layout = unsafe { ffi_dispatch!(XKBH, xkb_state_key_get_layout, keymap_state, key) };
        if layout == xkb_ffi::XKB_LAYOUT_INVALID {
            return None;
        }

        let mut syms: *const xkb_ffi::xkb_keysym_t = std::ptr::null();
        let count = unsafe {
            ffi_dispatch!(
                XKBH,
                xkb_keymap_key_get_syms_by_level,
                keymap,
                key,
                layout,
                0,
                &mut syms
            )
        };
        if count <= 0 || syms.is_null() {
            return None;
        }

        // `syms` points into the keymap and stays valid as long as it does.
        match unsafe { *syms } {
            0 => None,
            sym => Some(sym),
        }
    }

    fn handle_key(
        keymap: *mut xkb_ffi::xkb_keymap,
        keymap_state: *mut xkb_ffi::xkb_state,
        key: u32,
        state: wl_keyboard::KeyState,
        key_handler: &mut KeyHandler,
        held: &mut [Option<Key>; KEYCODE_SLOTS],
    ) {
        let is_down = state == wl_keyboard::KeyState::Pressed;

        if is_down {
            // Character input, unlike `Key`, *should* follow the active shift
            // level and layout, so it keeps using the state-resolved keysym.
            let sym = unsafe { ffi_dispatch!(XKBH, xkb_state_key_get_one_sym, keymap_state, key) };
            if sym != 0 {
                // Taken from GLFW
                let code_point = unsafe { ffi_dispatch!(XKBH, xkb_keysym_to_utf32, sym) };
                if !(code_point < 32 || (code_point > 126 && code_point < 160)) {
                    if let Some(ref mut callback) = key_handler.key_callback {
                        callback.add_char(code_point);
                    }
                }
            }
        }

        // A release clears what its own press recorded: the layout group can
        // change while a key is held, which would otherwise resolve the
        // release to a different `Key` and leave the pressed one stuck.
        if !is_down {
            if let Some(pressed) = held.get_mut(key as usize).and_then(Option::take) {
                key_handler.set_key_state(pressed, false);
                return;
            }
        }

        if let Some(key_xkb) = Self::keysym_at_level_0(keymap, keymap_state, key) {
            use super::xkb_keysyms as key;

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
                key::XKB_KEY_KP_Delete => Key::NumPadDot,
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

            if is_down {
                if let Some(slot) = held.get_mut(key as usize) {
                    *slot = Some(key_i);
                }
            }

            key_handler.set_key_state(key_i, is_down);
        }
    }

    fn handle_keymap(
        context: *mut xkb_ffi::xkb_context,
        keymap: WEnum<KeymapFormat>,
        fd: OwnedFd,
        len: u32,
    ) -> Result<*mut xkb_ffi::xkb_keymap> {
        match keymap {
            WEnum::Value(KeymapFormat::XkbV1) => {
                unsafe {
                    // mmap does not check `len` against the file, so a short
                    // fd would map fine and then fault when xkbcommon read it.
                    let mut stat: libc::stat = std::mem::zeroed();
                    if libc::fstat(fd.as_raw_fd(), &mut stat) != 0 {
                        return Err(Error::WindowCreate(format!(
                            "Could not stat the keymap from the compositor ({})",
                            std::io::Error::last_os_error()
                        )));
                    }
                    let file_len = stat.st_size;
                    if len == 0 || file_len < 0 || (file_len as u64) < u64::from(len) {
                        return Err(Error::WindowCreate(format!(
                            "Compositor sent a {} byte keymap backed by {} bytes",
                            len, file_len
                        )));
                    }

                    // The file descriptor must be memory-mapped (with MAP_PRIVATE).
                    let addr = libc::mmap(
                        std::ptr::null_mut(),
                        len as usize,
                        libc::PROT_READ,
                        libc::MAP_PRIVATE,
                        fd.as_raw_fd(),
                        0,
                    );
                    if addr == libc::MAP_FAILED {
                        return Err(Error::WindowCreate(format!(
                            "Could not mmap keymap from compositor ({})",
                            std::io::Error::last_os_error()
                        )));
                    }

                    // `len` counts the terminator, and the parser below scans
                    // for one; without it the scan runs past the mapping.
                    if *(addr as *const u8).add(len as usize - 1) != 0 {
                        libc::munmap(addr, len as usize);
                        return Err(Error::WindowCreate(
                            "Compositor sent an unterminated keymap.".to_owned(),
                        ));
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
            other => Err(Error::WindowCreate(format!(
                "Only XKB keymaps are supported, compositor sent {:?}",
                other
            ))),
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
        let result = (|| {
            check_buffer_size(buffer, buf_width, buf_height, buf_stride)?;

            // The GPU scales from the buffer you passed; the software path
            // below has to materialise a window-sized copy first.
            if self.try_present_gl(buffer, buf_width, buf_height, buf_stride) {
                return Ok(());
            }

            unsafe { self.scale_buffer(buffer, buf_width, buf_height, buf_stride) };
            self.display
                .update_framebuffer(&self.buffer, self.size)
                .map_err(|e| Error::UpdateFailed(format!("Error updating framebuffer: {:?}", e)))
        })();

        // `update()` is also where input events and the key-repeat timer
        // advance -- it must run whether or not the present above
        // succeeded. A caller whose `Err` handling does not itself call
        // `self.update()` (the natural shape, since that branch usually just
        // logs and retries) would otherwise leave `key_handler`'s duration
        // tracking frozen for a cycle: the next successful poll then sees an
        // already-held key's timer still at its initial `0.0` and reports it
        // as freshly pressed again, a spurious duplicate keystroke with no
        // visible cause tying it to the failed present.
        self.update();

        result
    }

    unsafe fn scale_buffer(
        &mut self,
        buffer: &[u32],
        buf_width: usize,
        buf_height: usize,
        buf_stride: usize,
    ) {
        self.buffer.resize(self.size.pixels(), 0);

        match self.scale_mode {
            ScaleMode::Stretch => {
                image_resize_linear(
                    self.buffer.as_mut_ptr(),
                    self.size.width as u32,
                    self.size.height as u32,
                    buffer.as_ptr(),
                    buf_width as u32,
                    buf_height as u32,
                    buf_stride as u32,
                );
            }

            ScaleMode::AspectRatioStretch => {
                image_resize_linear_aspect_fill(
                    self.buffer.as_mut_ptr(),
                    self.size.width as u32,
                    self.size.height as u32,
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
                    self.size.width as u32,
                    self.size.height as u32,
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
                    self.size.width as u32,
                    self.size.height as u32,
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

#[inline]
fn is_would_block(e: &WaylandError) -> bool {
    matches!(e, WaylandError::Io(io) if io.kind() == std::io::ErrorKind::WouldBlock)
}

impl HasWindowHandle for Window {
    fn window_handle(&self) -> std::result::Result<WindowHandle<'_>, HandleError> {
        let surface = self.display.surface.id().as_ptr();
        let display_surface =
            NonNull::new(surface as *mut c_void).ok_or(HandleError::Unavailable)?;

        let handle = WaylandWindowHandle::new(display_surface);
        let raw_handle = RawWindowHandle::Wayland(handle);
        unsafe { Ok(WindowHandle::borrow_raw(raw_handle)) }
    }
}

impl HasDisplayHandle for Window {
    fn display_handle(&self) -> std::result::Result<DisplayHandle<'_>, HandleError> {
        let raw_display = self.display.conn.backend().display_ptr();
        let display = NonNull::new(raw_display as *mut c_void).ok_or(HandleError::Unavailable)?;
        let handle = WaylandDisplayHandle::new(display);
        let raw_handle = RawDisplayHandle::Wayland(handle);
        unsafe { Ok(DisplayHandle::borrow_raw(raw_handle)) }
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        // Before anything else: the GL context holds an EGLSurface built on a
        // `wl_egl_window` wrapping `display`'s `wl_surface`, and `display`
        // owns the connection. Field drop order would run this after
        // `display`, handing freed native handles to the driver.
        self.gl = GlPath::Unavailable;

        unsafe {
            ffi_dispatch!(XKBH, xkb_state_unref, self.xkb_state);
            ffi_dispatch!(XKBH, xkb_keymap_unref, self.xkb_keymap);
            ffi_dispatch!(XKBH, xkb_context_unref, self.xkb_context);
        }
    }
}

#[cfg(test)]
mod key_level_tests {
    use super::*;
    use std::ffi::CString;

    /// A minimal keymap that reproduces the level structure the `Key` mapping
    /// has to survive, without depending on the host's installed layouts:
    ///
    /// - `<AE02>` is `2` unshifted and `quotedbl` shifted, exactly as the
    ///   German layout maps that physical key. On a US layout the shifted
    ///   level would be `at` instead -- the point is that *which* keysym the
    ///   shifted level yields is layout-specific, so `Key` must not be derived
    ///   from it.
    /// - `<AD01>` is the usual `q`/`Q` alphabetic pair.
    /// - `<KP1>` is `KP_End` at the base level and `KP_1` with NumLock, which
    ///   is how every real keymap describes the numpad.
    const KEYMAP: &str = r#"xkb_keymap {
xkb_keycodes "minifb_test" {
    minimum = 8;
    maximum = 255;
    <AE02> = 11;
    <AD01> = 24;
    <LFSH> = 50;
    <NMLK> = 77;
    <KP1>  = 87;
    <KPDL> = 91;
};
xkb_types "minifb_test" {
    virtual_modifiers NumLock;
    type "ONE_LEVEL"  { modifiers = none; level_name[1] = "Any"; };
    type "TWO_LEVEL"  {
        modifiers = Shift;
        map[Shift] = 2;
        level_name[1] = "Base"; level_name[2] = "Shift";
    };
    type "ALPHABETIC" {
        modifiers = Shift;
        map[Shift] = 2;
        level_name[1] = "Base"; level_name[2] = "Caps";
    };
    type "KEYPAD" {
        modifiers = Shift+NumLock;
        map[None] = 1;
        map[Shift] = 2;
        map[NumLock] = 2;
        map[Shift+NumLock] = 1;
        level_name[1] = "Base"; level_name[2] = "Number";
    };
};
xkb_compat "minifb_test" {
    virtual_modifiers NumLock;
    interpret Num_Lock+AnyOf(all) {
        virtualModifier = NumLock;
        action = LockMods(modifiers = NumLock);
    };
    interpret Shift_L+AnyOf(all) { action = SetMods(modifiers = Shift); };
};
xkb_symbols "minifb_test" {
    key <AE02> {
        type[Group1] = "TWO_LEVEL", symbols[Group1] = [ 2, quotedbl ],
        type[Group2] = "TWO_LEVEL", symbols[Group2] = [ bracketleft, braceleft ]
    };
    key <AD01> { type = "ALPHABETIC", [ q, Q ] };
    key <LFSH> { type = "ONE_LEVEL",  [ Shift_L ] };
    key <NMLK> { type = "ONE_LEVEL",  [ Num_Lock ] };
    key <KP1>  { type = "KEYPAD",     [ KP_End, KP_1 ] };
    key <KPDL> { type = "KEYPAD",     [ KP_Delete, KP_Decimal ] };
    modifier_map Shift { <LFSH> };
    modifier_map Mod2  { <NMLK> };
};
};"#;

    // XKB keycodes are evdev codes + KEY_XKB_OFFSET, which is what
    // `handle_key` receives from the compositor.
    const AE02: u32 = 3 + KEY_XKB_OFFSET;
    const AD01: u32 = 16 + KEY_XKB_OFFSET;
    const KP1: u32 = 79 + KEY_XKB_OFFSET;
    const KPDL: u32 = 83 + KEY_XKB_OFFSET;
    /// In range for the fixture's keycode block, but not one of its keys.
    const UNDECLARED: u32 = 200;

    const SHIFT: u32 = 1 << 0; // core Shift
    const NUMLOCK: u32 = 1 << 4; // Mod2

    const SYM_2: u32 = 0x0032;
    const SYM_QUOTEDBL: u32 = 0x0022;
    const SYM_KP_1: u32 = 0xffb1;
    const SYM_KP_DECIMAL: u32 = 0xffae;

    struct Keymap {
        context: *mut xkb_ffi::xkb_context,
        keymap: *mut xkb_ffi::xkb_keymap,
        state: *mut xkb_ffi::xkb_state,
        held: Box<[Option<Key>; KEYCODE_SLOTS]>,
    }

    impl Keymap {
        fn new() -> Keymap {
            let text = CString::new(KEYMAP).unwrap();
            unsafe {
                let context = ffi_dispatch!(
                    XKBH,
                    xkb_context_new,
                    xkb_ffi::xkb_context_flags::XKB_CONTEXT_NO_FLAGS
                );
                assert!(!context.is_null(), "failed to create xkb context");
                let keymap = ffi_dispatch!(
                    XKBH,
                    xkb_keymap_new_from_string,
                    context,
                    text.as_ptr(),
                    xkb_ffi::xkb_keymap_format::XKB_KEYMAP_FORMAT_TEXT_V1,
                    xkb_ffi::xkb_keymap_compile_flags::XKB_KEYMAP_COMPILE_NO_FLAGS
                );
                assert!(!keymap.is_null(), "test keymap failed to compile");
                let state = ffi_dispatch!(XKBH, xkb_state_new, keymap);
                assert!(!state.is_null(), "failed to create xkb state");
                Keymap {
                    context,
                    keymap,
                    state,
                    held: Box::new([None; KEYCODE_SLOTS]),
                }
            }
        }

        /// Stands in for the compositor's `Modifiers` event.
        fn set_mods(&self, depressed: u32, locked: u32) {
            unsafe {
                ffi_dispatch!(
                    XKBH,
                    xkb_state_update_mask,
                    self.state,
                    depressed,
                    0,
                    locked,
                    0,
                    0,
                    0
                )
            };
        }

        /// Stands in for the `group` field of the compositor's Modifiers event.
        fn set_group(&self, group: u32) {
            unsafe {
                ffi_dispatch!(
                    XKBH,
                    xkb_state_update_mask,
                    self.state,
                    0,
                    0,
                    0,
                    0,
                    0,
                    group
                )
            };
        }

        fn active_sym(&self, key: u32) -> u32 {
            unsafe { ffi_dispatch!(XKBH, xkb_state_key_get_one_sym, self.state, key) }
        }

        fn press(&mut self, keys: &mut KeyHandler, key: u32) {
            Window::handle_key(
                self.keymap,
                self.state,
                key,
                wl_keyboard::KeyState::Pressed,
                keys,
                &mut self.held,
            );
        }

        fn release(&mut self, keys: &mut KeyHandler, key: u32) {
            Window::handle_key(
                self.keymap,
                self.state,
                key,
                wl_keyboard::KeyState::Released,
                keys,
                &mut self.held,
            );
        }
    }

    impl Drop for Keymap {
        fn drop(&mut self) {
            unsafe {
                ffi_dispatch!(XKBH, xkb_state_unref, self.state);
                ffi_dispatch!(XKBH, xkb_keymap_unref, self.keymap);
                ffi_dispatch!(XKBH, xkb_context_unref, self.context);
            }
        }
    }

    /// The fixture is only meaningful if Shift really does change the keysym.
    #[test]
    fn fixture_has_a_layout_specific_shifted_level() {
        let mut km = Keymap::new();

        km.set_mods(0, 0);
        assert_eq!(km.active_sym(AE02), SYM_2);

        km.set_mods(SHIFT, 0);
        assert_eq!(km.active_sym(AE02), SYM_QUOTEDBL);

        km.set_mods(0, NUMLOCK);
        assert_eq!(km.active_sym(KP1), SYM_KP_1);
    }

    /// Ordinary typing rollover: a key goes down with Shift held and comes
    /// back up after Shift is already released, so press and release resolve
    /// through different shift levels. Whatever the press set, the release
    /// has to clear -- otherwise the key reads as held forever.
    #[test]
    fn shift_rollover_leaves_no_key_held() {
        let mut km = Keymap::new();
        let mut keys = KeyHandler::new();

        km.set_mods(SHIFT, 0);
        km.press(&mut keys, AE02);
        let held = keys.get_keys();
        assert!(!held.is_empty(), "shifted press was dropped entirely");

        km.set_mods(0, 0);
        km.release(&mut keys, AE02);

        assert_eq!(
            keys.get_keys(),
            Vec::new(),
            "release did not clear {held:?}"
        );
    }

    /// `Key` names a physical key, so the shift level must not change which
    /// one a given keycode reports.
    #[test]
    fn shift_does_not_change_the_reported_key() {
        // `<AE02>` is the discriminating case: its shifted level is
        // layout-specific (`quotedbl` here, `at` on a US layout), so a
        // mapping taken from the active level reports a different `Key`
        // under Shift. `<AD01>`'s `q`/`Q` pair does not discriminate.
        for (keycode, expected) in [(AE02, Key::Key2), (AD01, Key::Q)] {
            let mut km = Keymap::new();

            let mut unshifted = KeyHandler::new();
            km.set_mods(0, 0);
            km.press(&mut unshifted, keycode);

            let mut shifted = KeyHandler::new();
            km.set_mods(SHIFT, 0);
            km.press(&mut shifted, keycode);

            assert_eq!(unshifted.get_keys(), vec![expected]);
            assert_eq!(shifted.get_keys(), vec![expected]);
        }
    }

    /// `add_char` is the one path that *should* follow the active shift
    /// level: the character typed depends on the layout, even though the
    /// `Key` does not.
    #[test]
    fn char_input_follows_the_active_shift_level() {
        #[derive(Default)]
        struct Recorder(std::rc::Rc<std::cell::RefCell<Vec<u32>>>);

        impl InputCallback for Recorder {
            fn add_char(&mut self, uni_char: u32) {
                self.0.borrow_mut().push(uni_char);
            }
        }

        let mut km = Keymap::new();
        let chars = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let mut keys = KeyHandler::new();
        keys.set_input_callback(Box::new(Recorder(chars.clone())));

        km.set_mods(SHIFT, 0);
        km.press(&mut keys, AE02);
        km.set_mods(0, 0);
        km.press(&mut keys, AE02);

        assert_eq!(*chars.borrow(), vec![SYM_QUOTEDBL, SYM_2]);
    }

    /// A keycode the keymap does not describe has no level-0 symbol; it must
    /// be ignored rather than resolving to some other key.
    #[test]
    fn undeclared_keycode_is_ignored() {
        let mut km = Keymap::new();
        let mut keys = KeyHandler::new();

        assert_eq!(km.active_sym(UNDECLARED), 0);
        km.press(&mut keys, UNDECLARED);

        assert_eq!(keys.get_keys(), Vec::new());
    }

    /// The compositor can switch layout group while a key is held, which
    /// resolves the release through a different group than the press. The
    /// release must still clear the key the press set.
    #[test]
    fn group_change_while_held_leaves_no_key_held() {
        let mut km = Keymap::new();
        let mut keys = KeyHandler::new();

        km.set_group(0);
        km.press(&mut keys, AE02);
        assert_eq!(keys.get_keys(), vec![Key::Key2]);

        km.set_group(1);
        assert_ne!(
            km.active_sym(AE02),
            SYM_2,
            "fixture's second group must differ"
        );
        km.release(&mut keys, AE02);

        assert_eq!(keys.get_keys(), Vec::new());
    }

    /// The keypad decimal key is `KP_Delete` at level 0 and `KP_Decimal` only
    /// at the NumLock level, so a level-0 lookup has to recognise the former.
    #[test]
    fn keypad_decimal_registers() {
        let mut km = Keymap::new();
        km.set_mods(0, NUMLOCK);
        assert_eq!(km.active_sym(KPDL), SYM_KP_DECIMAL);

        let mut keys = KeyHandler::new();
        km.press(&mut keys, KPDL);

        assert_eq!(keys.get_keys(), vec![Key::NumPadDot]);
    }

    /// With NumLock on -- the normal state -- the numpad resolves to `KP_1`
    /// rather than `KP_End`, and must still report as a numpad key.
    #[test]
    fn numpad_registers_with_numlock_on() {
        let mut km = Keymap::new();
        km.set_mods(0, NUMLOCK);

        let mut keys = KeyHandler::new();
        km.press(&mut keys, KP1);

        assert_eq!(keys.get_keys(), vec![Key::NumPad1]);
    }
}

#[cfg(test)]
mod buffer_pool_tests {
    use super::{select_slot, Slot, SurfaceSize, MAX_POOLED_BUFFERS};

    fn size(w: i32, h: i32) -> SurfaceSize {
        SurfaceSize::new(w, h).unwrap()
    }

    #[test]
    fn byte_count_is_width_times_height_times_four() {
        assert_eq!(size(640, 480).bytes, 640 * 480 * 4);
        assert_eq!(size(640, 480).stride(), 640 * 4);
        assert_eq!(size(640, 480).pixels(), 640 * 480);
    }

    #[test]
    fn non_positive_dimensions_are_rejected() {
        assert_eq!(SurfaceSize::new(0, 480), None);
        assert_eq!(SurfaceSize::new(640, 0), None);
        assert_eq!(SurfaceSize::new(-1, 480), None);
        assert_eq!(SurfaceSize::new(640, -1), None);
    }

    /// `i32::MAX * i32::MAX * 4` exceeds `i64::MAX`, so the intermediate
    /// product has to be checked, not just the final narrowing to i32.
    #[test]
    fn the_largest_dimensions_do_not_overflow_the_check() {
        assert_eq!(SurfaceSize::new(i32::MAX, i32::MAX), None);
    }

    #[test]
    fn a_total_larger_than_i32_is_rejected() {
        // Dimensions are individually fine; the byte count is not.
        assert_eq!(SurfaceSize::new(40_000, 40_000), None);
    }

    #[test]
    fn scale_multiply_is_checked() {
        assert_eq!(SurfaceSize::scaled(320, 240, 2), SurfaceSize::new(640, 480));
        assert_eq!(SurfaceSize::scaled(usize::MAX, 240, 1), None);
        assert_eq!(SurfaceSize::scaled(i32::MAX as usize, 1, 32), None);
    }

    #[test]
    fn a_zero_configure_axis_keeps_the_current_value() {
        let current = size(640, 480);
        assert_eq!(current.reconfigured(0, 600), SurfaceSize::new(640, 600));
        assert_eq!(current.reconfigured(800, 0), SurfaceSize::new(800, 480));
        assert_eq!(current.reconfigured(0, 0), Some(current));
        assert_eq!(current.reconfigured(800, 600), SurfaceSize::new(800, 600));
    }

    #[test]
    fn an_unusable_configure_is_rejected() {
        let current = size(640, 480);
        assert_eq!(current.reconfigured(-1, 600), None);
        assert_eq!(current.reconfigured(40_000, 40_000), None);
    }

    #[test]
    fn empty_pool_grows() {
        assert_eq!(select_slot(&[], MAX_POOLED_BUFFERS), Slot::Grow);
    }

    #[test]
    fn released_buffer_is_reused() {
        assert_eq!(
            select_slot(&[false, true], MAX_POOLED_BUFFERS),
            Slot::Reuse(1)
        );
    }

    #[test]
    fn lowest_released_index_wins() {
        assert_eq!(
            select_slot(&[false, true, true], MAX_POOLED_BUFFERS),
            Slot::Reuse(1)
        );
    }

    #[test]
    fn all_busy_under_cap_grows() {
        assert_eq!(select_slot(&[false, false], MAX_POOLED_BUFFERS), Slot::Grow);
    }

    /// At the cap with nothing released there is no safe buffer to write to,
    /// so the frame is dropped rather than scribbled into a held buffer.
    #[test]
    fn all_busy_at_cap_waits() {
        let busy = [false; MAX_POOLED_BUFFERS];
        assert_eq!(select_slot(&busy, MAX_POOLED_BUFFERS), Slot::Wait);
    }

    /// A released buffer must still win at the cap, rather than dropping a
    /// frame that had somewhere safe to go.
    #[test]
    fn release_at_cap_is_preferred_over_waiting() {
        let mut slots = [false; MAX_POOLED_BUFFERS];
        slots[MAX_POOLED_BUFFERS - 1] = true;
        assert_eq!(
            select_slot(&slots, MAX_POOLED_BUFFERS),
            Slot::Reuse(MAX_POOLED_BUFFERS - 1)
        );
    }
}
