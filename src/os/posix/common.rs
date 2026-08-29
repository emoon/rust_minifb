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

#[cfg(test)]
mod scaler_tests {
    use super::*;

    /// The guard has to absorb the whole overrun, not just its first element:
    /// an overrun that reaches past the allocation corrupts the heap and takes
    /// the test binary down before the assertion runs. No scaler overruns by
    /// more than the source size, so a source-sized guard bounds them all.
    ///
    /// Only write overruns are caught. An out-of-bounds *read* leaves the guard
    /// intact and shows up as a wrong pixel value or a crash instead.
    fn scale_guarded(
        f: unsafe extern "C" fn(*mut u32, u32, u32, *const u32, u32, u32, u32, u32),
        dst_width: usize,
        dst_height: usize,
        src_width: usize,
        src_height: usize,
    ) -> Vec<u32> {
        const GUARD_VALUE: u32 = 0xdead_beef;

        let src: Vec<u32> = (0..src_width * src_height).map(|i| i as u32 | 1).collect();
        let visible = dst_width * dst_height;
        let guard = src.len() + 64;
        let mut dst = vec![GUARD_VALUE; visible + guard];

        unsafe {
            f(
                dst.as_mut_ptr(),
                dst_width as u32,
                dst_height as u32,
                src.as_ptr(),
                src_width as u32,
                src_height as u32,
                src_width as u32,
                0,
            );
        }

        assert!(
            dst[visible..].iter().all(|&v| v == GUARD_VALUE),
            "scaler wrote past the end of the destination buffer ({}x{} dst, {}x{} src)",
            dst_width,
            dst_height,
            src_width,
            src_height
        );

        dst.truncate(visible);
        dst
    }

    /// Reachable from safe code via `ScaleMode::UpperLeft`.
    #[test]
    fn upper_left_does_not_overflow_when_source_is_taller() {
        for (dw, dh, sw, sh) in [(1, 1, 1, 2), (200, 100, 100, 300), (64, 16, 33, 100)] {
            scale_guarded(image_upper_left, dw, dh, sw, sh);
        }
    }

    /// Upper-left anchoring shows the top-left corner of the source, not a
    /// vertically centered slice of it.
    #[test]
    fn upper_left_is_anchored_top_left() {
        let dst = scale_guarded(image_upper_left, 2, 2, 2, 4);
        assert_eq!(dst, vec![1, 1, 3, 3]);
    }

    #[test]
    fn center_does_not_overflow() {
        for (dw, dh, sw, sh) in [(1, 1, 2, 2), (100, 200, 300, 100), (33, 64, 100, 7)] {
            scale_guarded(image_center, dw, dh, sw, sh);
        }
    }

    #[test]
    fn aspect_fill_centers_the_image() {
        // 100x100 source (1:1) into a 300x100 window: pillarboxed, so the
        // image should occupy the middle third.
        let dst = scale_guarded(image_resize_linear_aspect_fill, 300, 100, 100, 100);
        let row = &dst[50 * 300..51 * 300];
        let first = row.iter().position(|&v| v != 0).unwrap();
        let last = row.iter().rposition(|&v| v != 0).unwrap();
        assert_eq!((first, last), (100, 199));

        // 300x100 source (3:1) into a 300x300 window: letterboxed.
        let dst = scale_guarded(image_resize_linear_aspect_fill, 300, 300, 300, 100);
        let col: Vec<u32> = (0..300).map(|y| dst[y * 300 + 150]).collect();
        let first = col.iter().position(|&v| v != 0).unwrap();
        let last = col.iter().rposition(|&v| v != 0).unwrap();
        assert_eq!((first, last), (100, 199));
    }

    /// A source dimension above 2^21 overflows a 32-bit 10.10 accumulator.
    /// Such a buffer passes `check_buffer_size` (2.2M x 1 is 8.8 MB).
    #[test]
    fn resize_linear_handles_large_source_dimensions() {
        let width = 2_200_000usize;
        let src = vec![7u32; width];
        let mut dst = vec![0u32; 100 * 100];

        unsafe {
            image_resize_linear(
                dst.as_mut_ptr(),
                100,
                100,
                src.as_ptr(),
                width as u32,
                1,
                width as u32,
            );
        }

        assert!(dst.iter().all(|&v| v == 7));
    }

    /// The pillarbox branch routes through a second copy of the resize loop
    /// with its own accumulators. Overflowing them needs a source dimension
    /// above 2^21 *and* a destination tall enough for the loop to reach the
    /// overflowing iteration -- a short destination scales the source down to
    /// a zero-width blit that never indexes it.
    #[test]
    fn aspect_fill_handles_large_source_dimensions() {
        let tall = 2_100_000usize;
        let dst = scale_guarded(image_resize_linear_aspect_fill, 2, tall, 1, tall);

        // One source column, centered: column 0 is the blit, column 1 is
        // background. Row y maps to source row y at this ratio.
        for y in [0, tall / 2, 2_097_151, 2_097_152, tall - 1] {
            assert_eq!(
                dst[y * 2],
                y as u32 | 1,
                "row {} came from outside the source",
                y
            );
        }
    }

    #[test]
    fn scalers_accept_degenerate_sizes() {
        let src = [1u32, 2, 3, 4];
        let mut dst = [0u32; 4];

        unsafe {
            image_resize_linear(dst.as_mut_ptr(), 0, 0, src.as_ptr(), 2, 2, 2);
            image_resize_linear(dst.as_mut_ptr(), 2, 2, src.as_ptr(), 0, 0, 0);
            image_resize_linear_aspect_fill(dst.as_mut_ptr(), 2, 2, src.as_ptr(), 0, 0, 0, 0);
            image_center(dst.as_mut_ptr(), 2, 2, src.as_ptr(), 0, 0, 0, 0);
            image_upper_left(dst.as_mut_ptr(), 2, 2, src.as_ptr(), 0, 0, 0, 0);
        }
    }
}
