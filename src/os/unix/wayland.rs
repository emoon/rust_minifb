use crate::{CursorStyle, MenuHandle, MenuItem, MenuItemHandle, UnixMenu, UnixMenuItem};
use crate::{InputCallback, Key, KeyRepeat, MouseButton, MouseMode, Scale, ScaleMode, WindowOptions};
use crate::{Error, Result};
use crate::rate::UpdateRate;
use crate::key_handler::KeyHandler;

use wayland_client::protocol::{wl_display::WlDisplay, wl_compositor::WlCompositor, wl_shm::{WlShm, Format}, wl_shm_pool::WlShmPool, wl_buffer::WlBuffer, wl_surface::WlSurface, wl_seat::WlSeat, wl_keyboard::WlKeyboard, wl_pointer::WlPointer};
use wayland_client::{EventQueue, GlobalManager};
use wayland_client::{Main, Attached};
use wayland_protocols::xdg_shell::client::{xdg_wm_base::XdgWmBase, xdg_surface::XdgSurface, xdg_toplevel::XdgToplevel};
use byteorder::{WriteBytesExt, NativeEndian};
use std::io::Write;
use std::ffi::c_void;


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
	fd: std::fs::File,
	seat: Main<WlSeat>,
	pointer: Option<Main<WlPointer>>,
	keyboard: Option<Main<WlKeyboard>>

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

		
		let seat = global_man.instantiate_exact::<WlSeat>(1).map_err(|e| Error::WindowCreate(format!("Failed retrieving the WlSeat: {:?}", e)))?;
		use std::sync::atomic::{AtomicBool, Ordering};
		use std::sync::Arc;
		
		let keyboard_fl = Arc::new(AtomicBool::new(false));
		let pointer_fl = Arc::new(AtomicBool::new(false));

		{
			let keyboard_fl = keyboard_fl.clone();
			let pointer_fl = pointer_fl.clone();
		
			//check pointer and mouse capability
			seat.assign_mono(move |seat, event|{
				use wayland_client::protocol::wl_seat::{Event, Capability};

				if let Event::Capabilities{capabilities} = event{
					if !pointer_fl.load(Ordering::SeqCst) && capabilities.contains(Capability::Pointer){
						pointer_fl.store(true, Ordering::SeqCst);
					}
					if !keyboard_fl.load(Ordering::SeqCst) && capabilities.contains(Capability::Keyboard){
						keyboard_fl.store(true, Ordering::SeqCst);
					}
				}
			});
		}

		event_q.sync_roundtrip(|_, _|{}).map_err(|e| Error::WindowCreate(format!("Roundtrip failed: {:?}", e)))?;

		//retrieve keyboard and pointer
		let keyboard = if keyboard_fl.load(Ordering::SeqCst){
			Some(seat.get_keyboard())
		}
		else{
			None
		};

		let pointer = if pointer_fl.load(Ordering::SeqCst){
			Some(seat.get_pointer())
		}
		else{
			None
		};

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
			fd: tmp_f,
			seat,
			keyboard,	
			pointer,
		})
	}

	fn set_geometry(&self, pos: (i32, i32), size: (i32, i32)){
		self.xdg_surface.set_window_geometry(pos.0, pos.1, size.0, size.1);
	}

	fn set_title(&self, title: &str){
		self.toplevel.set_title(title.to_owned());
	}
	fn set_no_resize(&self, size: (i32, i32)){
		self.toplevel.set_max_size(size.0, size.1);
		self.toplevel.set_min_size(size.0, size.1);
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

		if opts.borderless{
	
		}
		if opts.title{
			dsp.set_title(name);
		}
		if !opts.resize{
			dsp.set_no_resize((width as i32, height as i32));	
		}
		//TODO: opts.scale

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

	pub fn set_title(&mut self, title: &str){
		self.display.set_title(title);
	}

    pub fn set_background_color(&mut self, bg_color: u32){
        self.bg_color = bg_color;
    }

    pub fn is_open(&self) -> bool{
		!self.should_close
	}

	pub fn get_window_handle(&self) -> *mut c_void{
		self.display.surface.as_ref().c_ptr() as *mut c_void
	}

	pub fn get_size(&self) -> (usize, usize){
		(self.width as usize, self.height as usize)
	}

	pub fn get_keys(&self) -> Option<Vec<Key>>{
		self.key_handler.get_keys()
	}

	pub fn get_keys_pressed(&self, repeat: KeyRepeat) -> Option<Vec<Key>>{
		self.key_handler.get_keys_pressed(repeat)
	}

	pub fn get_mouse_pos(&self, mode: MouseMode) -> Option<(f32, f32)>{
		Some((self.mouse_x, self.mouse_y))
	}

	pub fn get_scroll_wheel(&self) -> Option<(f32, f32)>{
		Some((self.scroll_x, self.scroll_y))
	}

	pub fn is_key_down(&self, key: Key) -> bool{
		self.key_handler.is_key_down(key)
	}

	pub fn set_position(&mut self, x: isize, y: isize){
		self.display.set_geometry((x as i32, y as i32), (self.width, self.height));
	}

    //WIP
    pub fn update(&mut self){
		self.display.event_queue.sync_roundtrip(|event, object|{
		//TODO
		}).map_err(|e| Error::WindowCreate(format!("Roundtrip failed: {:?}", e))).unwrap();
    }

    //WIP
    pub fn update_with_buffer_stride(&mut self, buffer: &[u32], buf_width: usize, buf_height: usize, buf_stride: usize) -> Result<()>{
        //TODO: stride
		if buffer.len() < buf_width*buf_height{
			return Err(Error::UpdateFailed("Buffer size lower than given sizes".to_owned()));
		}
		else{
			use std::io::{Seek, SeekFrom};

			self.display.fd.seek(SeekFrom::Start(0));

			for i in 0..(buf_width*buf_height){
				let color = 0xFFFFFFFF & buffer[i];
				self.display.fd.write_u32::<NativeEndian>(color);
			}
		}

		self.update();

		Ok(())
    }
}

