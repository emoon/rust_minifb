use Key;

pub const MENU_ID_SEPARATOR:usize = 0xffffffff;

pub struct Menu<'a> {
    pub name: &'a str,
    pub id: usize,
    pub key: Key,
    pub modifier: usize,
    pub mac_mod: usize,
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
            sub_menu: None,
        }
    }
}


