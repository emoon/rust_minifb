use crate::{Key, MenuHandle, MenuItem, MenuItemHandle, Result, UnixMenu, UnixMenuItem};

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
    pub fn add_sub_menu(&mut self, name: &str, sub_menu: &Menu) {
        let handle = self.next_item_handle();
        self.internal.items.push(UnixMenuItem {
            label: name.to_owned(),
            handle,
            sub_menu: Some(Box::new(sub_menu.internal.clone())),
            id: 0,
            enabled: true,
            key: Key::Unknown,
            modifier: 0,
        });
    }

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
    pub fn remove_item(&mut self, handle: &MenuItemHandle) {
        self.internal.items.retain(|item| item.handle.0 != handle.0);
    }
}

// These functions are implemented in C in order to always have
// optimizations on (`-O3`), allowing debug builds to run fast as well.
extern "C" {
    pub(crate) fn image_upper_left(
        dst: *mut u32,
        dst_width: u32,
        dst_height: u32,
        src: *const u32,
        src_width: u32,
        src_height: u32,
        src_stride: u32,
        bg_color: u32,
    );

    pub(crate) fn image_center(
        dst: *mut u32,
        dst_width: u32,
        dst_height: u32,
        src: *const u32,
        src_width: u32,
        src_height: u32,
        src_stride: u32,
        bg_color: u32,
    );

    pub(crate) fn image_resize_linear_aspect_fill(
        dst: *mut u32,
        dst_width: u32,
        dst_height: u32,
        src: *const u32,
        src_width: u32,
        src_height: u32,
        src_stride: u32,
        bg_color: u32,
    );

    pub(crate) fn image_resize_linear(
        dst: *mut u32,
        dst_width: u32,
        dst_height: u32,
        src: *const u32,
        src_width: u32,
        src_height: u32,
        src_stride: u32,
    );
}
