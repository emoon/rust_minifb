/*
use Key;

/// Command key on Mac OS
pub const MENU_KEY_COMMAND: usize = 1;
/// Windows key on Windows
pub const MENU_KEY_WIN: usize = 2;
/// Shift key
pub const MENU_KEY_SHIFT: usize = 4;
/// Control key
pub const MENU_KEY_CTRL: usize = 8;
/// Alt key
pub const MENU_KEY_ALT: usize = 16;

const MENU_ID_SEPARATOR:usize = 0xffffffff;

#[cfg(target_os = "macos")]
use self::os::macos as imp;
#[cfg(target_os = "windows")]
use self::os::windows as imp;
#[cfg(any(target_os="linux",
    target_os="freebsd",
    target_os="dragonfly",
    target_os="netbsd",
    target_os="openbsd"))]
use self::os::unix as imp;

pub struct Menu(imp::Menu);

impl Menu {
    pub fn new(name: &name) -> Result<Menu> {
        imp::Menu::new(name).map(Menu)
    }

    #[inline]
    pub fn destroy_menu(&mut self) {
        self.0.destroy_menu()
    }

    #[inline]
    pub fn add_sub_menu(&mut self, menu: &Menu) {
        self.0.add_sub_menu(menu)
    }

    #[inline]
    pub fn add_item(&mut self, item: &mut MenuItem) {
        self.0.add_item(item)
    }

    #[inline]
    pub fn remove_item(&mut self, item: &mut MenuItem) {
        self.0.remove_item(item)
    }
}
*/
