#![allow(missing_docs, non_upper_case_globals, unused_parens, clippy::all)]

/** The subset of XKB_* key symbols that are used in minifb. */
pub const XKB_KEY_0: u32 = 0x0030;
pub const XKB_KEY_1: u32 = 0x0031;
pub const XKB_KEY_2: u32 = 0x0032;
pub const XKB_KEY_3: u32 = 0x0033;
pub const XKB_KEY_4: u32 = 0x0034;
pub const XKB_KEY_5: u32 = 0x0035;
pub const XKB_KEY_6: u32 = 0x0036;
pub const XKB_KEY_7: u32 = 0x0037;
pub const XKB_KEY_8: u32 = 0x0038;
pub const XKB_KEY_9: u32 = 0x0039;
pub const XKB_KEY_a: u32 = 0x0061;
pub const XKB_KEY_b: u32 = 0x0062;
pub const XKB_KEY_c: u32 = 0x0063;
pub const XKB_KEY_d: u32 = 0x0064;
pub const XKB_KEY_e: u32 = 0x0065;
pub const XKB_KEY_f: u32 = 0x0066;
pub const XKB_KEY_g: u32 = 0x0067;
pub const XKB_KEY_h: u32 = 0x0068;
pub const XKB_KEY_i: u32 = 0x0069;
pub const XKB_KEY_j: u32 = 0x006a;
pub const XKB_KEY_k: u32 = 0x006b;
pub const XKB_KEY_l: u32 = 0x006c;
pub const XKB_KEY_m: u32 = 0x006d;
pub const XKB_KEY_n: u32 = 0x006e;
pub const XKB_KEY_o: u32 = 0x006f;
pub const XKB_KEY_p: u32 = 0x0070;
pub const XKB_KEY_q: u32 = 0x0071;
pub const XKB_KEY_r: u32 = 0x0072;
pub const XKB_KEY_s: u32 = 0x0073;
pub const XKB_KEY_t: u32 = 0x0074;
pub const XKB_KEY_u: u32 = 0x0075;
pub const XKB_KEY_v: u32 = 0x0076;
pub const XKB_KEY_w: u32 = 0x0077;
pub const XKB_KEY_x: u32 = 0x0078;
pub const XKB_KEY_y: u32 = 0x0079;
pub const XKB_KEY_z: u32 = 0x007a;
pub const XKB_KEY_apostrophe: u32 = 0x0027;
pub const XKB_KEY_grave: u32 = 0x0060;
pub const XKB_KEY_backslash: u32 = 0x005c;
pub const XKB_KEY_comma: u32 = 0x002c;
pub const XKB_KEY_equal: u32 = 0x003d;
pub const XKB_KEY_bracketleft: u32 = 0x005b;
pub const XKB_KEY_bracketright: u32 = 0x005d;
pub const XKB_KEY_minus: u32 = 0x002d;
pub const XKB_KEY_period: u32 = 0x002e;
pub const XKB_KEY_semicolon: u32 = 0x003b;
pub const XKB_KEY_slash: u32 = 0x002f;
pub const XKB_KEY_space: u32 = 0x0020;
pub const XKB_KEY_F1: u32 = 0xffbe;
pub const XKB_KEY_F2: u32 = 0xffbf;
pub const XKB_KEY_F3: u32 = 0xffc0;
pub const XKB_KEY_F4: u32 = 0xffc1;
pub const XKB_KEY_F5: u32 = 0xffc2;
pub const XKB_KEY_F6: u32 = 0xffc3;
pub const XKB_KEY_F7: u32 = 0xffc4;
pub const XKB_KEY_F8: u32 = 0xffc5;
pub const XKB_KEY_F9: u32 = 0xffc6;
pub const XKB_KEY_F10: u32 = 0xffc7;
pub const XKB_KEY_F11: u32 = 0xffc8;
pub const XKB_KEY_F12: u32 = 0xffc9;
pub const XKB_KEY_Down: u32 = 0xff54;
pub const XKB_KEY_Left: u32 = 0xff51;
pub const XKB_KEY_Right: u32 = 0xff53;
pub const XKB_KEY_Up: u32 = 0xff52;
pub const XKB_KEY_Escape: u32 = 0xff1b;
pub const XKB_KEY_BackSpace: u32 = 0xff08;
pub const XKB_KEY_Delete: u32 = 0xffff;
pub const XKB_KEY_End: u32 = 0xff57;
pub const XKB_KEY_Return: u32 = 0xff0d;
pub const XKB_KEY_Home: u32 = 0xff50;
pub const XKB_KEY_Insert: u32 = 0xff63;
pub const XKB_KEY_Menu: u32 = 0xff67;
pub const XKB_KEY_Page_Down: u32 = 0xff56;
pub const XKB_KEY_Page_Up: u32 = 0xff55;
pub const XKB_KEY_Pause: u32 = 0xff13;
pub const XKB_KEY_Tab: u32 = 0xff09;
pub const XKB_KEY_Num_Lock: u32 = 0xff7f;
pub const XKB_KEY_Caps_Lock: u32 = 0xffe5;
pub const XKB_KEY_Scroll_Lock: u32 = 0xff14;
pub const XKB_KEY_Shift_L: u32 = 0xffe1;
pub const XKB_KEY_Shift_R: u32 = 0xffe2;
pub const XKB_KEY_Alt_L: u32 = 0xffe9;
pub const XKB_KEY_Alt_R: u32 = 0xffea;
pub const XKB_KEY_Control_L: u32 = 0xffe3;
pub const XKB_KEY_Control_R: u32 = 0xffe4;
pub const XKB_KEY_Super_L: u32 = 0xffeb;
pub const XKB_KEY_Super_R: u32 = 0xffec;
pub const XKB_KEY_KP_Insert: u32 = 0xff9e;
pub const XKB_KEY_KP_End: u32 = 0xff9c;
pub const XKB_KEY_KP_Down: u32 = 0xff99;
pub const XKB_KEY_KP_Next: u32 = 0xff9b;
pub const XKB_KEY_KP_Left: u32 = 0xff96;
pub const XKB_KEY_KP_Begin: u32 = 0xff9d;
pub const XKB_KEY_KP_Right: u32 = 0xff98;
pub const XKB_KEY_KP_Home: u32 = 0xff95;
pub const XKB_KEY_KP_Up: u32 = 0xff97;
pub const XKB_KEY_KP_Prior: u32 = 0xff9a;
pub const XKB_KEY_KP_Delete: u32 = 0xff9f;
pub const XKB_KEY_KP_Decimal: u32 = 0xffae;
pub const XKB_KEY_KP_Divide: u32 = 0xffaf;
pub const XKB_KEY_KP_Multiply: u32 = 0xffaa;
pub const XKB_KEY_KP_Subtract: u32 = 0xffad;
pub const XKB_KEY_KP_Add: u32 = 0xffab;
pub const XKB_KEY_KP_Enter: u32 = 0xff8d;
pub const XKB_KEY_KP_0: u32 = 0xffb0;
pub const XKB_KEY_KP_1: u32 = 0xffb1;
pub const XKB_KEY_KP_2: u32 = 0xffb2;
pub const XKB_KEY_KP_3: u32 = 0xffb3;
pub const XKB_KEY_KP_4: u32 = 0xffb4;
pub const XKB_KEY_KP_5: u32 = 0xffb5;
pub const XKB_KEY_KP_6: u32 = 0xffb6;
pub const XKB_KEY_KP_7: u32 = 0xffb7;
pub const XKB_KEY_KP_8: u32 = 0xffb8;
pub const XKB_KEY_KP_9: u32 = 0xffb9;

use crate::Key;

pub fn keysym_to_key(sym: u32) -> Option<Key> {
    let key = match sym {
        XKB_KEY_0 => Key::Key0,
        XKB_KEY_1 => Key::Key1,
        XKB_KEY_2 => Key::Key2,
        XKB_KEY_3 => Key::Key3,
        XKB_KEY_4 => Key::Key4,
        XKB_KEY_5 => Key::Key5,
        XKB_KEY_6 => Key::Key6,
        XKB_KEY_7 => Key::Key7,
        XKB_KEY_8 => Key::Key8,
        XKB_KEY_9 => Key::Key9,

        XKB_KEY_a => Key::A,
        XKB_KEY_b => Key::B,
        XKB_KEY_c => Key::C,
        XKB_KEY_d => Key::D,
        XKB_KEY_e => Key::E,
        XKB_KEY_f => Key::F,
        XKB_KEY_g => Key::G,
        XKB_KEY_h => Key::H,
        XKB_KEY_i => Key::I,
        XKB_KEY_j => Key::J,
        XKB_KEY_k => Key::K,
        XKB_KEY_l => Key::L,
        XKB_KEY_m => Key::M,
        XKB_KEY_n => Key::N,
        XKB_KEY_o => Key::O,
        XKB_KEY_p => Key::P,
        XKB_KEY_q => Key::Q,
        XKB_KEY_r => Key::R,
        XKB_KEY_s => Key::S,
        XKB_KEY_t => Key::T,
        XKB_KEY_u => Key::U,
        XKB_KEY_v => Key::V,
        XKB_KEY_w => Key::W,
        XKB_KEY_x => Key::X,
        XKB_KEY_y => Key::Y,
        XKB_KEY_z => Key::Z,

        XKB_KEY_apostrophe => Key::Apostrophe,
        XKB_KEY_grave => Key::Backquote,
        XKB_KEY_backslash => Key::Backslash,
        XKB_KEY_comma => Key::Comma,
        XKB_KEY_equal => Key::Equal,
        XKB_KEY_bracketleft => Key::LeftBracket,
        XKB_KEY_bracketright => Key::RightBracket,
        XKB_KEY_minus => Key::Minus,
        XKB_KEY_period => Key::Period,
        XKB_KEY_semicolon => Key::Semicolon,
        XKB_KEY_slash => Key::Slash,
        XKB_KEY_space => Key::Space,

        XKB_KEY_F1 => Key::F1,
        XKB_KEY_F2 => Key::F2,
        XKB_KEY_F3 => Key::F3,
        XKB_KEY_F4 => Key::F4,
        XKB_KEY_F5 => Key::F5,
        XKB_KEY_F6 => Key::F6,
        XKB_KEY_F7 => Key::F7,
        XKB_KEY_F8 => Key::F8,
        XKB_KEY_F9 => Key::F9,
        XKB_KEY_F10 => Key::F10,
        XKB_KEY_F11 => Key::F11,
        XKB_KEY_F12 => Key::F12,

        XKB_KEY_Down => Key::Down,
        XKB_KEY_Left => Key::Left,
        XKB_KEY_Right => Key::Right,
        XKB_KEY_Up => Key::Up,
        XKB_KEY_Escape => Key::Escape,
        XKB_KEY_BackSpace => Key::Backspace,
        XKB_KEY_Delete => Key::Delete,
        XKB_KEY_End => Key::End,
        XKB_KEY_Return => Key::Enter,
        XKB_KEY_Home => Key::Home,
        XKB_KEY_Insert => Key::Insert,
        XKB_KEY_Menu => Key::Menu,
        XKB_KEY_Page_Down => Key::PageDown,
        XKB_KEY_Page_Up => Key::PageUp,
        XKB_KEY_Pause => Key::Pause,
        XKB_KEY_Tab => Key::Tab,
        XKB_KEY_Num_Lock => Key::NumLock,
        XKB_KEY_Caps_Lock => Key::CapsLock,
        XKB_KEY_Scroll_Lock => Key::ScrollLock,
        XKB_KEY_Shift_L => Key::LeftShift,
        XKB_KEY_Shift_R => Key::RightShift,
        XKB_KEY_Alt_L => Key::LeftAlt,
        XKB_KEY_Alt_R => Key::RightAlt,
        XKB_KEY_Control_L => Key::LeftCtrl,
        XKB_KEY_Control_R => Key::RightCtrl,
        XKB_KEY_Super_L => Key::LeftSuper,
        XKB_KEY_Super_R => Key::RightSuper,

        XKB_KEY_KP_0 | XKB_KEY_KP_Insert => Key::NumPad0,
        XKB_KEY_KP_1 | XKB_KEY_KP_End => Key::NumPad1,
        XKB_KEY_KP_2 | XKB_KEY_KP_Down => Key::NumPad2,
        XKB_KEY_KP_3 | XKB_KEY_KP_Next => Key::NumPad3,
        XKB_KEY_KP_4 | XKB_KEY_KP_Left => Key::NumPad4,
        XKB_KEY_KP_5 | XKB_KEY_KP_Begin => Key::NumPad5,
        XKB_KEY_KP_6 | XKB_KEY_KP_Right => Key::NumPad6,
        XKB_KEY_KP_7 | XKB_KEY_KP_Home => Key::NumPad7,
        XKB_KEY_KP_8 | XKB_KEY_KP_Up => Key::NumPad8,
        XKB_KEY_KP_9 | XKB_KEY_KP_Prior => Key::NumPad9,
        XKB_KEY_KP_Decimal | XKB_KEY_KP_Delete => Key::NumPadDot,
        XKB_KEY_KP_Divide => Key::NumPadSlash,
        XKB_KEY_KP_Multiply => Key::NumPadAsterisk,
        XKB_KEY_KP_Subtract => Key::NumPadMinus,
        XKB_KEY_KP_Add => Key::NumPadPlus,
        XKB_KEY_KP_Enter => Key::NumPadEnter,

        _ => return None,
    };
    Some(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numpad_maps_with_numlock_on() {
        assert_eq!(keysym_to_key(XKB_KEY_KP_0), Some(Key::NumPad0));
        assert_eq!(keysym_to_key(XKB_KEY_KP_1), Some(Key::NumPad1));
        assert_eq!(keysym_to_key(XKB_KEY_KP_9), Some(Key::NumPad9));
        assert_eq!(keysym_to_key(XKB_KEY_KP_Decimal), Some(Key::NumPadDot));
    }

    #[test]
    fn numpad_maps_with_numlock_off() {
        assert_eq!(keysym_to_key(XKB_KEY_KP_Insert), Some(Key::NumPad0));
        assert_eq!(keysym_to_key(XKB_KEY_KP_End), Some(Key::NumPad1));
        assert_eq!(keysym_to_key(XKB_KEY_KP_Down), Some(Key::NumPad2));
        assert_eq!(keysym_to_key(XKB_KEY_KP_Next), Some(Key::NumPad3));
        assert_eq!(keysym_to_key(XKB_KEY_KP_Left), Some(Key::NumPad4));
        assert_eq!(keysym_to_key(XKB_KEY_KP_Begin), Some(Key::NumPad5));
        assert_eq!(keysym_to_key(XKB_KEY_KP_Right), Some(Key::NumPad6));
        assert_eq!(keysym_to_key(XKB_KEY_KP_Home), Some(Key::NumPad7));
        assert_eq!(keysym_to_key(XKB_KEY_KP_Up), Some(Key::NumPad8));
        assert_eq!(keysym_to_key(XKB_KEY_KP_Prior), Some(Key::NumPad9));
        assert_eq!(keysym_to_key(XKB_KEY_KP_Delete), Some(Key::NumPadDot));
    }

    #[test]
    fn letters_and_unmapped() {
        assert_eq!(keysym_to_key(XKB_KEY_a), Some(Key::A));
        assert_eq!(keysym_to_key(XKB_KEY_Return), Some(Key::Enter));
        assert_eq!(keysym_to_key(0xdead_beef), None);
    }
}
