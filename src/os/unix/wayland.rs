use crate::{CursorStyle, MenuHandle, MenuItem, MenuItemHandle, UnixMenu, UnixMenuItem};
use crate::{InputCallback, Key, KeyRepeat, MouseButton, MouseMode, Scale, ScaleMode, WindowOptions};
use crate::{Error, Result};
use crate::rate::UpdateRate;
use crate::key_handler::KeyHandler;

use wayland_client::protocol::{wl_display::WlDisplay, wl_compositor::WlCompositor, wl_shm::{WlShm, Format}, wl_shm_pool::WlShmPool, wl_buffer::WlBuffer, wl_surface::WlSurface};
use wayland_client::{EventQueue, ProtocolError, ConnectError, GlobalError, GlobalManager};
use wayland_client::{Main, Attached};
use wayland_protocols::xdg_shell::client::{xdg_wm_base::XdgWmBase, xdg_surface::XdgSurface, xdg_toplevel::XdgToplevel};
use byteorder::WriteBytesExt;
use byteorder::NativeEndian;
use std::io::Write;

pub struct DisplayInfo{
	display: wayland_client::Display,
	wl_display: Attached<WlDisplay>,
	comp: Main<WlCompositor>,
	base: Main<XdgWmBase>,
	surface: Main<WlSurface>,
	xdg_surface: Main<XdgSurface>,
	toplevel: Main<XdgToplevel>,
	shm: Main<WlShm>,
	shm_pool: Main<WlShmPool>,
	buf: Main<WlBuffer>,
	event_queue: EventQueue,
	fd: std::fs::File
}

impl DisplayInfo{
	pub fn new(size: (usize, usize)) -> Result<Self>{
		use std::os::unix::io::AsRawFd;
		
		//Get the wayland display
		let display = wayland_client::Display::connect_to_env().map_err(|e| Error::WindowCreate(format!("Failed connecting to the Wayland Display: {:?}", e)))?;
		let mut event_q = display.create_event_queue();
		let tkn = event_q.get_token();
		//Access internal WlDisplay with a token
		let wl_display = (*display).clone().attach(tkn);
		let global_man = GlobalManager::new(&wl_display);

		//wait the wayland server to process all events
		event_q.sync_roundtrip(|_, _|{ unreachable!() }).map_err(|e| Error::WindowCreate(format!("Roundtrip failed: {:?}", e)))?;


		let list = global_man.list();
		//retrieve some types from globals
		let comp = global_man.instantiate_exact::<WlCompositor>(1).map_err(|e| Error::WindowCreate(format!("Failed retrieving the compositor: {:?}", e)))?;
		let shm = global_man.instantiate_exact::<WlShm>(1).map_err(|e| Error::WindowCreate(format!("Failed creating the shared memory: {:?}", e)))?;
		let surface = comp.create_surface();
		//temporary file used as framebuffer
		let mut tmp_f = tempfile::tempfile().map_err(|e| Error::WindowCreate(format!("Failed creating the temporary file: {:?}", e)))?;

		//Add a black canvas into the framebuffer
		for i in 0..(size.0 * size.1){
			let _ = tmp_f.write_u32::<NativeEndian>(0xFF000000);
		}
		let _ = tmp_f.flush();

		//create a shared memory
		let shm_pool = shm.create_pool(tmp_f.as_raw_fd(), size.0 as i32*size.1 as i32*4);
		let buffer = shm_pool.create_buffer(0, size.0 as i32, size.1 as i32, size.0 as i32*4, Format::Argb8888);
		
		let xdg_wm_base = global_man.instantiate_exact::<XdgWmBase>(1).map_err(|e| Error::WindowCreate(format!("Failed retrieving the XdgWmBase: {:?}", e)))?;
		
		//Ping Pong
		xdg_wm_base.assign_mono(|xdg_wm_base, event|{
			use wayland_protocols::xdg_shell::client::xdg_wm_base::Event;

			if let Event::Ping{serial} = event{
				xdg_wm_base.pong(serial);
			}
		});

		let xdg_surface = xdg_wm_base.get_xdg_surface(&surface);
		let _surface = surface.clone();
		//Ping Pong
		xdg_surface.assign_mono(move |xdg_surface, event|{
			use wayland_protocols::xdg_shell::client::xdg_surface::Event;

			if let Event::Configure{serial} = event{
				xdg_surface.ack_configure(serial);
				_surface.commit();
			}
		});
		//Assigns the toplevel role and commit
		let _xdg_toplevel = xdg_surface.get_toplevel();
		surface.commit();

		event_q.sync_roundtrip(|_, _|{}).map_err(|e| Error::WindowCreate(format!("Roundtrip failed: {:?}", e)))?;

		//give the surface the buffer and commit
		surface.attach(Some(&buffer), 0, 0);
		surface.commit();

		event_q.sync_roundtrip(|_, _|{}).map_err(|e| Error::WindowCreate(format!("Roundtrip failed: {:?}", e)))?;


		Ok(Self{
			display,
			wl_display,
			comp,
			base: xdg_wm_base,
			surface,
			xdg_surface,
			toplevel: _xdg_toplevel,
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

	key_handler: KeyHandler,
	update_rate: UpdateRate,
	menu_counter: MenuHandle,
	menus: Vec<UnixMenu>
}


impl Window{
	pub fn new(name: &str, width: usize, height: usize, opts: WindowOptions) -> Result<Self>{
		let dsp = DisplayInfo::new((width, height))?;

		//unimplemented!();

		Ok(Self{
			display: dsp,

			width: width as i32,
			height: height as i32,
	
			//TODO
			scale: 0,
			bg_color: 0,
			scale_mode: opts.scale_mode,

			mouse_x: 0.,
			mouse_y: 0.,
			scroll_x: 0.,
			scroll_y: 0.,
			buttons: [0; 3],
			prev_cursor: CursorStyle::Arrow,

			should_close: false,

			key_handler: KeyHandler::new(),
			update_rate: UpdateRate::new(),
			menu_counter: MenuHandle(0),
			menus: Vec::new()
		})
	}
}

