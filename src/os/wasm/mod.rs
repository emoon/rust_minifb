#![cfg(target_arch = "wasm32")]

use stdweb::{
    unstable::TryInto,
    web::{
        self, document, html_element::CanvasElement, window, CanvasRenderingContext2d, INode,
        IParentNode, ImageData,
    },
};

use buffer_helper;
use error::Error;
use key_handler::KeyHandler;
use mouse_handler;
use InputCallback;
use Result;
use {CursorStyle, MouseButton, MouseMode};
use {Key, KeyRepeat};
use {MenuHandle, MenuItem, MenuItemHandle, UnixMenu, UnixMenuItem};
use {Scale, WindowOptions};

use std::os::raw;

pub struct Window {
    width: u32,
    height: u32,
    mouse_pos: Option<(i32, i32)>,
    mouse_scroll: Option<(i32, i32)>,
    /// The state of the left, middle and right mouse buttons
    mouse_state: (bool, bool, bool),
    window_scale: usize,

    canvas: CanvasElement,
    context: CanvasRenderingContext2d,

    key_handler: KeyHandler,
    menu_counter: MenuHandle,
    menus: Vec<UnixMenu>,
}

impl Window {
    pub fn new(name: &str, width: usize, height: usize, opts: WindowOptions) -> Result<Window> {
        stdweb::initialize();

        js!(
            window.onload = function() {
                console.log("Hello");
            }
        );

        let document = document();
        document.set_title(name);

        // Create a canvas element and place it in the window
        let canvas: CanvasElement = document
            .create_element("canvas")
            .unwrap()
            .try_into()
            .unwrap();

        let body = document.body().unwrap();
        body.append_child(&canvas);

        canvas.set_width(width as u32);
        canvas.set_height(height as u32);

        // Create an image buffer
        let context: CanvasRenderingContext2d = canvas.get_context().unwrap();

        let mut window = Window {
            width: width as u32,
            height: height as u32,
            mouse_pos: None,
            mouse_scroll: None,
            mouse_state: (false, false, false),
            window_scale: 1,

            canvas,
            context,

            key_handler: KeyHandler::new(),
            menu_counter: MenuHandle(0),
            menus: Vec::new(),
        };

        stdweb::event_loop();

        window.set_title(name);

        Ok(window)
    }

    #[inline]
    pub fn set_title(&mut self, title: &str) {
        document().set_title(title);
    }

    #[inline]
    pub fn get_window_handle(&self) -> *mut raw::c_void {
        0 as *mut raw::c_void
    }

    pub fn update_with_buffer(&mut self, buffer: &[u32]) -> Result<()> {
        buffer_helper::check_buffer_size(
            self.width as usize,
            self.height as usize,
            self.window_scale,
            buffer,
        )?;

        let image_data = self
            .context
            .create_image_data(self.width as f64, self.height as f64)
            .unwrap();

        for i in 0..buffer.len() {
            let pixel = buffer[i];
            let r = ((pixel & 0x00FF0000) >> 16) as u8;
            let g = ((pixel & 0x0000FF00) >> 8) as u8;
            let b = (pixel & 0x000000FF) as u8;
            let a = ((pixel & 0xFF000000) >> 24) as u8;

            let index = i as u32 * 4;
            js!(
                @{&image_data}.data[@{index} + 0] = @{r};  // R value
                @{&image_data}.data[@{index} + 1] = @{g};  // G value
                @{&image_data}.data[@{index} + 2] = @{b};  // B value
                @{&image_data}.data[@{index} + 3] = @{a};  // A value
            );
        }

        self.context.put_image_data(image_data, 0.0, 0.0).unwrap();

        self.update();

        Ok(())
    }

    pub fn update(&mut self) {}

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

    pub fn get_unix_menus(&self) -> Option<&Vec<UnixMenu>> {
        Some(&self.menus)
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
