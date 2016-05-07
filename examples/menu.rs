extern crate minifb;

use minifb::{Window, Key, Scale, WindowOptions, Menu, MenuItem};
//use minifb::{MENU_KEY_CTRL, MENU_KEY_COMMAND};

const WIDTH: usize = 640;
const HEIGHT: usize = 360;

/*
const MENU_TEST_ID: usize = 1;
const OTHER_MENU_ID: usize = 2;
const COLOR_0_ID: usize = 3;
const COLOR_1_ID: usize = 4;
const COLOR_2_ID: usize = 5;
const CLOSE_MENU_ID: usize = 6;
*/

fn main() {
    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];

    let mut window = Window::new("Noise Test - Press ESC to exit",
                                 WIDTH,
                                 HEIGHT,
                                 WindowOptions {
                                     resize: true,
                                     scale: Scale::X2,
                                     ..WindowOptions::default()
                                 })
                         .expect("Unable to Open Window");

    // Setup a sub menu

    /*
    let sub_menu = vec![
        Menu {
            name: "Color 0",
            key: Key::F1,
            id: COLOR_0_ID,
            ..Menu::default()
        },
        Menu {
            name: "Color 1",
            key: Key::F2,
            id: COLOR_1_ID,
            ..Menu::default()
        },
        Menu {
            name: "Color 2",
            key: Key::F12,
            id: COLOR_2_ID,
            ..Menu::default()
        },
    ];

    // Main menu

    let menu = vec![
        Menu {
            name: "Menu Test",
            key: Key::W,
            id: MENU_TEST_ID,
            modifier: MENU_KEY_CTRL,
            mac_mod: MENU_KEY_COMMAND,
            ..Menu::default()
        },
        Menu::separotor(),
        Menu {
            name: "Other menu!",
            key: Key::S,
            modifier: MENU_KEY_CTRL,
            mac_mod: MENU_KEY_CTRL,
            id: OTHER_MENU_ID,
            ..Menu::default()
        },
        Menu {
            name: "Remove Menu",
            key: Key::R,
            id: CLOSE_MENU_ID,
            ..Menu::default()
        },
        Menu {
            name: "Select Color",
            sub_menu: Some(&sub_menu),
            ..Menu::default()
        }
    ];
    */

    //window.add_menu("Test", &menu).expect("Unable to add menu");

    let mut menu = Menu::new("TestMenu").unwrap();
    let mut item = MenuItem::new("Item", 1).enabled(true);

    menu.add_item(&mut item);
    let _ = window.add_menu(&menu);

    let color_mul = 1;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                buffer[(y * WIDTH) + x] = (((x ^ y) & 0xff) * color_mul) as u32;
            }
        }

        /*
        window.is_menu_pressed().map(|menu_id| {
            match menu_id {
                COLOR_0_ID => {
                    color_mul = 0xfe0000;
                }
                COLOR_1_ID => {
                    color_mul = 0xff00;
                }
                COLOR_2_ID => {
                    color_mul = 1;
                }
                CLOSE_MENU_ID => {
                    println!("remove menu");
                    //window.remove_menu("Test").expect("Unable to remove menu");
                }
                _ => (),
            }

            println!("Menu id {} pressed", menu_id);
        });
        */

        window.get_keys().map(|keys| {
            for t in keys {
                match t {
                    Key::W => println!("holding w!"),
                    Key::T => println!("holding t!"),
                    _ => (),
                }
            }
        });

        window.update_with_buffer(&buffer);
    }
}
