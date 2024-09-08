// NOTE(stefano): add missing implementations of methods where possible
#![cfg(target_arch = "wasm32")]

mod keycodes;

use crate::{
    check_buffer_size, key_handler::KeyHandler, CursorStyle, Icon, InputCallback, Key, KeyRepeat,
    MenuHandle, MenuItem, MenuItemHandle, MouseButton, MouseMode, Result, Scale, UnixMenu,
    UnixMenuItem, WindowOptions,
};
use keycodes::event_to_key;
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, WebDisplayHandle, WebWindowHandle, WindowHandle,
};
use std::{
    cell::{Cell, RefCell},
    ffi::c_void,
    rc::Rc,
    time::Duration,
};
use wasm_bindgen::{prelude::*, Clamped, JsCast};
use web_sys::{window, CanvasRenderingContext2d, HtmlCanvasElement, HtmlElement, ImageData};

#[inline(always)]
#[allow(dead_code)] // Only used on 32-bit builds currently
pub fn u32_as_u8<'a>(src: &'a [u32]) -> &'a [u8] {
    unsafe {
        std::slice::from_raw_parts(
            src.as_ptr() as *mut u8,
            src.len() * std::mem::size_of::<u32>(),
        )
    }
}

struct MouseState {
    pos: Cell<Option<(i32, i32)>>,
    //scroll: Cell<Option<(i32, i32)>>,
    left_button: Cell<bool>,
    right_button: Cell<bool>,
    middle_button: Cell<bool>,
}

struct Context2D {
    context: CanvasRenderingContext2d,
    img_data: ImageData,
}

// IDEA(stefano): possibly have this contain a "document" field, so not to recompute it every time
pub struct Window {
    width: u32,
    height: u32,
    bg_color: u32,
    window_scale: usize,
    canvas: HtmlCanvasElement,
    // 2D context is created lazily since its creation preculdes creation of webgl & webgpu contexts.
    context2d: Option<Context2D>,
    mouse_state: Rc<MouseState>,
    key_handler: Rc<RefCell<KeyHandler>>,
    menu_counter: MenuHandle,
    menus: Vec<UnixMenu>,
    raw_handle_id: u32,
}

impl Window {
    /// Create a new window
    /// * `container` - The id of a DOM element to use as a container
    pub fn new(
        container: &str,
        width: usize,
        height: usize,
        opts: WindowOptions,
    ) -> Result<Window> {
        let window_scale = match opts.scale {
            Scale::X1 => 1,
            Scale::X2 => 2,
            Scale::X4 => 4,
            Scale::X8 => 8,
            Scale::X16 => 16,
            Scale::X32 => 32,
            Scale::FitScreen => 1, //TODO: Resize the canvas and implement this
        };
        let document = window().unwrap().document().unwrap();

        // Create a canvas element and place it in the window as a child of the container
        let canvas = document
            .create_element("canvas")
            .unwrap()
            .dyn_into::<HtmlCanvasElement>()
            .unwrap();

        // Raw handle requires to inject an id into the canvas' data attributes.
        // TODO assign a different ID to each window
        let raw_handle_id = 0;
        canvas
            .set_attribute("data-raw-handle", &raw_handle_id.to_string())
            .unwrap();

        let container = document
            .get_element_by_id(container)
            .unwrap()
            .dyn_into::<HtmlElement>()
            .unwrap();

        container.append_child(&canvas).unwrap();

        canvas.set_width(width as u32);
        canvas.set_height(height as u32);
        // set this to get the keyboard events
        canvas.set_tab_index(0);

        // Create an image buffer
        let key_handler = Rc::new(RefCell::new(KeyHandler::new()));
        let mouse_struct = MouseState {
            pos: Cell::new(None),
            //scroll: Cell::new(None),
            left_button: Cell::new(false),
            right_button: Cell::new(false),
            middle_button: Cell::new(false),
        };
        let mouse_state = Rc::new(mouse_struct);
        {
            let key_handler = key_handler.clone();
            let closure = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
                event.prevent_default();
                let key = event_to_key(&event);
                key_handler.borrow_mut().set_key_state(key, true);
            }) as Box<dyn FnMut(_)>);
            canvas.add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())?;
            closure.forget(); // FYI, the closure now lives forevah... evah... evah...
        }
        {
            let key_handler = key_handler.clone();
            let closure = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
                event.prevent_default();
                let key = event_to_key(&event);
                key_handler.borrow_mut().set_key_state(key, false);
            }) as Box<dyn FnMut(_)>);
            canvas.add_event_listener_with_callback("keyup", closure.as_ref().unchecked_ref())?;
            closure.forget(); // FYI, the closure now lives forevah... evah... evah...
        }
        {
            let mouse_state = mouse_state.clone();
            let closure = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
                mouse_state
                    .pos
                    .set(Some((event.offset_x() as i32, event.offset_y() as i32)));
                match event.button() {
                    0 => mouse_state.left_button.set(true),
                    1 => mouse_state.middle_button.set(true),
                    2 => mouse_state.right_button.set(true),
                    _ => (),
                }
            }) as Box<dyn FnMut(_)>);
            canvas
                .add_event_listener_with_callback("mousedown", closure.as_ref().unchecked_ref())?;
            closure.forget();
        }
        {
            let mouse_state = mouse_state.clone();
            let closure = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
                mouse_state
                    .pos
                    .set(Some((event.offset_x() as i32, event.offset_y() as i32)));
            }) as Box<dyn FnMut(_)>);
            canvas
                .add_event_listener_with_callback("mousemove", closure.as_ref().unchecked_ref())?;
            closure.forget();
        }
        {
            let mouse_state = mouse_state.clone();
            let closure = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
                mouse_state
                    .pos
                    .set(Some((event.offset_x() as i32, event.offset_y() as i32)));
                match event.button() {
                    0 => mouse_state.left_button.set(false),
                    1 => mouse_state.middle_button.set(false),
                    2 => mouse_state.right_button.set(false),
                    _ => (),
                }
            }) as Box<dyn FnMut(_)>);
            canvas.add_event_listener_with_callback("mouseup", closure.as_ref().unchecked_ref())?;
            closure.forget();
        }

        let mut window = Window {
            width: width as u32,
            height: height as u32,
            bg_color: 0,
            window_scale,
            canvas,
            context2d: None,
            key_handler,
            mouse_state,
            menu_counter: MenuHandle(0),
            menus: Vec::new(),
            raw_handle_id,
        };

        Ok(window)
    }

    fn create_2d_context(&self) -> CanvasRenderingContext2d {
        let context: CanvasRenderingContext2d = self
            .canvas
            .get_context("2d")
            .unwrap()
            .unwrap()
            .dyn_into::<CanvasRenderingContext2d>()
            .unwrap();
        context.set_image_smoothing_enabled(false);
        context
    }

    #[inline]
    pub fn set_title(&mut self, title: &str) {
        let document = window().unwrap().document().unwrap();
        document.set_title(title);
    }

    #[inline]
    pub fn set_rate(&mut self, rate: Option<Duration>) {}

    #[inline]
    pub fn update_rate(&mut self) {}

    #[inline]
    pub fn get_window_handle(&self) -> *mut c_void {
        std::ptr::null_mut()
    }

    #[inline]
    pub fn topmost(&self, topmost: bool) {
        // TODO?
    }

    #[inline]
    pub fn set_background_color(&mut self, bg_color: u32) {
        self.bg_color = bg_color;
    }

    #[inline]
    pub fn set_cursor_visibility(&mut self, visibility: bool) {
        self.canvas
            .style()
            .set_property(
                "cursor",
                match visibility {
                    true => "auto",
                    false => "none",
                },
            )
            .unwrap_or(());
    }

    pub fn update_with_buffer_stride(
        &mut self,
        buffer: &[u32],
        buf_width: usize,
        buf_height: usize,
        buf_stride: usize,
    ) -> Result<()> {
        check_buffer_size(buffer, buf_width, buf_height, buf_stride)?;
        // scaling not implemented. It's faster to just update the buffer
        //unsafe { self.scale_buffer(buffer, buf_width, buf_height, buf_stride) };
        self.update_with_buffer(&buffer).unwrap();

        Ok(())
    }

    pub fn update_with_buffer(&mut self, buffer: &[u32]) -> Result<()> {
        check_buffer_size(
            buffer,
            self.width as usize,
            self.height as usize,
            self.window_scale,
        )?;
        let mut data = u32_as_u8(buffer);

        let image_data = ImageData::new_with_u8_clamped_array_and_sh(
            Clamped(&mut data),
            self.width,
            self.height,
        )
        .unwrap();

        if let Some(context2d) = &mut self.context2d {
            context2d.img_data = image_data;
        } else {
            self.context2d = Some(Context2D {
                context: self.create_2d_context(),
                img_data: image_data,
            });
        }

        self.update();

        Ok(())
    }

    #[inline]
    pub fn update(&mut self) {
        self.key_handler.borrow_mut().update();

        if let Some(context2d) = &self.context2d {
            context2d
                .context
                .put_image_data(&context2d.img_data, 0.0, 0.0)
                .unwrap();
        }
    }

    #[inline]
    pub fn set_icon(&mut self, icon: Icon) {}

    #[inline]
    pub fn set_position(&mut self, x: isize, y: isize) {}

    #[inline]
    pub fn get_position(&self) -> (isize, isize) {
        (0isize, 0isize)
    }

    #[inline]
    pub fn get_size(&self) -> (usize, usize) {
        (self.width as usize, self.height as usize)
    }

    #[inline]
    pub fn get_mouse_pos(&self, mode: MouseMode) -> Option<(f32, f32)> {
        if let Some((mouse_x, mouse_y)) = self.mouse_state.pos.get() {
            mode.get_pos(
                mouse_x as f32,
                mouse_y as f32,
                self.window_scale as f32,
                self.width as f32 * self.window_scale as f32,
                self.height as f32 * self.window_scale as f32,
            )
        } else {
            None
        }
    }

    #[inline]
    pub fn get_unscaled_mouse_pos(&self, mode: MouseMode) -> Option<(f32, f32)> {
        if let Some((mouse_x, mouse_y)) = self.mouse_state.pos.get() {
            mode.get_pos(
                mouse_x as f32,
                mouse_y as f32,
                1.0 as f32,
                self.width as f32 * self.window_scale as f32,
                self.height as f32 * self.window_scale as f32,
            )
        } else {
            None
        }
    }

    #[inline]
    pub fn get_mouse_down(&self, button: MouseButton) -> bool {
        match button {
            MouseButton::Left => self.mouse_state.left_button.get(),
            MouseButton::Middle => self.mouse_state.middle_button.get(),
            MouseButton::Right => self.mouse_state.right_button.get(),
        }
    }

    #[inline]
    pub fn get_scroll_wheel(&self) -> Option<(f32, f32)> {
        None
    }

    #[inline]
    pub fn set_cursor_style(&mut self, cursor: CursorStyle) {}

    #[inline]
    pub fn get_keys(&self) -> Vec<Key> {
        self.key_handler.borrow().get_keys()
    }

    #[inline]
    pub fn get_keys_pressed(&self, repeat: KeyRepeat) -> Vec<Key> {
        self.key_handler.borrow().get_keys_pressed(repeat)
    }

    #[inline]
    pub fn is_key_down(&self, key: Key) -> bool {
        self.key_handler.borrow().is_key_down(key)
    }

    #[inline]
    pub fn set_key_repeat_delay(&mut self, delay: f32) {
        self.key_handler.borrow_mut().set_key_repeat_delay(delay)
    }

    #[inline]
    pub fn set_key_repeat_rate(&mut self, rate: f32) {
        self.key_handler.borrow_mut().set_key_repeat_rate(rate)
    }

    #[inline]
    pub fn is_key_pressed(&self, key: Key, repeat: KeyRepeat) -> bool {
        self.key_handler.borrow().is_key_pressed(key, repeat)
    }

    #[inline]
    pub fn is_key_released(&self, key: Key) -> bool {
        self.key_handler.borrow().is_key_released(key)
    }

    #[inline]
    pub fn set_input_callback(&mut self, callback: Box<dyn InputCallback>) {
        self.key_handler.borrow_mut().set_input_callback(callback)
    }

    #[inline]
    pub fn is_open(&self) -> bool {
        true
    }

    #[inline]
    pub fn get_keys_released(&self) -> Vec<Key> {
        self.key_handler.borrow().get_keys_released()
    }

    #[inline]
    pub fn is_active(&mut self) -> bool {
        let document = window().unwrap().document().unwrap();
        document.active_element().unwrap_or(return false) == **self.canvas
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
    pub fn remove_menu(&mut self, handle: MenuHandle) {
        self.menus.retain(|ref menu| menu.handle != handle);
    }

    #[inline]
    pub fn is_menu_pressed(&mut self) -> Option<usize> {
        None
    }
}

pub struct Menu {
    pub internal: UnixMenu,
}

impl Menu {
    pub fn new(name: &str) -> Result<Menu> {
        Ok(Menu {
            internal: UnixMenu {
                handle: MenuHandle(0),
                item_counter: MenuItemHandle(0),
                name: name.to_owned(),
                items: Vec::new(),
            },
        })
    }

    #[inline]
    pub fn add_sub_menu(&mut self, name: &str, sub_menu: &Menu) {}

    #[inline]
    fn next_item_handle(&mut self) -> MenuItemHandle {
        let handle = self.internal.item_counter;
        self.internal.item_counter.0 += 1;
        handle
    }

    #[inline]
    pub fn add_menu_item(&mut self, item: &MenuItem) -> MenuItemHandle {
        let item_handle = self.next_item_handle();
        self.internal.items.push(UnixMenuItem {
            sub_menu: None,
            handle: self.internal.item_counter,
            id: item.id,
            label: item.label.clone(),
            enabled: item.enabled,
            key: item.key,
            modifier: item.modifier,
        });
        item_handle
    }

    #[inline]
    pub fn remove_item(&mut self, handle: &MenuItemHandle) {}
}

impl HasWindowHandle for Window {
    fn window_handle(&self) -> std::result::Result<WindowHandle, HandleError> {
        let handle = WebWindowHandle::new(self.raw_handle_id);
        let raw_handle = RawWindowHandle::Web(handle);
        unsafe { Ok(WindowHandle::borrow_raw(raw_handle)) }
    }
}

impl HasDisplayHandle for Window {
    fn display_handle(&self) -> std::result::Result<DisplayHandle, HandleError> {
        let handle = WebDisplayHandle::new();
        let raw_handle = RawDisplayHandle::Web(handle);
        unsafe { Ok(DisplayHandle::borrow_raw(raw_handle)) }
    }
}
