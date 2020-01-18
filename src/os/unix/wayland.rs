use crate::{CursorStyle, MenuHandle, MenuItem, MenuItemHandle, UnixMenu, UnixMenuItem};
use crate::{InputCallback, Key, KeyRepeat, MouseButton, MouseMode, Scale, ScaleMode, WindowOptions};
use crate::rate::UpdateRate;
use crate::key_handler::KeyHandler;

use wayland_client::protocol::{wl_display::WlDisplay, wl_compositor::WlCompositor, wl_shm::{WlShm, Format}, wl_shm_pool::WlShmPool, wl_buffer::WlBuffer};
use wayland_client::{EventQueue, ProtocolError, ConnectError, GlobalError, GlobalManager};
use wayland_client::{Main, Attached};
use wayland_protocols::xdg_shell::client::xdg_wm_base::XdgWmBase;

pub enum WlError{
	Io(std::io::Error),
	Prot(ProtocolError),
	Global(GlobalError),
	Connect(ConnectError)
}

impl From<ConnectError> for WlError{
	fn from(e: ConnectError) -> Self{
		WlError::Connect(e)
	}
}

impl From<std::io::Error> for WlError{
	fn from(e: std::io::Error) -> Self{
		WlError::Io(e)
	}
}

impl From<ProtocolError> for WlError{
	fn from(e: ProtocolError) -> Self{
		WlError::Prot(e)
	}
}

impl From<GlobalError> for WlError{
	fn from(e: GlobalError) -> Self{
		WlError::Global(e)
	}
}


pub struct DisplayInfo{
	display: wayland_client::Display,
	wl_display: Attached<WlDisplay>,
	comp: Main<WlCompositor>,
	base: Main<XdgWmBase>,
	shm: Main<WlShm>,
	shm_pool: Main<WlShmPool>,
	buf: Main<WlBuffer>,
	event_queue: EventQueue,
	fd: std::fs::File
}

impl DisplayInfo{
	pub fn new(size: (usize, usize)) -> Result<Self, WlError>{
		use std::os::unix::io::AsRawFd;

		let display = wayland_client::Display::connect_to_env()?;
		let mut event_q = display.create_event_queue();
		let tkn = event_q.get_token();
		let wl_display = (*display).clone().attach(tkn);
		let global_man = GlobalManager::new(&wl_display);

		event_q.sync_roundtrip(|_, _|{ unreachable!() })?;


		let list = global_man.list();
		let comp = global_man.instantiate_exact::<WlCompositor>(1)?;
		let shm = global_man.instantiate_exact::<WlShm>(1)?;
		let surface = comp.create_surface();
		let tmp_f = tempfile::tempfile()?;
		let shm_pool = shm.create_pool(tmp_f.as_raw_fd(), size.0 as i32*size.1 as i32*4);
		let buffer = shm_pool.create_buffer(0, size.0 as i32, size.1 as i32, size.0 as i32*4, Format::Argb8888);
		let xdg_wm_base = global_man.instantiate_exact::<XdgWmBase>(1)?;


		Ok(Self{
			display,
			wl_display,
			comp,
			base: xdg_wm_base,
			shm_pool,
			shm,
			buf: buffer,
			event_queue: event_q,
			fd: tmp_f
		})
	}
}


pub struct Window{
	display: DisplayInfo,

	width: u32,
	height: u32,
	
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

	key_handler: KeyHandler,
	update_rate: UpdateRate,
	menu_counter: MenuHandle,
	menus: Vec<UnixMenu>
}


impl Window{
	pub fn new(name: &str, width: usize, height: usize, opts: WindowOptions) -> Result<Self, WlError>{
		let dsp = DisplayInfo::new((width, height))?;
	
		unimplemented!()
	}
}

