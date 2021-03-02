#![cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
use wasm_bindgen::Clamped;
use wasm_bindgen::JsCast;
use web_sys::ImageData;
use web_sys::{window, CanvasRenderingContext2d, HtmlCanvasElement};

use crate::error::Error;
use crate::key_handler::KeyHandler;
use crate::mouse_handler;
use crate::rate::UpdateRate;
use crate::InputCallback;
use crate::Result;
use crate::{CursorStyle, MouseButton, MouseMode};
use crate::{Key, KeyRepeat};
use crate::{MenuHandle, MenuItem, MenuItemHandle, UnixMenu, UnixMenuItem};
use crate::{Scale, WindowOptions};

use core;
use std::os::raw;

#[inline(always)]
#[allow(dead_code)] // Only used on 32-bit builds currently
pub fn u32_as_u8<'a>(src: &'a [u32]) -> &'a [u8] {
    unsafe { core::slice::from_raw_parts(src.as_ptr() as *mut u8, src.len() * 4) }
}

pub struct Window {
    width: u32,
    height: u32,
    bg_color: u32,
    mouse_pos: Option<(i32, i32)>,
    mouse_scroll: Option<(i32, i32)>,
    /// The state of the left, middle and right mouse buttons
    mouse_state: (bool, bool, bool),
    window_scale: usize,
    img_data: ImageData,
    canvas: HtmlCanvasElement,
    context: CanvasRenderingContext2d,

    key_handler: KeyHandler,
    menu_counter: MenuHandle,
    menus: Vec<UnixMenu>,
}

impl Window {
    pub fn new(name: &str, width: usize, height: usize, opts: WindowOptions) -> Result<Window> {
        let buffer = vec![0u8; width * height * 4];
        let document = window().unwrap().document().unwrap();
        document.set_title(name);

        // Create a canvas element and place it in the window
        let canvas = document
            .create_element("canvas")
            .unwrap()
            .dyn_into::<HtmlCanvasElement>()
            .unwrap();

        let body = document.body().unwrap();
        body.append_child(&canvas).unwrap();

        canvas.set_width(width as u32);
        canvas.set_height(height as u32);

        // Create an image buffer
        let context: CanvasRenderingContext2d = canvas
            .get_context("2d")
            .unwrap()
            .unwrap()
            .dyn_into::<CanvasRenderingContext2d>()
            .unwrap();
        context.set_image_smoothing_enabled(false);
        let img_data = ImageData::new_with_sw(width as u32, height as u32).unwrap();
        let mut window = Window {
            width: width as u32,
            height: height as u32,
            bg_color: 0,
            mouse_pos: None,
            mouse_scroll: None,
            mouse_state: (false, false, false),
            window_scale: 1,
            img_data,
            canvas,
            context,
            key_handler: KeyHandler::new(),
            menu_counter: MenuHandle(0),
            menus: Vec::new(),
        };

        window.set_title(name);

        Ok(window)
    }

    #[inline]
    pub fn set_title(&mut self, title: &str) {
        let document = window().unwrap().document().unwrap();
        document.set_title(title);
    }

    #[inline]
    pub fn set_rate(&mut self, rate: Option<std::time::Duration>) {}

    #[inline]
    pub fn update_rate(&mut self) {}

    #[inline]
    pub fn get_window_handle(&self) -> *mut raw::c_void {
        0 as *mut raw::c_void
    }

    #[inline]
    pub fn topmost(&self, topmost: bool) {
        // TODO?
    }

    pub fn set_background_color(&mut self, bg_color: u32) {
        self.bg_color = bg_color;
    }

    pub fn set_cursor_visibility(&mut self, visibility: bool) {
        //TODO?
    }

    pub fn update_with_buffer_stride(
        &mut self,
        buffer: &[u32],
        buf_width: usize,
        buf_height: usize,
        buf_stride: usize,
    ) -> Result<()> {
        //buffer_helper::check_buffer_size(buf_width, buf_height, buf_width, buffer)?;
        // scaling not implemented. It's faster to just update the buffer
        // unsafe { self.scale_buffer(buffer, buf_width, buf_height, buf_stride) };
        self.update_with_buffer(&buffer).unwrap();

        Ok(())
    }

    pub fn update_with_buffer(&mut self, buffer: &[u32]) -> Result<()> {
        /*buffer_helper::check_buffer_size(
            self.width as usize,
            self.height as usize,
            self.window_scale,
            buffer,
        )?;*/
        let mut data = u32_as_u8(buffer);

        self.img_data = ImageData::new_with_u8_clamped_array_and_sh(
            Clamped(&mut data),
            self.width,
            self.height,
        )
        .unwrap();

        self.update();

        Ok(())
    }

    pub fn update(&mut self) {
        self.context
            .put_image_data(&self.img_data, 0.0, 0.0)
            .unwrap();
    }

    #[inline]
    pub fn set_position(&mut self, x: isize, y: isize) {}

    #[inline]
    pub fn get_size(&self) -> (usize, usize) {
        (self.width as usize, self.height as usize)
    }

    pub fn get_mouse_pos(&self, mode: MouseMode) -> Option<(f32, f32)> {
        None
    }

    pub fn get_unscaled_mouse_pos(&self, mode: MouseMode) -> Option<(f32, f32)> {
        None
    }

    pub fn get_mouse_down(&self, button: MouseButton) -> bool {
        match button {
            MouseButton::Left => self.mouse_state.0,
            MouseButton::Middle => self.mouse_state.1,
            MouseButton::Right => self.mouse_state.2,
        }
    }

    pub fn get_scroll_wheel(&self) -> Option<(f32, f32)> {
        None
    }

    #[inline]
    pub fn set_cursor_style(&mut self, cursor: CursorStyle) {}

    #[inline]
    pub fn get_keys(&self) -> Option<Vec<Key>> {
        None
    }

    #[inline]
    pub fn get_keys_pressed(&self, repeat: KeyRepeat) -> Option<Vec<Key>> {
        None
    }

    #[inline]
    pub fn is_key_down(&self, key: Key) -> bool {
        false
    }

    #[inline]
    pub fn set_input_callback(&mut self, callback: Box<dyn InputCallback>) {}

    #[inline]
    pub fn set_key_repeat_delay(&mut self, delay: f32) {}

    #[inline]
    pub fn set_key_repeat_rate(&mut self, rate: f32) {}

    #[inline]
    pub fn is_key_pressed(&self, key: Key, repeat: KeyRepeat) -> bool {
        false
    }

    #[inline]
    pub fn is_key_released(&self, key: Key) -> bool {
        false
    }

    #[inline]
    pub fn is_open(&self) -> bool {
        true
    }

    #[inline]
    pub fn get_keys_released(&self) -> Option<Vec<Key>> {
        self.key_handler.get_keys_released()
    }
    pub fn is_active(&mut self) -> bool {
        true
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

    pub fn remove_menu(&mut self, handle: MenuHandle) {
        self.menus.retain(|ref menu| menu.handle != handle);
    }

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

    pub fn add_sub_menu(&mut self, name: &str, sub_menu: &Menu) {}

    fn next_item_handle(&mut self) -> MenuItemHandle {
        let handle = self.internal.item_counter;
        self.internal.item_counter.0 += 1;
        handle
    }

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

    pub fn remove_item(&mut self, handle: &MenuItemHandle) {}
}

unsafe impl raw_window_handle::HasRawWindowHandle for Window {
    fn raw_window_handle(&self) -> raw_window_handle::RawWindowHandle {
        let handle = raw_window_handle::web::WebHandle {
            id: 1, //TODO: assign a different ID to each window
            ..raw_window_handle::web::WebHandle::empty()
        };
        raw_window_handle::RawWindowHandle::Web(handle)
    }
}
