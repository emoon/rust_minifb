use Key;
use std::os::raw;
use std::ptr;
use std::ffi::CString;

pub const MENU_ID_SEPARATOR:usize = 0;

//#[derive(Debug, Default)]
pub struct Menu {
    name: &'static str,
    id: usize,
    key: Key,
    modifier: usize,
    mac_mod: usize,
    sub_menu: Option<Box<Menu>>,
}

#[repr(C)]
struct CMenu {
    name: *const raw::c_char,
    id: raw::c_int,
    key: raw::c_int,
    modifier: raw::c_int,
    mac_mod: raw::c_int,
    sub_menu: *mut raw::c_void,
}

unsafe fn recursive_convert(in_menu: &Option<Box<Menu>>) -> *mut raw::c_void {
    if in_menu.is_none() {
        return ptr::null_mut();
    }

    let m = in_menu.as_ref().unwrap();

    let menu = Box::new(CMenu {
        name: CString::new(m.name).unwrap().as_ptr(),
        id: m.id as raw::c_int, 
        key: m.key as raw::c_int, 
        modifier: m.modifier as raw::c_int, 
        mac_mod: m.mac_mod as raw::c_int, 
        sub_menu : recursive_convert(&m.sub_menu),
    });

    Box::into_raw(menu) as *mut raw::c_void
}

pub fn convert_menu_to_c_menu(menu: Box<Menu>) -> *mut raw::c_void {
    unsafe { recursive_convert(&Some(menu)) }
}
