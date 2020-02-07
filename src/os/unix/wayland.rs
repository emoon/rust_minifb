use crate::{CursorStyle, MenuHandle, MenuItem, MenuItemHandle, UnixMenu, UnixMenuItem};
use crate::{InputCallback, Key, KeyRepeat, MouseButton, MouseMode, Scale, ScaleMode, WindowOptions};
use crate::{Error, Result};
use crate::rate::UpdateRate;
use crate::key_handler::KeyHandler;
use crate::mouse_handler;

use wayland_client::protocol::{wl_display::WlDisplay, wl_compositor::WlCompositor, wl_shm::{WlShm, Format}, wl_shm_pool::WlShmPool, wl_buffer::WlBuffer, wl_surface::WlSurface, wl_seat::WlSeat, wl_keyboard::WlKeyboard, wl_pointer::WlPointer, wl_touch::WlTouch};
use wayland_client::{EventQueue, GlobalManager};
use wayland_client::{Main, Attached};
use wayland_protocols::xdg_shell::client::{xdg_wm_base::XdgWmBase, xdg_surface::XdgSurface, xdg_toplevel::XdgToplevel};
use std::io::Write;
use std::ffi::c_void;

use std::rc::Rc;
use std::cell::RefCell;


pub struct DisplayInfo{
	display: wayland_client::Display,
	wl_display: Attached<WlDisplay>,
	comp: Main<WlCompositor>,
	base: Main<XdgWmBase>,
	surface: Main<WlSurface>,
	xdg_surface: Main<XdgSurface>,
	toplevel: Main<XdgToplevel>,
	shm: Main<WlShm>,
	//Current max ShmPool size
	shm_pool: (Main<WlShmPool>, i32),
	//Hold the state of each WlBuffer if allowed to be destroyed
	buf: Vec<(Main<WlBuffer>, Rc<RefCell<bool>>)>,
	event_queue: EventQueue,
	fd: std::fs::File,
	seat: Main<WlSeat>,
	//size of the framebuffer
	fb_size: (i32, i32),
	//Wayland buffer pixel format
	format: Format
}

impl DisplayInfo{
	//size: size of the surface to be created
	//alpha: whether the alpha channel shall be rendered
	//decoration: whether server-side window decoration shall be created
	pub fn new(size: (i32, i32), alpha: bool, decoration: bool) -> Result<Self>{
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
		let mut frame: Vec<u32> = vec![0xFF000000; (size.0*size.1) as usize];
		let slice = unsafe{std::slice::from_raw_parts(frame[..].as_ptr() as *const u8, frame.len() * std::mem::size_of::<u32>())};
		tmp_f.write_all(&slice[..]).unwrap();
		tmp_f.flush().unwrap();

		//specify format
		let format = if alpha{
			Format::Argb8888
		}
		else{
			Format::Xrgb8888
		};

		//create a shared memory
		let shm_pool = shm.create_pool(tmp_f.as_raw_fd(), size.0*size.1*std::mem::size_of::<u32>() as i32);
		
		let (buffer, buf_not_needed) = Self::create_shm_buffer(&shm_pool, size, format);

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
		let xdg_toplevel = xdg_surface.get_toplevel();
		if decoration{
			use wayland_protocols::unstable::xdg_decoration::v1::client::zxdg_decoration_manager_v1::ZxdgDecorationManagerV1;

			if let Ok(deco_man) = global_man.instantiate_exact::<ZxdgDecorationManagerV1>(1).map_err(|e| Error::WindowCreate(format!("Failed creating server-side surface decoration: {:?}", e))).map_err(|e| println!("{:?}", e)){
				deco_man.get_toplevel_decoration(&xdg_toplevel);
				deco_man.destroy();
			}
		}
		surface.commit();

		event_q.sync_roundtrip(|_, _|{}).map_err(|e| Error::WindowCreate(format!("Roundtrip failed: {:?}", e)))?;

		//give the surface the buffer and commit
		surface.attach(Some(&buffer), 0, 0);
		surface.damage_buffer(0, 0, size.0, size.1);
		surface.commit();

		let seat = global_man.instantiate_exact::<WlSeat>(1).map_err(|e| Error::WindowCreate(format!("Failed retrieving the WlSeat: {:?}", e)))?;	

		let keyboard = seat.get_keyboard();
		let pointer = seat.get_pointer();

		//Removed the surface commit because of redrawing issue
		xdg_surface.assign_mono(move |xdg_surface, event|{
			use wayland_protocols::xdg_shell::client::xdg_surface::Event;

			if let Event::Configure{serial} = event{
				xdg_surface.ack_configure(serial);
			}
		});


		Ok(Self{
			display,
			wl_display,
			comp,
			base: xdg_wm_base,
			surface,
			xdg_surface,
			toplevel: xdg_toplevel,
			shm_pool: (shm_pool, size.0 * size.1 * std::mem::size_of::<u32>() as i32),
			shm,
			buf: {let mut v = Vec::new(); v.push((buffer, buf_not_needed)); v},
			event_queue: event_q,
			fd: tmp_f,
			seat,
			fb_size: (size.0, size.1),
			format
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

	fn create_shm_buffer(shm_pool: &Main<WlShmPool>, size: (i32, i32), format: Format) -> (Main<WlBuffer>, Rc<RefCell<bool>>){
		let buf = shm_pool.create_buffer(0, size.0, size.1, size.0*std::mem::size_of::<u32>() as i32, format);
		let buf_not_needed = Rc::new(RefCell::new(false));
		{
			let buf_not_needed = buf_not_needed.clone();

			buf.assign_mono(move |buf, event|{
				use wayland_client::protocol::wl_buffer::Event;

				if let Event::Release = event{
					*buf_not_needed.borrow_mut() = true;
				}
			});
		}

		(buf, buf_not_needed)
	}

	//resizes when buffer is bigger or less
	fn update_framebuffer(&mut self, buffer: &[u32], size: (i32, i32)){
		use std::io::{Seek, SeekFrom};

		let cnt = (self.fb_size.0 * self.fb_size.1 * std::mem::size_of::<u32>() as i32) as usize;
		self.fb_size = size;

		self.fd.seek(SeekFrom::Start(0)).unwrap();
		let slice = unsafe{std::slice::from_raw_parts(buffer[..].as_ptr() as *const u8, buffer.len() * std::mem::size_of::<u32>())};
		self.fd.write_all(&slice[..]).unwrap();
		self.fd.flush().unwrap();

		if cnt != buffer.len() * std::mem::size_of::<u32>(){
			//change file length
			self.fd.set_len((size.0 * size.1 * std::mem::size_of::<u32>() as i32) as u64).unwrap();
			//Shm Pool is not allowed to be resized
			let new_pool_size = (buffer.len() * std::mem::size_of::<u32>()) as i32;
			if new_pool_size > self.shm_pool.1{
				self.shm_pool.0.resize(size.0 * size.1 * std::mem::size_of::<u32>() as i32);
				self.shm_pool.1 = new_pool_size;
			}

			//create new buffer and add it to the vec
			let (buf, buf_not_needed) = Self::create_shm_buffer(&self.shm_pool.0, size, self.format);

			//remove the buffers which are allowed to be removed
			self.buf.retain(|(wlbuf, not_req)|{
				if *not_req.borrow(){
					wlbuf.destroy();
					false
				}
				else{
					true
				}
			});
			self.buf.push((buf, buf_not_needed));
		}

		self.surface.attach(Some(&self.buf[self.buf.len()-1].0), 0, 0);
		self.surface.damage_buffer(0, 0, size.0, size.1);
		self.surface.commit();
	}

	fn get_input_devs(&self) -> (Main<WlKeyboard>, Main<WlPointer>, Main<WlTouch>){
		(self.seat.get_keyboard(), self.seat.get_pointer(), self.seat.get_touch())
	}

	//Keyboard, Pointer, Touch
	fn check_capabilities(seat: &Main<WlSeat>, event_queue: &mut EventQueue) -> (bool, bool, bool){
		let keyboard_fl = Rc::new(RefCell::new(false));
		let pointer_fl = Rc::new(RefCell::new(false));
		let touch_fl = Rc::new(RefCell::new(false));

		{
			let keyboard_fl = keyboard_fl.clone();
			let pointer_fl = pointer_fl.clone();
			let touch_fl = touch_fl.clone();
		
			//check pointer and mouse capability
			seat.assign_mono(move |seat, event|{
				use wayland_client::protocol::wl_seat::{Event, Capability};

				if let Event::Capabilities{capabilities} = event{
					if !*pointer_fl.borrow() && capabilities.contains(Capability::Pointer){
						*pointer_fl.borrow_mut() = true;
					}
					if !*keyboard_fl.borrow() && capabilities.contains(Capability::Keyboard){
						*keyboard_fl.borrow_mut() = true;
					}
					if !*touch_fl.borrow() && capabilities.contains(Capability::Touch){
						*touch_fl.borrow_mut() = true;
					}
				}
			});
		}

		event_queue.sync_roundtrip(|_, _|{}).map_err(|e| Error::WindowCreate(format!("Roundtrip failed: {:?}", e))).unwrap();
		
		let ret = (*keyboard_fl.borrow(), *pointer_fl.borrow(), *touch_fl.borrow());

		ret
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
	//TODO: crate: wayland_cursor
	prev_cursor: CursorStyle,

	should_close: bool,
	active: bool,

	key_handler: KeyHandler,
	update_rate: UpdateRate,
	menu_counter: MenuHandle,
	menus: Vec<UnixMenu>,
	input_devs: (Main<WlKeyboard>, Main<WlPointer>),
	events: (Rc<RefCell<Vec<wayland_client::protocol::wl_keyboard::Event>>>, Rc<RefCell<Vec<wayland_client::protocol::wl_pointer::Event>>>),
	resizable: bool
}


impl Window{
	pub fn new(name: &str, width: usize, height: usize, opts: WindowOptions) -> Result<Self>{
		let dsp = DisplayInfo::new((width as i32, height as i32), false, !opts.borderless)?;
		let scale;
		if opts.title{
			dsp.set_title(name);
		}
		if !opts.resize{
			dsp.set_no_resize((width as i32, height as i32));	
		}
		//TODO: opts.scale
        match opts.scale{
            Scale::FitScreen => {
               //TODO
               scale=1;
            }
            Scale::X1 => {
                scale = 1;
            }
            Scale::X2 => {
                scale = 2;
            }
            Scale::X4 => {
                scale = 4;
            }
            Scale::X8 => {
                scale = 8;
            }
            Scale::X16 => {
                scale = 16;
            }
            Scale::X32 => {
                scale = 32;
			}
		}

		let (keyboard, pointer, _touch) = dsp.get_input_devs();

        let events_kb = Rc::new(RefCell::new(Vec::new()));

		{
			let events_kb = events_kb.clone();
			keyboard.assign_mono(move |keyboard, event|{
				(*events_kb.borrow_mut()).push(event);
			});
		}

		let events_pt = Rc::new(RefCell::new(Vec::new()));

		{
			let events_pt = events_pt.clone();

			pointer.assign_mono(move |pointer, event|{
				(*events_pt.borrow_mut()).push(event);
			});
		}

		Ok(Self{
			display: dsp,

			width: width as i32,
			height: height as i32,
	
			//TODO
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
			update_rate: UpdateRate::new(),
			menu_counter: MenuHandle(0),
			menus: Vec::new(),
			input_devs: (keyboard, pointer),
			events: (events_kb, events_pt),
			resizable: opts.resize
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
		mouse_handler::get_pos(mode, self.mouse_x, self.mouse_y, self.scale as f32, self.width as f32, self.height as f32)
	}

	pub fn get_unscaled_mouse_pos(&self, mode: MouseMode) -> Option<(f32, f32)>{
		mouse_handler::get_pos(mode, self.mouse_x, self.mouse_y, 1.0, self.width as f32, self.height as f32)
	}

	pub fn get_scroll_wheel(&self) -> Option<(f32, f32)>{
		if self.scroll_x.abs() > 0.0 || self.scroll_y > 0.0{
			Some((self.scroll_x, self.scroll_y))
		}
		else{
			None
		}
	}

	pub fn is_key_down(&self, key: Key) -> bool{
		self.key_handler.is_key_down(key)
	}

	pub fn set_position(&mut self, x: isize, y: isize){
		self.display.set_geometry((x as i32, y as i32), (self.width, self.height));
	}

	pub fn set_rate(&mut self, rate: Option<std::time::Duration>){
		self.update_rate.set_rate(rate);
	}

	pub fn set_key_repeat_rate(&mut self, rate: f32){
		self.key_handler.set_key_repeat_delay(rate);
	}

	pub fn set_key_repeat_delay(&mut self, delay: f32){
		self.key_handler.set_key_repeat_delay(delay);
	}

	pub fn is_key_pressed(&self, key: Key, repeat: KeyRepeat) -> bool{
		self.key_handler.is_key_pressed(key, repeat)
	}

	pub fn is_key_released(&self, key: Key) -> bool{
		!self.key_handler.is_key_released(key)
	}

	pub fn update_rate(&mut self){
		self.update_rate.update();
	}

	pub fn is_active(&self) -> bool{
		self.active
	}

    //WIP
    pub fn update(&mut self){
		let configure = Rc::new(RefCell::new(None));
		let close = Rc::new(RefCell::new(false));

		{
			let configure = configure.clone();
			let close = close.clone();

			self.display.toplevel.assign_mono(move |xdg_toplevel, event|{
				use wayland_protocols::xdg_shell::client::xdg_toplevel::Event;

				if let Event::Configure{width, height, states} = event{
					*configure.borrow_mut() = Some((width, height));
				}
				else if let Event::Close = event{
					*close.borrow_mut() = true;
				}
			});
		}

		self.display.event_queue.dispatch(|event, object|{}).map_err(|e| Error::WindowCreate(format!("Event dispatch failed: {:?}", e))).unwrap();
		
		if let Some(resize) = *configure.borrow(){
			if self.resizable{
				self.width = resize.0;
				self.height = resize.1;
			}
		}
		if *close.borrow(){
			self.should_close=true;
		}

		for event in self.events.0.borrow().iter(){
			use wayland_client::protocol::wl_keyboard::Event;
			match event{
				Event::Enter{serial, surface, keys} => {
						
				},
				Event::Leave{serial, surface} => {
						
				},
				Event::Key{serial, time, key, state} => {
						
				},
				Event::Modifiers{serial, mods_depressed, mods_latched, mods_locked, group} => {
	
				},
				_ => {}
			}
		}

		for event in self.events.1.borrow().iter(){
			use wayland_client::protocol::wl_pointer::Event;
			match event{
				Event::Enter{serial, surface, surface_x, surface_y} => {
					self.mouse_x = *surface_x as f32;
					self.mouse_y = *surface_y as f32;
				},
				Event::Leave{serial, surface} => {
					
				},
				Event::Motion{time, surface_x, surface_y} => {
	
				},
				Event::Button{serial, time, button, state} => {
	
				},
				Event::Axis{time, axis, value} => {
	
				},
				Event::Frame{} => {
	
				},
				Event::AxisSource{axis_source} => {
	
				},
				Event::AxisStop{time, axis} => {
	
				},
				Event::AxisDiscrete{axis, discrete} => {
	
				},
				_ => {}
			}
		}

		(*self.events.0.borrow_mut()).clear();
		(*self.events.1.borrow_mut()).clear();
	}

    pub fn update_with_buffer_stride(&mut self, buffer: &[u32], buf_width: usize, buf_height: usize, buf_stride: usize) -> Result<()>{
		crate::buffer_helper::check_buffer_size(buf_width, buf_height, buf_width, buffer)?;

		self.display.update_framebuffer(buffer, (buf_width as i32, buf_height as i32));

		self.update();

		Ok(())
    }
}


unsafe impl raw_window_handle::HasRawWindowHandle for Window{
	fn raw_window_handle(&self) -> raw_window_handle::RawWindowHandle{
		let mut handle = raw_window_handle::unix::WaylandHandle::empty();
		handle.surface = self.display.surface.as_ref().c_ptr() as *mut _ as *mut c_void;
		handle.display = self.display.wl_display.clone().detach().as_ref().c_ptr() as *mut _ as *mut c_void;
		raw_window_handle::RawWindowHandle::Wayland(handle)
	}
}

