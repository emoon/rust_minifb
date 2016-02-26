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

///
/// Used to hold the data for creating menus for the Application
///
pub struct Menu<'a> {
    /// Name of the menu item
    pub name: &'a str,
    /// User-defined Id thot will be sent back to the application in [get_menu_event]
    pub id: usize,
    /// Shortcut key for the menu item
    pub key: Key,
    /// Modifier on Windows for the menu
    pub modifier: usize,
    /// Modifier on Mac OS
    pub mac_mod: usize,
    /// Menu item should be enabled on grayed out
    pub enabled: bool,
    /// Sub-menu. Vector of a sub-menu, otherwise None if no sub-menu
    pub sub_menu: Option<&'a Vec<Menu<'a>>>,
}

impl<'a> Menu<'a> {
    pub fn separotor() -> Menu<'a> {
        Menu {
            id: MENU_ID_SEPARATOR,
            .. Self::default()
        }
    }
}

impl<'a> Default for Menu<'a> {
    fn default() -> Menu<'a> {
        Menu {
            name: "",
            id: 0,
            key: Key::Unknown,
            modifier: 0,
            mac_mod: 0,
            enabled: true,
            sub_menu: None,
        }
    }
}


